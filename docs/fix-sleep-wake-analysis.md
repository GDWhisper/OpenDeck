# Sleep/Wake Recovery 完整分析

> 2026-06-30 分析，基于 `dev` 分支 commit `2681df4`

## 背景

OpenDeck 在系统睡眠后，插件的 WebSocket 连接全部断裂。两个受影响最明显的插件：

| 插件 | 类型 | 症状 |
|------|------|------|
| `fail.marc.onairclock` (onairclock) | JS WebView (CodePath: index.html) | 显示一帧当前时间，然后不再更新 |
| `st.lynx.plugins.opendeck-akp153` (akp153) | Rust 原生二进制 (CodePathWin: .exe) | 完全没有响应 |

## 上游原始状态

> ⚠️ **修正（2026-07-08）：本节原结论「上游完全没有睡眠恢复逻辑」事实有误。** 详见下方核对。

### d9ff9f8 当时（fork 基点之前的上游提交）

原作者（nekename/Aman Khanna）的 `power_events.rs`（commit `d9ff9f8`，"feat: support disabling devices while the computer screen is locked (#325)"）中：

```rust
PowerState::Suspend | PowerState::Resume |
    PowerState::Shutdown | PowerState::Unknown => {}
//  ↑ 在 d9ff9f8 这个提交点，Resume 确实被忽略
```

- 在 d9ff9f8 时：没有 HID 设备重枚举、没有插件进程重启/重连。

### 但上游随后补上了唤醒事件（且已包含在 fork 基点上）

上游在 commit `b9e4735`（作者 **nekename**，"feat: implement the `systemDidWakeUp` event"）中为 `PowerState::Resume` 增加了 `system_did_wake_up()` 调用，会把 `systemDidWakeUp` 事件广播给所有已连接的插件（见 `upstream/main:src-tauri/src/power_events.rs`）：

```rust
PowerState::Resume => {
    tauri::async_runtime::spawn(async {
        if let Err(error) = crate::events::outbound::misc::system_did_wake_up().await {
            log::error!("Failed to send the systemDidWakeUp event: {error}");
        }
    });
}
```

因此，**上游确实拥有睡眠恢复逻辑**——至少在「系统唤醒时向插件广播 `systemDidWakeUp`」这一层，它依赖插件自行恢复（自行重连 WebSocket 等）。

**上游没有做的**：HID 设备重枚举、插件进程 kill+restart、WebView 重载。这些才是本 fork 额外补充的硬恢复部分。

> 注：`b9e4735` 已存在于 `dev` 分支的基点历史中（位于 `d9ff9f8` 之后），并非本 fork 新增。

## 我们的改动时间线

> ⚠️ **修正：** commit `b9e4735`（"feat: implement the `systemDidWakeUp` event"）作者为 **nekename（上游）**，并非 GDWhisper，且已包含在 fork 基点的历史中，不属于「我们的改动」。下表只列出 GDWhisper 在 `dev` 分支上实际新增的提交。

| 顺序 | Commit | 作者 | 做了什么 |
|------|--------|------|---------|
| 1 | `7eea216` | GDWhisper | 引入 `handle_wake()` 入口，替换原来的墙钟睡眠检测为 psp 事件 |
| 2 | `cd2292c` | GDWhisper | 加入 `invalidate_hidapi()` + `initialise_devices()` + `reload_webview_plugins()` |
| 3 | `88983df` | GDWhisper | **关键变动：** 把 `reload_webview_plugins()` 替换为 `deactivate_plugins()` + 所有插件 `initialise_plugin()`（包括原生插件 kill+restart），**塞入 `tokio::spawn` + 1 秒延迟** |
| 4 | `4efaafc` | GDWhisper | 加入 generation counter 防止 socket 清理竞态 |
| 5 | `2681df4` | GDWhisper | **本次修复：** atomic socket + 拆分 webview/原生恢复路径 + 修复 `systemDidWakeUp` 发送时机 |

> 上游已通过 `b9e4735` 提供 `systemDidWakeUp` 事件本身；本 fork 在此基础上补充了设备重枚举与插件/WebView 进程级恢复，并修正了事件发送时机。

## 我们自己引入的 Bug

### Bug A：WebView 窗口 close+recreate 竞态条件

**引入：** commit `88983df`（我们第 4 步）

**机制：**
1. `deactivate_plugin(webview)` 调用 `window.close()`，等 10ms
2. `initialise_plugin()` 用相同标签调用 `WebviewWindowBuilder::new()`
3. Tauri 发现标签已存在（10ms 不够窗口关闭），**返回旧窗口**
4. 旧窗口页面没有刷新，Stream Deck SDK 的 Web Worker 在睡眠后已死，`setInterval` 不触发
5. eval 创建了新 WebSocket，但时钟动画死在旧线程里

**影响：** onairclock「显示一帧后停住」

### Bug B：Socket/Generation 竞态条件

**引入：** commit `4efaafc`（我们第 5 步）

**机制：**
1. `PLUGIN_SOCKETS`（存 socket）和 `PLUGIN_CONN_GEN`（存 generation）是两把独立锁
2. 旧清理任务在 `PLUGIN_CONN_GEN` 锁下确认 generation 匹配 ✓
3. 释放锁的间隙，新连接插入新 socket 并递增 generation
4. 旧清理任务拿到 `PLUGIN_SOCKETS` 锁，删除 entry——但此 socket 已被换成了新的
5. 插件 WebSocket 连着但 `PLUGIN_SOCKETS` 里没记录 → OpenDeck 无法向插件发送事件

**影响：** 可能影响所有插件（socket 被错误清理）

### Bug C：`systemDidWakeUp` 发送时机错误

**引入：** 上游已提供 `systemDidWakeUp` 事件（commit `b9e4735`，nekename）；我们 `88983df` 把插件 kill+restart 塞进延迟 `tokio::spawn` 后，事件（在 `handle_wake()` 主线程立即发送）与此延迟恢复之间产生了时序差

**机制：**
1. `system_did_wake_up()` 在 `handle_wake()` 主线程立即发送
2. 原生插件的 kill+restart 在 `tokio::spawn` 里 1 秒后才执行
3. 发事件时原生插件的 WebSocket 还没恢复 → 事件丢失
4. 原生插件永远收不到 `systemDidWakeUp`

**影响：** akp153 无法接收到唤醒通知

## 本次修复内容（commit `2681df4`）

### 修复 A：拆分 webview/原生恢复路径

`power_events.rs`:

- **WebView 插件：** 改用 `reload_webview_plugins()`（保留窗口 + `window.reload()` + iframe 恢复原生定时器 + 重连 WebSocket），不走 close+recreate
- **原生插件：** 保留 kill+restart（`deactivate_plugin()` + `initialise_plugin()`），这个对原生进程是正确可靠的

### 修复 B：合并 Socket+Generation 为单锁

`events/mod.rs`:

```rust
struct ConnEntry {
    socket: SplitSink<...>,
    generation: u64,
}
```

socket 插入和 generation 递增现在在同一把 `Mutex` 下原子完成，清理检查也在同一把锁下。

### 修复 C：移动 `systemDidWakeUp` 发送时机

`power_events.rs`:

- 从 `handle_wake()` 主线程移到 `tokio::spawn` 任务内部
- 放在插件恢复代码**之后**，确保事件到达活跃连接

## 评估

| 问题 | 是否上游原有 | 是否能靠回滚上游解决 |
|------|------------|-------------------|
| 插件睡眠后无恢复 | **部分**（上游只广播 `systemDidWakeUp`，不做设备重枚举/进程重启） | **不能**——回滚上游后插件连接仍会断裂，只是会收到 `systemDidWakeUp` 通知 |
| WebView close/recreate 竞态 | 否（我们引入） | 不能——回滚上游后无此竞态，但也没有 WebView 恢复 |
| Socket/Generation 竞态 | 否（我们引入） | — |
| systemDidWakeUp 时序 | 否（我们引入） | — |

**结论（修正）：** 文档原结论「上游根本没有睡眠恢复逻辑」**有误**——上游（commit `b9e4735`，nekename）已在系统唤醒时向插件广播 `systemDidWakeUp` 事件；它只是依赖插件自行恢复，且不做 HID 重枚举 / 插件进程重启 / WebView 重载。本 fork 真正补充的是**设备重枚举 + 插件进程重启 + WebView 重载 + atomic socket** 这几层硬恢复；Bug A/B/C 仍是我们在此过程中引入、并已在 `2681df4` 修正的问题。回滚上游无法解决设备/进程级恢复问题，但上游确实提供了唤醒通知这一基础能力。
