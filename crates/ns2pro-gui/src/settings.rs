//! GUI preferences, stored next to the controller cache.
use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `None` follows the Windows UI language.
    pub lang: Option<Lang>,
    pub auto_connect: bool,
    /// Keep the controller's player LEDs lit while connected.
    pub keep_leds_lit: bool,
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: None,
            auto_connect: true,
            keep_leds_lit: true,
            close_to_tray: true,
        }
    }
}

fn path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("ns2pro-bridge").join("gui-settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn lang(&self) -> Lang {
        self.lang.unwrap_or_else(Lang::system)
    }
}
