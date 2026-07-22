# Sleep/Wake Recovery Fix Summary

> 2026-06-30: Two root causes fixed. Plugins now recover after system sleep.

## Root Cause 1: Webview Window Close+Recreate Race

**File:** `src-tauri/src/power_events.rs`

### Problem

原代码在系统唤醒后对所有插件执行 `deactivate_plugins()` + `initialise_plugin()`：

1. `deactivate_plugin()` 对 webview 插件调用 `window.close()`（10ms 超时）
2. `initialise_plugin()` 调用 `WebviewWindowBuilder::new()` 使用**相同窗口标签**创建新窗口

**Tauri 的行为：** 如果标签已存在，`WebviewWindowBuilder::new()` 返回**旧的（已僵死的）窗口**，而不是创建新窗口。页面不会被重新加载。此时虽然 eval 创建了新的 WebSocket 连接，但时钟的 `setInterval`（由 Stream Deck SDK 的 Web Worker 驱动，睡眠后已死）永远不会恢复。这导致「显示一帧后停住」的症状。

### Fix

将唤醒恢复路径拆分为两条：

| 插件类型 | 恢复方式 | 原因 |
|---------|---------|------|
| Webview | `reload_webview_plugins()` — 保留窗口句柄，调用 `window.reload()` 重新加载页面，通过 iframe 恢复原生定时器，重连 WebSocket | 窗口 close+recreate 不可靠 |
| 原生（二进制） | 保留原有 `deactivate_plugin()` kill 进程 + 新 spawn 进程 | 进程 kill+spawn 工作正常 |

## Root Cause 2: Socket/Generation 竞态条件

**File:** `src-tauri/src/events/mod.rs`

### Problem

`PLUGIN_SOCKETS`（存放 WebSocket 写入端）和 `PLUGIN_CONN_GEN`（generation 计数器）使用**两把独立锁**：

1. 旧清理任务在 `PLUGIN_CONN_GEN` 锁下检查 generation 匹配
2. 释放第一把锁后，新连接插入新 socket 并递增 generation（在 `PLUGIN_SOCKETS` 锁下）
3. 旧清理任务获取 `PLUGIN_SOCKETS` 锁，删除 socket——但此时 socket 已被替换为新的

**结果：** 插件 WebSocket 已连接但 `PLUGIN_SOCKETS` 中没有对应条目，OpenDeck 无法发送事件给插件。

### Fix

将 socket 和 generation 合并为一个 `ConnEntry` 结构体，用**同一把 `Mutex` 保护**：

```rust
struct ConnEntry {
    socket: SplitSink<WebSocketStream<TcpStream>, Message>,
    generation: u64,
}
```

插入 + generation 递增现在是一个原子操作，清理检查也在同一把锁下完成。

## Root Cause 3: `systemDidWakeUp` 事件丢失

**File:** `src-tauri/src/power_events.rs`

### Problem

`system_did_wake_up()` 在休眠恢复的**主线程**中立即发送，此时插件恢复还在 `tokio::spawn` 的延迟任务中（约 1 秒后才执行）。事件发送时插件 WebSocket 连接已全部断裂，消息发不到队列就失败了。

### Fix

将 `system_did_wake_up()` 移到 `tokio::spawn` 任务内部、放在插件恢复代码的**后面**，确保事件发送时插件已经重建了 WebSocket 连接。

## Changes Summary

| 文件 | 改动 |
|------|------|
| `src-tauri/src/events/mod.rs` | 合并 `PLUGIN_SOCKETS` / `PLUGIN_CONN_GEN` 为 `ConnEntry` |
| `src-tauri/src/events/outbound/mod.rs` | 适配 `entry.socket` 字段访问 |
| `src-tauri/src/power_events.rs` | 拆分 webview/原生恢复路径，移动 `system_did_wake_up()` |
| `src-tauri/src/plugins/mod.rs` | 公开 `PluginInstance` 和 `INSTANCES`（供 power_events 访问） |

## Verification

- `cargo check` 通过
- `cargo clippy -- -D warnings` 通过
- 构建 `dev` 分支版本号 `2.13.0`，安装后睡眠唤醒测试通过
- onairclock（WebView JS插件）和 akp153（Rust原生二进制插件）均能正常恢复
