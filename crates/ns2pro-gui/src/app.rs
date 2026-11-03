//! The main window.
use crate::{
    bridge::{self, Bridge, Phase},
    i18n::{Lang, Text},
    settings::Settings,
    system::{self, Window},
    tray::Tray,
};
use eframe::egui::{self, Color32, RichText};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Status,
    Settings,
    Log,
}

const GREEN: Color32 = Color32::from_rgb(40, 170, 80);
const AMBER: Color32 = Color32::from_rgb(225, 150, 0);
const RED: Color32 = Color32::from_rgb(210, 50, 50);
/// Text style for the one value that needs attention: the connection state.
const EMPHASIS: &str = "emphasis";

/// Three sizes: body text, buttons/tabs, and the emphasized status.
fn install_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Small,
                FontId::new(12.5, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(12.5, FontFamily::Monospace),
            ),
            (
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Heading,
                FontId::new(16.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Name(EMPHASIS.into()),
                FontId::new(24.0, FontFamily::Proportional),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 6.0);
    });
}

/// A titled, rounded card spanning the available width.
fn card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).heading().strong());
            ui.add_space(2.0);
            add_contents(ui);
        });
}

fn key(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).weak());
}

const DRIVER_URL: &str = "https://github.com/vadimgrn/usbip-win2/releases";
const BLUETOOTH_SETTINGS: &str = "ms-settings:bluetooth";

#[derive(Default)]
struct Checks {
    bluetooth: Option<Result<(), String>>,
    driver: Option<bool>,
}

impl Checks {
    fn ready(&self) -> bool {
        matches!(self.bluetooth, Some(Ok(()))) && self.driver == Some(true)
    }
}

pub struct App {
    bridge: Arc<Bridge>,
    settings: Settings,
    autostart: bool,
    tray: Option<Tray>,
    window: Option<Window>,
    checks: Arc<Mutex<Checks>>,
    remembered: Option<String>,
    last_phase: Phase,
    shown_lang: Option<Lang>,
    tab: Tab,
    confirm_forget: bool,
    notice: Option<String>,
    quitting: bool,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        bridge: Arc<Bridge>,
        settings: Settings,
        window: Option<Window>,
    ) -> Self {
        system::install_fonts(&cc.egui_ctx);
        install_style(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        bridge.set_repaint(move || ctx.request_repaint());
        let lang = settings.lang();
        let tray = window.and_then(|window| Tray::new(lang, &bridge, window, &cc.egui_ctx).ok());
        let app = Self {
            bridge,
            autostart: system::autostart_enabled(),
            tray,
            window,
            checks: Arc::new(Mutex::new(Checks::default())),
            remembered: bridge::remembered(),
            last_phase: Phase::Idle,
            shown_lang: None,
            tab: Tab::Status,
            confirm_forget: false,
            notice: None,
            quitting: false,
            settings,
        };
        app.run_checks(&cc.egui_ctx, app.settings.auto_connect);
        app
    }

    fn lang(&self) -> Lang {
        self.settings.lang()
    }

    /// Checks Bluetooth and the driver off the UI thread; optionally connects after.
    fn run_checks(&self, ctx: &egui::Context, connect_when_ready: bool) {
        *self
            .checks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Checks::default();
        let checks = Arc::clone(&self.checks);
        let bridge = Arc::clone(&self.bridge);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = Checks {
                driver: Some(bridge::driver_available()),
                bluetooth: Some(bridge::check_bluetooth()),
            };
            let ready = result.ready();
            *checks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = result;
            if connect_when_ready && ready {
                bridge.connect();
            }
            ctx.request_repaint();
        });
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.quitting && self.settings.close_to_tray && self.tray.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if let Some(window) = self.window {
                    window.hide();
                }
            } else if self.bridge.is_running() {
                // Release the controller and the virtual USB device first.
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.quitting = true;
                self.bridge.disconnect();
            }
        }
        if self.quitting && !self.bridge.is_running() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn status_ui(&mut self, ui: &mut egui::Ui, t: &Text) {
        let status = self.bridge.status();
        let (color, label) = match status.phase {
            Phase::Idle => (Color32::GRAY, t.idle),
            Phase::Searching if self.remembered.is_some() => (AMBER, t.waiting_wake),
            Phase::Searching => (AMBER, t.waiting_pair),
            Phase::Connected => (GREEN, t.connected),
            Phase::Reconnecting => (AMBER, t.reconnecting),
            Phase::Stopping => (Color32::GRAY, t.stopping),
            Phase::Failed => (RED, t.failed),
        };
        let emphasis = egui::TextStyle::Name(EMPHASIS.into());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("●").color(color).text_style(emphasis.clone()));
            // Only the indicator carries the state color; the text stays readable.
            ui.label(RichText::new(label).strong().text_style(emphasis));
        });
        egui::Grid::new("details")
            .num_columns(2)
            .spacing([20.0, 6.0])
            .show(ui, |ui| {
                key(ui, t.remembered);
                ui.label(self.remembered.as_deref().unwrap_or(t.none_remembered));
                ui.end_row();
                key(ui, t.virtual_usb);
                if status.usb_attached {
                    ui.colored_label(GREEN, t.attached);
                } else {
                    ui.label(t.not_attached);
                }
                ui.end_row();
                if let Some(ms) = status.interval_ms {
                    key(ui, t.interval);
                    ui.label(format!("{ms:.2} ms"));
                    ui.end_row();
                }
                if let Some(hz) = status.report_hz {
                    key(ui, t.report_rate);
                    ui.label(format!("{hz:.0} Hz"));
                    ui.end_row();
                }
            });
        if let Some(error) = &status.error {
            ui.colored_label(RED, error);
        }
        if matches!(status.phase, Phase::Searching | Phase::Idle | Phase::Failed) {
            ui.label(
                RichText::new(if self.remembered.is_some() {
                    t.hint_wake
                } else {
                    t.hint_pair
                })
                .small(),
            );
            ui.label(RichText::new(t.hint_no_settings_pair).small().weak());
        }
    }

    fn buttons_ui(&mut self, ui: &mut egui::Ui, t: &Text, ready: bool) {
        let running = self.bridge.is_running();
        let size = egui::vec2(96.0, 34.0);
        ui.horizontal_wrapped(|ui| {
            let connect =
                egui::Button::new(RichText::new(t.connect).color(Color32::WHITE).strong())
                    .fill(Color32::from_rgb(40, 120, 220))
                    .min_size(size);
            if ui.add_enabled(!running && ready, connect).clicked() {
                self.notice = None;
                self.bridge.connect();
            }
            if ui
                .add_enabled(running, egui::Button::new(t.disconnect).min_size(size))
                .clicked()
            {
                self.bridge.disconnect();
            }
            if ui
                .add_enabled(running, egui::Button::new(t.reconnect).min_size(size))
                .clicked()
            {
                self.bridge.reconnect();
            }
            if ui
                .add_enabled(
                    !running && self.remembered.is_some(),
                    egui::Button::new(t.forget).min_size(size),
                )
                .clicked()
            {
                self.confirm_forget = true;
            }
        });
        if self.confirm_forget {
            ui.group(|ui| {
                ui.label(t.forget_confirm);
                ui.horizontal(|ui| {
                    if ui.button(t.forget).clicked() {
                        self.confirm_forget = false;
                        self.notice = Some(match bridge::forget() {
                            Ok(()) => t.forgotten.to_owned(),
                            Err(e) => e.to_string(),
                        });
                        self.remembered = bridge::remembered();
                    }
                    if ui.button(t.cancel).clicked() {
                        self.confirm_forget = false;
                    }
                });
            });
        }
        if let Some(notice) = &self.notice {
            ui.label(notice);
        }
    }

    fn checks_ui(&self, ui: &mut egui::Ui, ctx: &egui::Context, t: &Text) {
        let checks = self
            .checks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let good = |ui: &mut egui::Ui| {
            ui.colored_label(Color32::from_rgb(40, 170, 80), format!("● {}", t.ok))
        };
        let bad = |ui: &mut egui::Ui, text: &str| {
            ui.colored_label(Color32::from_rgb(210, 50, 50), format!("● {text}"))
        };
        egui::Grid::new("checks")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.label(t.bluetooth);
                match &checks.bluetooth {
                    None => ui.label(t.checking),
                    Some(Ok(())) => good(ui),
                    Some(Err(_)) => bad(ui, t.missing),
                };
                ui.end_row();
                ui.label(t.driver);
                match checks.driver {
                    None => ui.label(t.checking),
                    Some(true) => good(ui),
                    Some(false) => bad(ui, t.missing),
                };
                ui.end_row();
            });
        if let Some(Err(error)) = &checks.bluetooth {
            ui.label(t.bt_missing);
            ui.small(error);
            ui.hyperlink_to(t.open_bt_settings, BLUETOOTH_SETTINGS);
        }
        if checks.driver == Some(false) {
            ui.label(t.driver_missing);
            ui.hyperlink_to(t.download_driver, DRIVER_URL);
        }
        let checking = checks.bluetooth.is_none() || checks.driver.is_none();
        drop(checks);
        if ui
            .add_enabled(!checking, egui::Button::new(t.recheck))
            .clicked()
        {
            self.run_checks(ctx, false);
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui, t: &Text) {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(t.language);
            let mut lang = self.lang();
            egui::ComboBox::from_id_salt("lang")
                .selected_text(match lang {
                    Lang::Zh => "简体中文",
                    Lang::En => "English",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut lang, Lang::Zh, "简体中文");
                    ui.selectable_value(&mut lang, Lang::En, "English");
                });
            if lang != self.lang() {
                self.settings.lang = Some(lang);
                changed = true;
            }
        });
        changed |= ui
            .checkbox(&mut self.settings.auto_connect, t.auto_connect)
            .changed();
        if self.tray.is_some() {
            changed |= ui
                .checkbox(&mut self.settings.close_to_tray, t.close_to_tray)
                .changed();
            if ui.checkbox(&mut self.autostart, t.autostart).changed()
                && let Err(e) = system::set_autostart(self.autostart)
            {
                self.notice = Some(e.to_string());
                self.autostart = system::autostart_enabled();
            }
        }
        if changed {
            self.settings.save();
        }
    }

    fn log_ui(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.bridge.status().log {
                    ui.label(RichText::new(line).monospace());
                }
            });
    }
}

impl eframe::App for App {
    /// Runs before every frame and, unlike `ui`, also while the window is hidden.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_close(ctx);
        let phase = self.bridge.status().phase;
        if phase != self.last_phase {
            // A connection may have just paired and saved a new controller.
            self.remembered = bridge::remembered();
            self.last_phase = phase;
        }
        let lang = self.lang();
        if self.shown_lang != Some(lang) {
            self.shown_lang = Some(lang);
            if let Some(tray) = &self.tray {
                tray.set_lang(lang);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(lang.text().title.to_owned()));
        }
        // Keep "running" button states fresh while the bridge thread works.
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let t = self.lang().text();
        let ready = self
            .checks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .ready();
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (Tab::Status, t.status),
                    (Tab::Settings, t.settings),
                    (Tab::Log, t.log),
                ] {
                    let text = RichText::new(label).text_style(egui::TextStyle::Button);
                    ui.selectable_value(&mut self.tab, tab, text);
                }
            });
            ui.add_space(2.0);
        });
        egui::CentralPanel::default_margins().show(ui, |ui| match self.tab {
            Tab::Status => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    card(ui, t.status, |ui| {
                        self.status_ui(ui, t);
                        ui.add_space(4.0);
                        self.buttons_ui(ui, t, ready);
                    });
                    ui.add_space(4.0);
                    card(ui, t.checks, |ui| self.checks_ui(ui, &ctx, t));
                });
            }
            Tab::Settings => {
                card(ui, t.settings, |ui| self.settings_ui(ui, t));
            }
            Tab::Log => self.log_ui(ui),
        });
    }
}
