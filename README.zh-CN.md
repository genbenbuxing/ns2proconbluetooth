# ns2proconbluetooth — Switch 2 Pro 手柄蓝牙连接 PC（Windows / Linux）

[English](README.md) | **简体中文**

让 **Nintendo Switch 2 Pro 手柄通过蓝牙 BLE 无线连接电脑**。程序把无线手柄模拟成一个有线 USB Switch 2 Pro 手柄（USB `057e:2069`），Steam 和 SDL 会把它当作普通的任天堂有线手柄识别。

## 来源声明

本项目基于 **[Cookiekira/ns2pro-bridge](https://github.com/Cookiekira/ns2pro-bridge)**（v0.4.0，GPL-3.0-or-later）修改而来，完整保留了上游的提交历史和作者署名。感谢原作者 Harvey（Cookiekira）及其参考的 [VIIPER](https://github.com/Alia5/VIIPER)、[switch2_controller_research](https://github.com/ndeadly/switch2_controller_research)、[usbip-win2](https://github.com/vadimgrn/usbip-win2)。

上游只在 Linux 上实机测试过，Windows 仅通过 CI 编译。我们在它的基础上**优化了程序，并对 Windows 支持做了验证与修复**：逐项对照 usbip-win2 驱动源码（v0.9.6.1–v0.9.8.1）核对了 Windows 端的驱动接口、USB/IP 协议和蓝牙链路，修复了导致 Windows 下无法工作的问题，并在 Windows 11 上从源码编译、通过了全部测试。

### Windows 支持状态

| 项目 | 状态 |
| --- | --- |
| Windows 11 x64 从源码编译 + 全部单元/集成测试 | ✅ 通过 |
| usbip-win2 驱动接口（IOCTL、结构体布局）对照驱动源码核对 | ✅ 一致 |
| USB/IP 协议对照驱动源码核对 | ✅ 已修复不兼容处（见下） |
| 安全审计（网络、文件、进程、依赖、发布流程） | ✅ 未发现恶意行为 |
| 真实手柄 + 真实蓝牙适配器 + usbip-win2 实机测试 | ✅ 通过（2026-11-03）：配对、挂载、输入均正常 |
| Windows 下蓝牙连接间隔（输入延迟） | ✅ 已实测：20 ms，见[延迟](#延迟) |

**测试硬件：** Windows 11 Pro x64（build 26300），技嘉 B850M FORCE WIFI6E 板载 MediaTek MT7902 Wi-Fi 6E / 蓝牙（驱动 1.1146.0.585），usbip-win2 v0.9.8.1，Nintendo Switch 2 Pro 手柄。

## 相对上游的修改

1. **修复：Windows 下虚拟手柄无法枚举（致命）。** usbip-win2 对非等时传输发送 `number_of_packets = -1`（见驱动 `usbip/proto.h` 的 `number_of_packets_non_isoch`），上游只接受 Linux vhci 发送的 `0`，导致 Windows 发来的第一个 USB 请求（读取设备描述符）就被拒绝、连接断开。现在两种取值都接受。（`crates/ns2pro-usb/src/usbip.rs`）
2. **修复：退出后驱动反复重连。** usbip-win2 v0.9.7.7 起会在连接断开后自动重试导入，最长约 2 小时；上游退出时先关闭连接再拔出设备，触发了这一行为，重新启动程序时可能出现两个手柄。现在启动前和退出后都会调用 `STOP_ATTACH_ATTEMPTS` 取消驱动的重连，并把"设备已被驱动自行拔出"视为正常退出。（`crates/ns2pro-usb/src/usbip_win.rs`、`crates/ns2pro-bridge/src/usb/usbip_win.rs`）
3. **修复：手柄休眠后被"忘记"。** 上游在扫描超时（例如手柄休眠超过 30 秒）时也会清除已记住的手柄，下次连接会重新配对、且不再限定地址。现在蓝牙超时不再清除缓存，只有真正的连接错误才会。（`crates/ns2pro-bridge/src/cache.rs`）
4. 移除只适用于上游仓库的 release-plz 发布流程，仓库地址改为本仓库。

## 依赖（Windows）

本程序**需要另外安装一个驱动**：**[usbip-win2](https://github.com/vadimgrn/usbip-win2)**（GPL-3.0，作者 vadimgrn），支持 v0.9.6.1–v0.9.8.1。

原因：Windows 只认"插在 USB 总线上的设备"，普通程序无法凭空创建 USB 设备。usbip-win2 是一个已签名的内核驱动，会在系统里添加一个虚拟 USB 主控制器（设备管理器中显示为 "USBip 3.X Emulated Host Controller"）。本程序通过蓝牙连接手柄，再经 USB/IP 协议把一个虚拟的有线 Switch 2 Pro 手柄"插"进这个虚拟 USB 口，Windows 和 Steam 就会看到一个有线手柄。本项目不附带、也不修改 usbip-win2。

此外还需要一个支持蓝牙 LE、驱动正常的蓝牙适配器。

## 图形界面（Windows）

`ns2pro-bridge-gui.exe` 是给不熟悉命令行的用户准备的窗口程序，内部运行的是同一个桥接程序，另外提供：

- **连接 / 断开 / 重新连接**按钮，实时显示状态（正在寻找手柄、已连接、正在重连），以及蓝牙连接间隔和输入频率。
- **忘记手柄**：用于更换手柄，或配对出错后重新配对。
- **环境检测**：检查蓝牙 LE 适配器和 usbip-win2 驱动；缺少驱动时给出 usbip-win2 官方下载页链接（程序本身不会下载或安装任何东西）。
- **托盘图标**：可设置为关闭窗口后最小化到右下角托盘继续运行；托盘菜单可以显示窗口、连接、断开、退出。勾选**开机自动启动**后会在登录时最小化到托盘启动。
- **中文 / English** 界面，默认跟随 Windows 系统语言。

退出时（托盘菜单 → 退出，或在关闭托盘模式时直接关窗口）程序总会先断开手柄、拔出虚拟 USB 设备。同一时间只能运行一个实例，也不要和命令行版 `ns2pro-bridge.exe` 同时运行。设置保存在 `%LOCALAPPDATA%\ns2pro-bridge\gui-settings.json`。

## 在 Windows 上使用

1. **安装 usbip-win2 驱动**：从 [usbip-win2 Releases](https://github.com/vadimgrn/usbip-win2/releases) 下载已签名的安装包（如 `USBip-0.9.8.1-x64.exe`）并安装。这是内核驱动，需要管理员权限。
2. **获取程序**：从源码编译（推荐，见下方），或使用本仓库 Releases 中的 `ns2pro-bridge-x86_64-pc-windows-msvc.zip`（如有）。
3. **运行** `ns2pro-bridge-gui.exe` 并点击**连接**（或在终端运行 `ns2pro-bridge.exe`），首次连接时按下手柄顶部的**同步键（SYNC）**进行配对，程序会记住手柄。
4. 看到 `controller initialized; receiving input` 后打开 Steam 即可。下次使用时运行程序并按 **Home** 键唤醒手柄。按 **Ctrl+C** 退出。

> ℹ️ **关于 Windows SmartScreen 提示。** 本程序**没有代码签名**。首次运行从网上下载的版本时，Windows 可能提示“Windows 已保护你的电脑”，点击**更多信息 → 仍要运行**即可。运行前可以把文件的 SHA-256 与发布页公布的值核对（PowerShell 中执行 `Get-FileHash .\ns2pro-bridge-gui.exe`）。本程序不需要管理员权限；只有 usbip-win2 驱动安装包（由其作者签名）会请求管理员权限。自己从源码编译则不会出现这个提示。

> ⚠️ **不要在 Windows 设置里配对这个手柄。** 否则 Windows 会一直占用手柄的蓝牙连接，程序将扫描不到它。如果已经配对过，请先在"设置 → 蓝牙和设备"里删除该设备。

可先运行 `ns2pro-bridge.exe --bt-smoke-test` 检查蓝牙适配器能否正常扫描。

### 延迟

Linux 下程序可以把蓝牙连接间隔设为 7.5 ms；Windows 不允许普通程序强制指定间隔，只能请求"吞吐量优先"，实际值由系统、蓝牙驱动和手柄共同决定。程序每次连接都会在日志中打印实际间隔：

```
Observed BLE connection interval  interval_units_1_25_ms=N
```

N 的单位是 1.25 ms：6 = 7.5 ms（理想），12 = 15 ms（良好），24 及以上 = 30 ms 以上（可能感觉到延迟）。

**上述测试硬件的实测结果：** N = 16（连接间隔 **20 ms**，Windows 接受了吞吐量优先请求），输入上报平均约每 15.7 ms 一次（约 64 Hz），程序自身增加约 0.08 ms（p50）/ 0.15 ms（p99）。大多数游戏没有问题；格斗、音游、竞技射击可能感觉比有线稍慢。实际结果取决于你的蓝牙适配器和驱动。加 `--log-level debug` 每 5 秒还会输出报告频率和延迟 p50/p99。如果延迟不理想，可以改用 USB-C 有线连接。

## 在 Linux 上使用

运行 `ns2pro-bridge`，按提示输入密码（程序会通过 `sudo`/`pkexec` 提权并自动加载 `vhci-hcd`），首次连接按同步键。Linux 二进制要求 glibc ≥ 2.39。32 位 Steam 能看到手柄但没有输入时，请安装 32 位 libusb（如 Arch 上的 `lib32-libusb`）并重启 Steam。

## 从源码编译

需要 Rust（`rust-toolchain.toml` 会自动安装指定版本）；Windows 还需要 Visual Studio Build Tools 的 C++ 工具。

```sh
cargo build --locked --release -p ns2pro-bridge
cargo test --locked --workspace
```

输出：`target/release/ns2pro-bridge`（Windows 下为 `.exe`）。图形界面（仅 Windows，需要 Rust 1.95+）：`cargo build --locked --release -p ns2pro-gui`，输出 `target/release/ns2pro-bridge-gui.exe`。

## 命令行选项

| 选项 | 默认值 | 说明 |
| --- | --- | --- |
| `--forget-device` | 关闭 | 忘记已保存的手柄并重新配对 |
| `--device-address MAC` | 使用缓存 | 连接指定地址的手柄 |
| `--no-auto-attach` | 关闭 | 只提供 USB/IP 服务，不在本机挂载 |
| `--usb-addr HOST:PORT` | `localhost:3241` | USB/IP 服务地址 |
| `--log-level LEVEL` | `info` | `trace`/`debug`/`info`/`warn`/`error`/`off`；`RUST_LOG` 优先 |
| `--pair-host` | 关闭 | 每次运行时重新配对主机一次 |
| `--host-address MAC` | 适配器地址 | 配对时使用的主机地址 |
| `--cache-file PATH` | Windows：`%LOCALAPPDATA%\ns2pro-bridge\controller-cache.json`；Linux：`/var/lib/ns2pro-bridge/controller-cache.json` | 保存手柄地址 |
| `--feature-flags FLAGS` | `0x07` | 按键、摇杆、IMU 功能标志 |
| `--bt-smoke-test` | 关闭 | 只检查蓝牙适配器和扫描 |

> 🔒 USB/IP 服务**没有身份验证**。默认只监听本机回环地址；不要把 `--usb-addr` 设为 `0.0.0.0` 等对外地址，否则局域网内任何人都可以读取手柄输入、控制震动和指示灯。

## 许可证

本项目沿用上游的 **GPL-3.0-or-later**（见 [LICENSE.txt](LICENSE.txt)）。由于代码派生自 GPL 授权的 ns2pro-bridge 和 VIIPER（GPL-3.0，见 [LICENSES/VIIPER-GPL-3.0.txt](LICENSES/VIIPER-GPL-3.0.txt)），修改版必须以相同或兼容的 GPL 许可证发布，不能改为 MIT 等宽松许可证。第三方来源与修改声明见 [THIRD_PARTY.md](THIRD_PARTY.md)。

本项目与任天堂无关。Nintendo Switch 是任天堂的商标。

