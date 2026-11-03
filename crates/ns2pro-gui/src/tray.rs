//! Notification-area icon. Its handlers act directly on the bridge and the native
//! window, because eframe does not repaint while the window is hidden.
use crate::{bridge::Bridge, i18n::Lang, system::Window};
use std::{sync::Arc, time::Duration};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub struct Tray {
    _icon: TrayIcon,
    show: MenuItem,
    connect: MenuItem,
    disconnect: MenuItem,
    quit: MenuItem,
}

impl Tray {
    pub fn new(
        lang: Lang,
        bridge: &Arc<Bridge>,
        window: Window,
        ctx: &eframe::egui::Context,
    ) -> Result<Self, String> {
        let text = lang.text();
        let show = MenuItem::new(text.tray_show, true, None);
        let connect = MenuItem::new(text.connect, true, None);
        let disconnect = MenuItem::new(text.disconnect, true, None);
        let quit = MenuItem::new(text.tray_quit, true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &connect,
            &disconnect,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;
        let (rgba, size) = crate::system::icon_rgba();
        let icon = Icon::from_rgba(rgba, size, size).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip(text.title)
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        let (show_id, connect_id, disconnect_id, quit_id) = (
            show.id().clone(),
            connect.id().clone(),
            disconnect.id().clone(),
            quit.id().clone(),
        );
        let menu_bridge = Arc::clone(bridge);
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == show_id {
                window.show();
                menu_ctx.request_repaint();
            } else if event.id == connect_id {
                menu_bridge.connect();
            } else if event.id == disconnect_id {
                menu_bridge.disconnect();
            } else if event.id == quit_id {
                // Release the controller and the virtual USB device before exiting.
                menu_bridge.disconnect_and_wait(Duration::from_secs(10));
                std::process::exit(0);
            }
        }));
        let click_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window.show();
                click_ctx.request_repaint();
            }
        }));
        Ok(Self {
            _icon: tray,
            show,
            connect,
            disconnect,
            quit,
        })
    }

    /// Keeps menu labels in the window's language. Items stay enabled: this runs
    /// only while the window repaints, and connect/disconnect are no-ops when not
    /// applicable.
    pub fn set_lang(&self, lang: Lang) {
        let text = lang.text();
        self.show.set_text(text.tray_show);
        self.connect.set_text(text.connect);
        self.disconnect.set_text(text.disconnect);
        self.quit.set_text(text.tray_quit);
    }
}
