# ns2proconbluetooth — Switch 2 Pro Controller on PC over Bluetooth (Windows / Linux)

**English** | [简体中文](README.zh-CN.md)

Connect a **Nintendo Switch 2 Pro Controller to a PC over Bluetooth LE**. The program presents the wireless controller as a wired USB Switch 2 Pro Controller (USB `057e:2069`), so Steam and SDL recognize it as a regular wired Nintendo controller.

## Attribution

This project is a modified version of **[Cookiekira/ns2pro-bridge](https://github.com/Cookiekira/ns2pro-bridge)** (v0.4.0, GPL-3.0-or-later). The full upstream commit history and authorship are preserved. Thanks to the original author Harvey (Cookiekira) and to the projects it builds on: [VIIPER](https://github.com/Alia5/VIIPER), [switch2_controller_research](https://github.com/ndeadly/switch2_controller_research) and [usbip-win2](https://github.com/vadimgrn/usbip-win2).

Upstream was only tested on real hardware under Linux; Windows was only built in CI. Building on it, we **optimized the program and verified and fixed Windows support**: the Windows driver interface, USB/IP protocol and Bluetooth path were checked item by item against the usbip-win2 driver source (v0.9.6.1–v0.9.8.1), the issues that prevented it from working on Windows were fixed, and it was built from source and passes the full test suite on Windows 11.

### Windows support status

| Item | Status |
| --- | --- |
| Build from source on Windows 11 x64 + all unit/integration tests | ✅ Passing |
| usbip-win2 driver interface (IOCTLs, struct layouts) checked against driver source | ✅ Matches |
| USB/IP protocol checked against driver source | ✅ Incompatibilities fixed (see below) |
| Security audit (network, files, processes, dependencies, release pipeline) | ✅ No malicious behavior found |
| Real controller + real Bluetooth adapter + usbip-win2 hardware test | ✅ Passing (2026-11-03): pairs, attaches, input received |
| Bluetooth connection interval on Windows (input latency) | ✅ Measured: 20 ms interval, see [Latency](#latency) |

**Tested hardware:** Windows 11 Pro x64 (build 26300), Gigabyte B850M FORCE WIFI6E with its onboard MediaTek MT7902 Wi-Fi 6E / Bluetooth (driver 1.1146.0.585), usbip-win2 v0.9.8.1, Nintendo Switch 2 Pro Controller.

## Changes from upstream

1. **Fix: virtual controller fails to enumerate on Windows (critical).** usbip-win2 sends `number_of_packets = -1` for non-isochronous transfers (see `number_of_packets_non_isoch` in the driver's `usbip/proto.h`). Upstream only accepted the `0` sent by Linux vhci, so the very first USB request from Windows (reading the device descriptor) was rejected and the connection dropped. Both values are now accepted. (`crates/ns2pro-usb/src/usbip.rs`)
2. **Fix: driver keeps re-attaching after exit.** Since v0.9.7.7, usbip-win2 automatically retries an import for up to about 2 hours after its connection drops. Upstream closed the connection before unplugging the device on exit, which triggered this, and restarting the program could produce two controllers. The bridge now calls `STOP_ATTACH_ATTEMPTS` before attaching and after detaching to cancel the driver's retries, and treats "device already unplugged by the driver" as a clean exit. (`crates/ns2pro-usb/src/usbip_win.rs`, `crates/ns2pro-bridge/src/usb/usbip_win.rs`)
3. **Fix: controller forgotten after it sleeps.** Upstream also cleared the remembered controller on a scan timeout (e.g. when the controller slept for more than 30 s), so the next connection re-paired and no longer filtered by address. A Bluetooth timeout no longer clears the cache; only real connection errors do. (`crates/ns2pro-bridge/src/cache.rs`)
4. Removed the release-plz pipeline that only applied to the upstream repository, and pointed the repository URL at this repo.

## Requirements (Windows)

This program **requires a separately installed driver**: **[usbip-win2](https://github.com/vadimgrn/usbip-win2)** (GPL-3.0, by vadimgrn), versions v0.9.6.1–v0.9.8.1.

Why: Windows only recognizes a controller that appears as a device on a USB bus, and a normal program cannot create one. usbip-win2 is a signed kernel driver that adds a virtual USB host controller ("USBip 3.X Emulated Host Controller" in Device Manager). This program connects to the controller over Bluetooth and plugs a virtual wired Switch 2 Pro Controller into that virtual USB port over the USB/IP protocol, so Windows and Steam see a wired controller. This project does not bundle or modify usbip-win2.

You also need a Bluetooth adapter with Bluetooth LE support and a working driver.

## Desktop app (Windows)

`ns2pro-bridge-gui.exe` is a window app for people who do not use the command line. It runs the same bridge and adds:

- **Connect / Disconnect / Reconnect** buttons and a live status (looking for the controller, connected, reconnecting), with the Bluetooth interval and input rate.
- **Forget controller**, to pair a different controller or recover from a failed pairing.
- **System check** for the Bluetooth LE adapter and the usbip-win2 driver, with a link to the official usbip-win2 download page when it is missing (the app does not download or install anything itself).
- **Tray icon**: closing the window keeps it running in the notification area (optional); the tray menu can show the window, connect, disconnect or quit. **Start with Windows** starts it minimized to the tray.
- **Chinese / English** interface, following the Windows language by default.
- **Cable detection**: when the controller is plugged in by USB cable, the Bluetooth bridge pauses and removes its virtual controller, so Windows does not see two controllers; Bluetooth resumes when the cable is unplugged (press Home on the controller if it does not reconnect by itself).
- **Keep controller LEDs lit while connected** (on by default): if a game or Steam asks to turn every player LED off, the controller shows the default pattern instead, so you can always tell it is connected. Other LED patterns, such as player numbers, are shown as requested.
- **Log file**: `%LOCALAPPDATA%\ns2pro-bridge\gui.log` (the previous run is kept as `gui.log.old`); the Log tab has an "Open log folder" button.

Quitting (tray menu → Quit, or closing the window with tray mode off) always releases the controller and unplugs the virtual USB device first. Only one copy of the app runs at a time; do not run it together with the command-line `ns2pro-bridge.exe`. Settings are stored in `%LOCALAPPDATA%\ns2pro-bridge\gui-settings.json`.

### Known issues

- **Bluetooth stutter or drops**: usually weak signal. On motherboards with Wi-Fi/Bluetooth, make sure the Wi-Fi antenna is attached; it is shared with Bluetooth.

## Using it on Windows

1. **Install the usbip-win2 driver**: download the signed installer (e.g. `USBip-0.9.8.1-x64.exe`) from [usbip-win2 Releases](https://github.com/vadimgrn/usbip-win2/releases) and install it. It is a kernel driver and needs administrator rights.
2. **Get the program**: build from source (recommended, see below), or use `ns2pro-bridge-x86_64-pc-windows-msvc.zip` from this repository's Releases (if available).
3. **Run** `ns2pro-bridge-gui.exe` and click **Connect** (or run `ns2pro-bridge.exe` in a terminal). On the first connection, press the **SYNC** button on top of the controller to pair; the program remembers the controller.
4. When `controller initialized; receiving input` appears, open Steam. Next time, run the program and press **Home** to wake the controller. Press **Ctrl+C** to exit.

> ℹ️ **Windows SmartScreen warning.** The executables are **not code-signed**. When you run a downloaded copy for the first time, Windows may show "Windows protected your PC". Click **More info → Run anyway**. Before doing so, you can compare the file's SHA-256 with the value published in the release (`Get-FileHash .\ns2pro-bridge-gui.exe` in PowerShell). The app needs no administrator rights; only the usbip-win2 driver installer (which is signed by its author) asks for them. Building from source avoids the warning entirely.

> ⚠️ **Do not pair the controller in Windows Settings.** Windows would then hold the controller's Bluetooth connection and the program could not find it. If you already paired it, remove it first under Settings → Bluetooth & devices.

You can run `ns2pro-bridge.exe --bt-smoke-test` first to check that your Bluetooth adapter can scan.

### Latency

On Linux the program sets the Bluetooth connection interval to 7.5 ms. Windows does not let a normal program force the interval; it can only request "throughput optimized", and the actual value is decided by Windows, the Bluetooth driver and the controller together. The program logs the actual interval on every connection:

```
Observed BLE connection interval  interval_units_1_25_ms=N
```

N is in units of 1.25 ms: 6 = 7.5 ms (ideal), 12 = 15 ms (good), 24 or more = 30 ms+ (latency may be noticeable).

**Measured on the tested hardware above:** N = 16 (**20 ms** interval; Windows accepted the throughput-optimized request), input reports every ~15.7 ms on average (~64 Hz), and the bridge itself adds ~0.08 ms (p50) / ~0.15 ms (p99). Fine for most games; fighting, rhythm and competitive shooting games may feel slightly slower than wired. Results depend on your Bluetooth adapter and driver. With `--log-level debug` it also prints the report rate and latency p50/p99 every 5 seconds. If latency is not acceptable, use a wired USB-C connection instead.

## Using it on Linux

Run `ns2pro-bridge` and enter your password when asked (the program elevates via `sudo`/`pkexec` and loads `vhci-hcd` automatically), then press SYNC on first connection. Linux binaries require glibc ≥ 2.39. If 32-bit Steam sees the controller but gets no input, install 32-bit libusb (e.g. `lib32-libusb` on Arch) and restart Steam.

## Building from source

Requires Rust (`rust-toolchain.toml` installs the pinned version automatically); on Windows you also need the C++ tools from Visual Studio Build Tools.

```sh
cargo build --locked --release -p ns2pro-bridge
cargo test --locked --workspace
```

Output: `target/release/ns2pro-bridge` (`.exe` on Windows). For the desktop app (Windows, Rust 1.95+): `cargo build --locked --release -p ns2pro-gui` produces `target/release/ns2pro-bridge-gui.exe`.

## Command-line options

| Option | Default | Description |
| --- | --- | --- |
| `--forget-device` | off | Forget the saved controller and pair again |
| `--device-address MAC` | cached | Connect to the controller at this address |
| `--no-auto-attach` | off | Only serve USB/IP; do not attach locally |
| `--usb-addr HOST:PORT` | `localhost:3241` | USB/IP server address |
| `--log-level LEVEL` | `info` | `trace`/`debug`/`info`/`warn`/`error`/`off`; `RUST_LOG` takes precedence |
| `--pair-host` | off | Re-pair the host once per run |
| `--host-address MAC` | adapter address | Host address used for pairing |
| `--keep-leds-lit` | off | Show the default LED pattern instead of turning every player LED off |
| `--cache-file PATH` | Windows: `%LOCALAPPDATA%\ns2pro-bridge\controller-cache.json`; Linux: `/var/lib/ns2pro-bridge/controller-cache.json` | Where the controller address is saved |
| `--feature-flags FLAGS` | `0x07` | Button, stick and IMU feature flags |
| `--bt-smoke-test` | off | Only check the Bluetooth adapter and scanning |

> 🔒 The USB/IP server has **no authentication**. By default it only listens on loopback. Do not set `--usb-addr` to `0.0.0.0` or another external address, or anyone on your LAN could read controller input and drive rumble and LEDs.

## License

This project keeps upstream's **GPL-3.0-or-later** (see [LICENSE.txt](LICENSE.txt)). Because the code derives from GPL-licensed ns2pro-bridge and VIIPER (GPL-3.0, see [LICENSES/VIIPER-GPL-3.0.txt](LICENSES/VIIPER-GPL-3.0.txt)), modified versions must be released under the same or a compatible GPL license and cannot be relicensed under permissive licenses such as MIT. Third-party sources and the modification notice are in [THIRD_PARTY.md](THIRD_PARTY.md).

This project is not affiliated with Nintendo. Nintendo Switch is a trademark of Nintendo.
