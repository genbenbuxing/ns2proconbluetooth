//! Windows integration: single instance, start with Windows, window visibility,
//! fonts and the app icon.
use windows::{
    Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND},
        System::{
            Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
                RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
            },
            Threading::CreateMutexW,
        },
        UI::WindowsAndMessaging::{
            MB_ICONINFORMATION, MB_OK, MessageBoxW, SW_HIDE, SW_RESTORE, SW_SHOW,
            SetForegroundWindow, ShowWindow,
        },
    },
    core::{HSTRING, PCWSTR, w},
};

/// Stops the raw mouse and keyboard input that winit registers for every window.
///
/// winit asks Windows for raw input while the window is in the foreground, and
/// eframe redraws once per raw mouse movement while the pointer is over the
/// window. With a high polling-rate mouse (1000–8000 Hz) that is thousands of
/// redraws per second, and the cursor stutters system-wide. The app only needs
/// ordinary window messages, which are unaffected.
pub fn disable_raw_input() {
    use windows::Win32::UI::Input::{RAWINPUTDEVICE, RIDEV_REMOVE, RegisterRawInputDevices};
    // HID usage page 1 (generic desktop): usage 2 is mouse, 6 is keyboard.
    let devices = [2, 6].map(|usage| RAWINPUTDEVICE {
        usUsagePage: 1,
        usUsage: usage,
        dwFlags: RIDEV_REMOVE,
        hwndTarget: HWND::default(),
    });
    #[expect(
        clippy::cast_possible_truncation,
        reason = "RAWINPUTDEVICE is 16 bytes"
    )]
    let size = size_of::<RAWINPUTDEVICE>() as u32;
    // SAFETY: `devices` is a valid array; RIDEV_REMOVE requires a null target window.
    if let Err(error) = unsafe { RegisterRawInputDevices(&devices, size) } {
        tracing::warn!(%error, "could not stop raw input");
    }
}

/// Holds the named mutex for the life of the process.
pub struct SingleInstance(#[allow(dead_code)] HANDLE);

impl SingleInstance {
    /// `None` when another instance already holds the mutex.
    pub fn acquire() -> Option<Self> {
        // SAFETY: the name is a static NUL-terminated wide string; default security.
        let handle = unsafe { CreateMutexW(None, true, w!("Local\\NS2ProBridgeGui")) }.ok()?;
        // SAFETY: reads the calling thread's last-error value set by CreateMutexW.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return None;
        }
        Some(Self(handle))
    }
}

pub fn message_box(title: &str, text: &str) {
    // SAFETY: both strings are NUL-terminated HSTRINGs alive for the call; no owner window.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(title),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("NS2ProBridge");

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the key was opened by RegOpenKeyExW and is closed once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

fn open_run_key(access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<Key> {
    let mut key = HKEY::default();
    // SAFETY: RUN_KEY is a static wide string and key is writable storage.
    unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, access, &raw mut key) }
        .ok()
        .ok()?;
    Some(Key(key))
}

/// Whether the current-user Run entry exists.
pub fn autostart_enabled() -> bool {
    let Some(key) = open_run_key(KEY_QUERY_VALUE) else {
        return false;
    };
    // SAFETY: a size-only query on an open key; no data buffer is passed.
    unsafe { RegQueryValueExW(key.0, RUN_VALUE, None, None, None, None) }.is_ok()
}

/// Adds or removes the current-user Run entry that starts this app minimized.
pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    let key =
        open_run_key(KEY_SET_VALUE).ok_or_else(|| std::io::Error::other("cannot open Run key"))?;
    if !enabled {
        // SAFETY: deletes one named value of an open key.
        let result = unsafe { RegDeleteValueW(key.0, RUN_VALUE) };
        return if result.is_ok() || result == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(result.0.cast_signed()))
        };
    }
    let exe = std::env::current_exe()?;
    let command = format!("\"{}\" --minimized", exe.display());
    let wide: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
    let bytes: Vec<u8> = wide.iter().flat_map(|unit| unit.to_le_bytes()).collect();
    // SAFETY: bytes is a REG_SZ payload including its NUL terminator, alive for the call.
    let result = unsafe { RegSetValueExW(key.0, RUN_VALUE, None, REG_SZ, Some(&bytes)) };
    if result.is_ok() {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(result.0.cast_signed()))
    }
}

/// The native window, for showing and hiding it while eframe is not repainting.
#[derive(Clone, Copy)]
pub struct Window(pub isize);

impl Window {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }
    pub fn show(self) {
        // SAFETY: the handle belongs to this process's main window; both calls only
        // change visibility and focus.
        unsafe {
            let _ = ShowWindow(self.hwnd(), SW_SHOW);
            let _ = ShowWindow(self.hwnd(), SW_RESTORE);
            let _ = SetForegroundWindow(self.hwnd());
        }
    }
    pub fn hide(self) {
        // SAFETY: as above.
        unsafe {
            let _ = ShowWindow(self.hwnd(), SW_HIDE);
        }
    }
}

/// Adds a system CJK font as a fallback so Chinese text renders.
pub fn install_fonts(ctx: &eframe::egui::Context) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = ["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc"];
    let Some(bytes) = candidates
        .iter()
        .find_map(|name| std::fs::read(format!("{windir}\\Fonts\\{name}")).ok())
    else {
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".into(),
        std::sync::Arc::new(FontData::from_owned(bytes)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".into());
    }
    ctx.set_fonts(fonts);
}

/// The app icon as 128×128 RGBA, from `assets/icon-128.rgba`.
pub fn icon_rgba() -> (Vec<u8>, u32) {
    (include_bytes!("../assets/icon-128.rgba").to_vec(), 128)
}
