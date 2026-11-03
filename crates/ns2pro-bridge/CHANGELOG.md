# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/genbenbuxing/ns2proconbluetooth/compare/v0.5.0...v0.5.1) - 2026-11-04

### Added

- Desktop app: when a controller is plugged in by USB cable, the Bluetooth bridge pauses and its virtual controller is removed, so Windows does not show two controllers; Bluetooth resumes automatically when the cable is unplugged.
- Desktop app: the log is also written to `%LOCALAPPDATA%\ns2pro-bridge\gui.log` (previous run in `gui.log.old`), with an "Open log folder" button on the Log tab.

### Fixed

- A dropped or unreachable Bluetooth/GATT link no longer forgets the remembered controller, so a brief drop no longer forces pairing again. The controller is forgotten only when it answers commands or pairing incorrectly.

### Known issues (under investigation)

- After the bridge pauses for a plugged-in cable, Steam may not use the wired controller (it reports it as "If_Hid" and does not initialize it). Bluetooth works normally. Until this is fixed, keep using Bluetooth.

## [0.5.0](https://github.com/genbenbuxing/ns2proconbluetooth/compare/v0.4.0...v0.5.0) - 2026-11-03

First release of this fork of [Cookiekira/ns2pro-bridge](https://github.com/Cookiekira/ns2pro-bridge), with Windows support verified on real hardware.

### Added

- Windows desktop app `ns2pro-bridge-gui.exe`: connect / disconnect / reconnect / forget controller, Bluetooth and usbip-win2 driver checks, tray icon, start with Windows, Chinese and English UI.
- `app::run_until` and `app::check_bluetooth` for embedding the bridge; `usbip_win::driver_available`.

### Fixed

- Windows: accept usbip-win2's `number_of_packets = -1` on non-isochronous USB/IP submits; the virtual controller previously failed to enumerate.
- Windows: cancel usbip-win2 re-attach attempts before attaching and after detaching (`STOP_ATTACH_ATTEMPTS`), and treat an already-released port as a clean detach.
- Windows: shut down cleanly when the console window is closed, on logoff and on system shutdown.
- A Bluetooth timeout (for example a sleeping controller) no longer forgets the cached controller.

## [0.4.0](https://github.com/Cookiekira/ns2pro-bridge/compare/v0.3.1...v0.4.0) - 2026-10-27

### Fixed

- resolve the cache default fallibly; document lock recovery limits; test write_command
- keep battery clamp and low-32-bit USB/IP lengths; add boundary tests

### Other

- widen three contracts the re-review found too narrow
- correct contracts the review found overstated
- *(bridge)* document the bridge library; deny rustdoc warnings in CI
- *(proto)* document the public protocol API; make rt private
- drop three remaining redundant checks
- assert private invariants instead of inventing failure outcomes
- state internal invariants instead of unreachable fallbacks
- state exactly what poison recovery does and does not do
- *(bridge)* remove panic paths and casts; poison-tolerant locking
- *(proto)* parse without indexing, casts or panics
- satisfy strict lints in integration tests
- satisfy strict lints in USB attachment; encode manual mode in the type
- satisfy strict lints in Windows BLE backend
- document usbip-win2 unsafe boundaries; satisfy strict lints
- document Linux and runtime unsafe boundaries; satisfy strict lints
- satisfy strict lints in bridge core modules
- satisfy strict lints in ns2pro-proto and ns2pro-usb
- apply machine-applicable strict lint fixes
- use named module files instead of mod.rs

## [0.3.1](https://github.com/Cookiekira/ns2pro-bridge/releases/tag/v0.3.1) - 2026-10-24

### Added

- Install with `cargo binstall ns2pro-bridge` or the shell/PowerShell installers; releases ship checksums.
- Linux: running as a normal user prompts for the password (`sudo`, or `pkexec` without a terminal).
- Linux: `vhci-hcd` is loaded automatically when auto-attach needs it.
- Linux: SIGTERM and SIGHUP detach the virtual controller cleanly, like Ctrl+C.

### Changed

- Renamed to `ns2pro-bridge` (executable, repository and release assets). Release assets are now `ns2pro-bridge-<target>.tar.xz`/`.zip`.
- The saved controller moved to `/var/lib/ns2pro-bridge/` on Linux and `<local app data>/ns2pro-bridge/` on Windows. Pair once again after upgrading.
- Linux release binaries need glibc ≥ 2.39.
