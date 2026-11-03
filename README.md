# ns2proconbluetooth — Switch 2 Pro 手柄蓝牙连接 PC（Windows / Linux）

**简体中文** | [English](README.en.md)

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
| 真实手柄 + 真实蓝牙适配器 + usbip-win2 实机测试 | ⏳ 进行中 |
| Windows 下蓝牙连接间隔（输入延迟） | ⏳ 需实机测量，见[延迟](#延迟) |

## 相对上游的修改

1. **修复：Windows 下虚拟手柄无法枚举（致命）。** usbip-win2 对非等时传输发送 `number_of_packets = -1`（见驱动 `usbip/proto.h` 的 `number_of_packets_non_isoch`），上游只接受 Linux vhci 发送的 `0`，导致 Windows 发来的第一个 USB 请求（读取设备描述符）就被拒绝、连接断开。现在两种取值都接受。（`crates/ns2pro-usb/src/usbip.rs`）
2. **修复：退出后驱动反复重连。** usbip-win2 v0.9.7.7 起会在连接断开后自动重试导入，最长约 2 小时；上游退出时先关闭连接再拔出设备，触发了这一行为，重新启动程序时可能出现两个手柄。现在启动前和退出后都会调用 `STOP_ATTACH_ATTEMPTS` 取消驱动的重连，并把"设备已被驱动自行拔出"视为正常退出。（`crates/ns2pro-usb/src/usbip_win.rs`、`crates/ns2pro-bridge/src/usb/usbip_win.rs`）
3. **修复：手柄休眠后被"忘记"。** 上游在扫描超时（例如手柄休眠超过 30 秒）时也会清除已记住的手柄，下次连接会重新配对、且不再限定地址。现在蓝牙超时不再清除缓存，只有真正的连接错误才会。（`crates/ns2pro-bridge/src/cache.rs`）
4. 移除只适用于上游仓库的 release-plz 发布流程，仓库地址改为本仓库。

## 在 Windows 上使用

1. **安装 usbip-win2 驱动**：从 [usbip-win2 Releases](https://github.com/vadimgrn/usbip-win2/releases) 下载并安装已签名的版本（支持 v0.9.6.1–v0.9.8.1）。这是内核驱动，需要管理员权限，请自行确认来源后安装。
2. **获取程序**：从源码编译（推荐，见下方），或使用本仓库 Releases 中的 `ns2pro-bridge-x86_64-pc-windows-msvc.zip`（如有）。
3. **运行** `ns2pro-bridge.exe`，首次连接时按下手柄顶部的**同步键（SYNC）**进行配对，程序会记住手柄。
4. 看到 `controller initialized; receiving input` 后打开 Steam 即可。下次使用时运行程序并按 **Home** 键唤醒手柄。按 **Ctrl+C** 退出。

> ⚠️ **不要在 Windows 设置里配对这个手柄。** 否则 Windows 会一直占用手柄的蓝牙连接，程序将扫描不到它。如果已经配对过，请先在"设置 → 蓝牙和设备"里删除该设备。

可先运行 `ns2pro-bridge.exe --bt-smoke-test` 检查蓝牙适配器能否正常扫描。

### 延迟

Linux 下程序可以把蓝牙连接间隔设为 7.5 ms；Windows 不允许普通程序强制指定间隔，只能请求"吞吐量优先"，实际值由系统、蓝牙驱动和手柄共同决定。程序每次连接都会在日志中打印实际间隔：

```
Observed BLE connection interval  interval_units_1_25_ms=N
```

N 的单位是 1.25 ms：6 = 7.5 ms（理想），12 = 15 ms（良好），24 及以上 = 30 ms 以上（可能感觉到延迟）。加 `--log-level debug` 每 5 秒还会输出报告频率和延迟 p50/p99。如果延迟不理想，可以改用 USB-C 有线连接。

## 在 Linux 上使用

运行 `ns2pro-bridge`，按提示输入密码（程序会通过 `sudo`/`pkexec` 提权并自动加载 `vhci-hcd`），首次连接按同步键。Linux 二进制要求 glibc ≥ 2.39。32 位 Steam 能看到手柄但没有输入时，请安装 32 位 libusb（如 Arch 上的 `lib32-libusb`）并重启 Steam。

## 从源码编译

需要 Rust（`rust-toolchain.toml` 会自动安装指定版本）；Windows 还需要 Visual Studio Build Tools 的 C++ 工具。

```sh
cargo build --locked --release -p ns2pro-bridge
cargo test --locked --workspace
```

输出：`target/release/ns2pro-bridge`（Windows 下为 `.exe`）。

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

