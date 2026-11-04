//! Runs the bridge library on a background thread and turns its log events into
//! connection state for the window and the tray.
use ns2pro_bridge::{app, cache::Cache, cli::Options};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Write as _,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use tracing::{Event, Subscriber, field::Field};
use tracing_subscriber::{Layer, layer::Context};

const LOG_LINES: usize = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// Running, before the controller has connected.
    Searching,
    Connected,
    Reconnecting,
    Stopping,
    Failed,
}

#[derive(Debug)]
pub struct Status {
    pub phase: Phase,
    pub address: Option<String>,
    pub interval_ms: Option<f64>,
    pub report_hz: Option<f64>,
    pub usb_attached: bool,
    pub error: Option<String>,
    pub log: VecDeque<String>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            address: None,
            interval_ms: None,
            report_hz: None,
            usb_attached: false,
            error: None,
            log: VecDeque::new(),
        }
    }
}

type Repaint = Arc<dyn Fn() + Send + Sync>;

/// Shared by the window, the tray handlers and the bridge thread.
pub struct Bridge {
    status: Mutex<Status>,
    run: Mutex<Option<Run>>,
    repaint: Mutex<Option<Repaint>>,
    /// A real controller is plugged in by USB cable.
    wired: AtomicBool,
    /// The bridge was stopped because of the cable and resumes when it is unplugged.
    paused_for_wire: AtomicBool,
    /// Passed to the bridge as `--keep-leds-lit` on the next connection.
    keep_leds_lit: AtomicBool,
}

struct Run {
    stop: CancellationToken,
    thread: JoinHandle<()>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Bridge {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            status: Mutex::new(Status::default()),
            run: Mutex::new(None),
            repaint: Mutex::new(None),
            wired: AtomicBool::new(false),
            paused_for_wire: AtomicBool::new(false),
            keep_leds_lit: AtomicBool::new(false),
        })
    }

    pub fn set_repaint(&self, repaint: impl Fn() + Send + Sync + 'static) {
        *lock(&self.repaint) = Some(Arc::new(repaint));
    }

    fn repaint(&self) {
        let repaint = lock(&self.repaint).clone();
        if let Some(repaint) = repaint {
            repaint();
        }
    }

    pub fn status(&self) -> MutexGuard<'_, Status> {
        lock(&self.status)
    }

    pub fn is_running(&self) -> bool {
        lock(&self.run)
            .as_ref()
            .is_some_and(|run| !run.thread.is_finished())
    }

    /// Takes effect from the next connection.
    pub fn set_keep_leds_lit(&self, keep: bool) {
        self.keep_leds_lit.store(keep, Ordering::Relaxed);
    }

    pub fn wired(&self) -> bool {
        self.wired.load(Ordering::Relaxed)
    }

    /// Polls for a controller plugged in by cable. While one is, the bridge stays
    /// stopped so Steam uses the wired controller instead of a silent virtual one.
    pub fn watch_wired(self: &Arc<Self>) {
        self.wired
            .store(crate::wired::controller_plugged_in(), Ordering::Relaxed);
        let bridge = Arc::clone(self);
        std::thread::spawn(move || {
            loop {
                let now = crate::wired::controller_plugged_in();
                let was = bridge.wired.swap(now, Ordering::Relaxed);
                if now && !was {
                    tracing::info!("wired controller detected; pausing the Bluetooth bridge");
                    if bridge.is_running() {
                        bridge.paused_for_wire.store(true, Ordering::Relaxed);
                        bridge.disconnect();
                    }
                    bridge.repaint();
                } else if !now && was {
                    tracing::info!("wired controller unplugged");
                    if bridge.paused_for_wire.swap(false, Ordering::Relaxed) {
                        // Let the previous run finish releasing the device first.
                        bridge.disconnect_and_wait(Duration::from_secs(10));
                        bridge.connect();
                    }
                    bridge.repaint();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });
    }

    /// Starts connecting; does nothing while a previous run is still active or a
    /// controller is plugged in by cable.
    pub fn connect(self: &Arc<Self>) {
        if self.wired() {
            return;
        }
        let mut run = lock(&self.run);
        if run.as_ref().is_some_and(|run| !run.thread.is_finished()) {
            return;
        }
        if let Some(old) = run.take() {
            let _ = old.thread.join();
        }
        {
            let mut status = self.status();
            status.phase = Phase::Searching;
            status.error = None;
            status.address = None;
            status.interval_ms = None;
            status.report_hz = None;
            status.usb_attached = false;
        }
        let stop = CancellationToken::new();
        let bridge = Arc::clone(self);
        let thread_stop = stop.clone();
        let keep_leds_lit = self.keep_leds_lit.load(Ordering::Relaxed);
        let thread = std::thread::spawn(move || {
            let result = run_bridge(thread_stop, keep_leds_lit);
            {
                let mut status = bridge.status();
                status.usb_attached = false;
                status.report_hz = None;
                match result {
                    Ok(()) => status.phase = Phase::Idle,
                    Err(error) => {
                        status.phase = Phase::Failed;
                        status.error = Some(error);
                    }
                }
            }
            bridge.repaint();
        });
        *run = Some(Run { stop, thread });
        drop(run);
        self.repaint();
    }

    /// Asks the running bridge to release the controller and the virtual device.
    pub fn disconnect(&self) {
        if let Some(run) = lock(&self.run).as_ref()
            && !run.thread.is_finished()
        {
            run.stop.cancel();
            self.status().phase = Phase::Stopping;
        }
        self.repaint();
    }

    /// Disconnects, waits for the release to finish, then connects again.
    pub fn reconnect(self: &Arc<Self>) {
        let bridge = Arc::clone(self);
        std::thread::spawn(move || {
            bridge.disconnect_and_wait(Duration::from_secs(10));
            bridge.connect();
        });
    }

    /// Disconnects and blocks until the bridge thread has released everything.
    pub fn disconnect_and_wait(&self, timeout: Duration) {
        self.disconnect();
        let deadline = std::time::Instant::now() + timeout;
        while self.is_running() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn on_event(&self, message: &str, fields: &HashMap<&'static str, String>) {
        let field = |name: &str| fields.get(name).map(String::as_str);
        let mut status = self.status();
        match message {
            "controller initialized; receiving input" => {
                status.phase = Phase::Connected;
                status.address = field("address").map(str::to_owned);
            }
            "controller session ended; reconnecting" => {
                // The virtual USB device stays attached across Bluetooth reconnects.
                if status.phase == Phase::Connected {
                    status.phase = Phase::Reconnecting;
                }
                status.report_hz = None;
            }
            "Attached virtual USB device through usbip-win2" => status.usb_attached = true,
            "Observed BLE connection interval" => {
                if let Some(units) =
                    field("interval_units_1_25_ms").and_then(|v| v.parse::<f64>().ok())
                {
                    status.interval_ms = Some(units * 1.25);
                }
            }
            "bridge statistics (receipt to completed socket write; excludes BLE and host)" => {
                let mean = field("interarrival_mean_us").and_then(|v| v.parse::<f64>().ok());
                status.report_hz = mean.filter(|m| *m > 0.0).map(|m| 1_000_000.0 / m);
            }
            _ => {}
        }
    }

    fn push_log(&self, line: String) {
        let mut status = self.status();
        if status.log.len() >= LOG_LINES {
            status.log.pop_front();
        }
        status.log.push_back(line);
    }
}

fn run_bridge(stop: CancellationToken, keep_leds_lit: bool) -> Result<(), String> {
    let mut options = default_options();
    options.keep_leds_lit = keep_leds_lit;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let local = tokio::task::LocalSet::new();
    local
        .block_on(
            &runtime,
            app::run_until(options, async move { stop.cancelled().await }),
        )
        .map_err(|e| format!("{e}\n{}", e.hint()))
}

fn default_options() -> Options {
    use clap::Parser;
    Options::parse_from(["ns2pro-bridge"])
}

fn cache() -> std::io::Result<Cache> {
    default_options().cache_file().map(Cache::new)
}

/// The remembered controller address, if any.
pub fn remembered() -> Option<String> {
    cache()
        .ok()?
        .load()
        .ok()
        .flatten()
        .map(|address| address.to_string())
}

/// Forgets the remembered controller so the next connection pairs again.
pub fn forget() -> std::io::Result<()> {
    cache()?.forget()
}

/// Whether the usbip-win2 driver is installed.
pub fn driver_available() -> bool {
    ns2pro_bridge::usb::usbip_win::driver_available()
}

/// Whether a Bluetooth LE adapter is usable; the error text otherwise.
pub fn check_bluetooth() -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let local = tokio::task::LocalSet::new();
    local
        .block_on(&runtime, app::check_bluetooth())
        .map_err(|e| e.to_string())
}

/// `%LOCALAPPDATA%\ns2pro-bridge\gui.log`; the previous run's log is kept as `gui.log.old`.
pub fn open_log_file() -> Option<(std::fs::File, std::path::PathBuf)> {
    let dir = dirs::data_local_dir()?.join("ns2pro-bridge");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("gui.log");
    let _ = std::fs::rename(&path, dir.join("gui.log.old"));
    let file = std::fs::File::create(&path).ok()?;
    Some((file, path))
}

/// Feeds bridge log events into [`Bridge`] state and the in-window log.
pub struct StatusLayer(pub Arc<Bridge>);

#[derive(Default)]
struct Fields {
    message: String,
    fields: HashMap<&'static str, String>,
}

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.insert(field.name(), format!("{value:?}"));
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.fields.insert(field.name(), value.to_owned());
        }
    }
}

impl<S: Subscriber> Layer<S> for StatusLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut visited = Fields::default();
        event.record(&mut visited);
        let metadata = event.metadata();
        self.0.on_event(&visited.message, &visited.fields);
        // Periodic statistics only update the numbers, not the visible log.
        if *metadata.level() <= tracing::Level::INFO {
            let mut line = format!("{} {}", metadata.level(), visited.message);
            let mut fields: Vec<_> = visited.fields.iter().collect();
            fields.sort();
            for (name, value) in fields {
                let _ = write!(line, " {name}={value}");
            }
            self.0.push_log(line);
        }
        self.0.repaint();
    }
}
