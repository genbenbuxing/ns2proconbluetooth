//! Detects a Switch 2 Pro Controller plugged in by USB cable.
//!
//! The bridge's virtual controller has the same USB IDs (057E:2069), so the two are
//! told apart by their place in the device tree: the virtual one sits under the
//! usbip-win2 emulated host controller, whose driver service is `usbip2_ude`.
use windows::{
    Win32::Devices::DeviceAndDriverInstallation::{
        CM_DRP_SERVICE, CM_GETIDLIST_FILTER_ENUMERATOR, CM_GETIDLIST_FILTER_PRESENT,
        CM_Get_DevNode_Registry_PropertyW, CM_Get_Device_ID_List_SizeW, CM_Get_Device_ID_ListW,
        CM_Get_Parent, CM_LOCATE_DEVNODE_NORMAL, CM_Locate_DevNodeW, CR_SUCCESS,
    },
    core::{HSTRING, w},
};

const CONTROLLER_PREFIX: &str = "USB\\VID_057E&PID_2069\\";
const USBIP_SERVICE: &str = "usbip2_ude";
/// Controller, hub, host controller: the service is found within a few levels.
const MAX_DEPTH: usize = 6;

/// Instance IDs of present devices enumerated by the USB bus driver.
fn present_usb_devices() -> Vec<String> {
    let flags = CM_GETIDLIST_FILTER_ENUMERATOR | CM_GETIDLIST_FILTER_PRESENT;
    let mut len = 0u32;
    // SAFETY: a size query with a static filter string and writable length.
    if unsafe { CM_Get_Device_ID_List_SizeW(&raw mut len, w!("USB"), flags) } != CR_SUCCESS {
        return Vec::new();
    }
    let mut buffer = vec![0u16; len as usize];
    // SAFETY: buffer holds `len` UTF-16 units as reported by the size query.
    if unsafe { CM_Get_Device_ID_ListW(w!("USB"), &mut buffer, flags) } != CR_SUCCESS {
        return Vec::new();
    }
    // A double-NUL-terminated list of NUL-terminated IDs.
    buffer
        .split(|unit| *unit == 0)
        .filter(|id| !id.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

fn service(devinst: u32) -> Option<String> {
    let mut buffer = [0u16; 128];
    let mut len = u32::try_from(std::mem::size_of_val(&buffer)).ok()?;
    // SAFETY: buffer is writable for `len` bytes and outlives the call.
    let result = unsafe {
        CM_Get_DevNode_Registry_PropertyW(
            devinst,
            CM_DRP_SERVICE,
            None,
            Some(buffer.as_mut_ptr().cast()),
            &raw mut len,
            0,
        )
    };
    if result != CR_SUCCESS {
        return None;
    }
    let text = String::from_utf16_lossy(&buffer);
    Some(text.trim_end_matches('\0').to_owned())
}

fn under_usbip(id: &str) -> bool {
    let mut devinst = 0u32;
    // SAFETY: the ID is a NUL-terminated HSTRING alive for the call.
    if unsafe {
        CM_Locate_DevNodeW(
            &raw mut devinst,
            &HSTRING::from(id),
            CM_LOCATE_DEVNODE_NORMAL,
        )
    } != CR_SUCCESS
    {
        return false;
    }
    for _ in 0..MAX_DEPTH {
        if service(devinst).is_some_and(|s| s.eq_ignore_ascii_case(USBIP_SERVICE)) {
            return true;
        }
        let mut parent = 0u32;
        // SAFETY: devinst is a located device node and parent is writable.
        if unsafe { CM_Get_Parent(&raw mut parent, devinst, 0) } != CR_SUCCESS {
            return false;
        }
        devinst = parent;
    }
    false
}

/// Whether a real Switch 2 Pro Controller is connected by USB cable.
pub fn controller_plugged_in() -> bool {
    present_usb_devices().iter().any(|id| {
        // The composite parent only, not its MI_xx interfaces.
        id.get(..CONTROLLER_PREFIX.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(CONTROLLER_PREFIX))
            && !under_usbip(id)
    })
}
