//! Desktop app for connecting a Switch 2 Pro Controller over Bluetooth without the
//! command line. Windows only; the `ns2pro-bridge` command-line tool covers Linux.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod bridge;
#[cfg(windows)]
mod i18n;
#[cfg(windows)]
mod settings;
#[cfg(windows)]
mod system;
#[cfg(windows)]
mod tray;

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "ns2pro-bridge-gui currently supports Windows only; use the ns2pro-bridge command-line tool."
    );
}

#[cfg(windows)]
fn main() -> eframe::Result {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tracing_subscriber::{EnvFilter, Layer, filter::filter_fn, layer::SubscriberExt};

    let settings = settings::Settings::load();
    let text = settings.lang().text();
    let Some(_instance) = system::SingleInstance::acquire() else {
        system::message_box(text.title, text.already_running);
        return Ok(());
    };
    let bridge = bridge::Bridge::new();
    // Only the bridge library's own events reach the status layer. Installed with
    // set_global_default rather than init(): init() would also forward `log` records
    // from egui/winit, which can be emitted while egui holds its context lock and
    // would deadlock when the layer requests a repaint.
    let status =
        bridge::StatusLayer(std::sync::Arc::clone(&bridge)).with_filter(filter_fn(|metadata| {
            metadata.target().starts_with("ns2pro_bridge")
        }));
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("info,ns2pro_bridge::stats=debug"))
        .with(status);
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Started by Windows at login: stay in the tray.
    let minimized = std::env::args().any(|arg| arg == "--minimized");
    let (rgba, size) = system::icon_rgba();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(text.title)
            .with_inner_size([480.0, 640.0])
            .with_min_inner_size([400.0, 420.0])
            .with_visible(!minimized)
            .with_icon(eframe::egui::IconData {
                rgba,
                width: size,
                height: size,
            }),
        ..Default::default()
    };
    eframe::run_native(
        text.title,
        options,
        Box::new(move |cc| {
            let window = cc
                .window_handle()
                .ok()
                .and_then(|handle| match handle.as_raw() {
                    RawWindowHandle::Win32(handle) => Some(system::Window(handle.hwnd.get())),
                    _ => None,
                });
            Ok(Box::new(app::App::new(cc, bridge, settings, window)))
        }),
    )
}
