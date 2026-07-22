# AKP153 设备冻结问题：排查、根因与修复总结

> 适用场景：用户使用 Ajazz AKP153（克隆 Stream Deck），设备"运行一小会后自己冻结/卡死"，且**不是**系统睡眠/唤醒场景。
> 本文档沉淀于 2026-07-22 的一轮修复，目的是让后续 agent 直接复用结论、跳过已排除的死路。

---

## 0. 环境与关键事实（先读这个，避免走偏）

- **设备**：Ajazz AKP153（VID `5548`，PID `6674`），`kind: AKP153`。
- **设备 ID**：`99-355499441494-153`。这是 v1 设备（Windows 返回无效序列号），由插件用 `kind.id_suffix()` 拼出，不是 `sd-<serial>` 方案。看到这个 ID 就说明走的是**插件注册路径**，不是原生 `elgato.rs` 路径。
- **`settings.json` 关键项**：`"disableelgato": true`。**这意味着 OpenDeck 主程序的 `elgato` 路径完全不碰 HID**；设备由插件 `st.lynx.plugins.opendeck-akp153` 经 WebSocket `registerDevice` 注册并持有 HID 句柄。
- **进程**：主程序 `D:\OpenDeck\opendeck.exe`（dev 构建安装版，PATH 指向 D 盘）；插件 `opendeck-akp153-win.exe` 在 `%APPDATA%\opendeck\plugins\st.lynx.plugins.opendeck-akp153.sdPlugin\`。
- **时钟插件**：`fail.marc.onairclock` 在 **button 17** 显示时钟，是设备上的主要持续写入源。
- **日志时区坑**：akp153 插件日志用 **UTC**；OpenDeck 主日志（`OpenDeck.log`）和系统事件用**本地时间（UTC+8）**。对比两个日志时先换算，否则会误判时间线。

---

## 1. 症状

- "运行一小会后自己冻结"，非唤醒相关。
- 设备出现 HID 致命错误（`0x8007001F` 设备未发挥作用 / `0x800703E3` I/O 已中止，常紧随系统的 `Disconnected` 事件）。
- 设备从 `DEVICES` 消失 → 之后每次 `save_profile` 报 `device not found`。
- 最终形态：日志停在 `Running device task`（重连卡在 `connect()`），设备永久"初始化中"= 看似冻结；或 `connect()` 反复 8s 超时 → 重扫 → 超时死循环。

---

## 2. 根因（三层叠加，逐层放大）

1. **诱因 — 时钟插件高频冗余刷图**：`fail.marc.onairclock` 以 `setInterval(..., 250)`（**4 次/秒**）无条件整屏重绘并 `setImage`，而时钟每秒才变一次，3/4 是冗余写入，压垮廉价 HID 设备。
2. **放大器 — 插件运行中无恢复**：设备运行中报 HID 致命错误时，`handle_error` 判定 fatal 后**永久结束设备任务**；但设备仍被系统枚举、不会触发 USB `Connected` 事件 → 设备永不重连。
3. **致命伤 — HID 句柄泄漏导致楔死（真正终态）**：`handle_error` 把设备移出 `DEVICES` 后 `Device` 被 `drop`，但 mirajazz 的 `Drop` **不保证及时关闭 HID 句柄** → 每次错误恢复周期泄漏一个句柄，堆积后设备彻底楔死（wedged），重连时 `Device::connect()` 永远阻塞超时。这才是"运行一小会后自己冻结"的本质。

> 关键认知：USB 选择性挂起关掉、时钟降到 1Hz 之后，设备**仍会楔死**，因为根因是第 3 层的句柄泄漏，不是频率也不是系统电源管理。

---

## 3. 已做的修复（含提交与文件）

### 仓库 `opendeck-akp153`（设备插件）— `G:\Codes\opendeck\opendeck-akp153`
- **`ce64caf`** `src/device.rs` 的 `handle_error` 在 fatal 错误后置位 `watcher::NEEDS_RESCAN`；`src/watcher.rs` 的 1 秒轮询检测到后 `continue 'outer'` 重扫并重生设备任务 → 运行中设备报错可自愈。
- **`efd0e78`** `src/device.rs` 给整段初始化（connect + 亮度 + 清屏 + flush）加 **8 秒 `tokio::time::timeout`** → 恢复时绝不会再卡死在 `connect()`。
- **`dc3fb1b`** `src/device.rs`：`handle_error` **不再**从 `DEVICES` 移除设备；改由 `device_task` 退出时 **显式 `remove` + `device.shutdown()`** 本地持有的 HID 句柄 → 彻底消除句柄泄漏楔死。

### 仓库 `streamdeck-onairclock`（时钟插件）— `G:\Codes\opendeck\streamdeck-onairclock`
- **`3c2a7e1`** `fail.marc.onairclock.sdPlugin/app.js`：刷新频率 `250ms → 1000ms` 并对齐到秒边界（流量降 4 倍）；新增**画面变化检测**（渲染结果相同跳过 `setImage`），滤掉冗余写入。

### 系统层（非代码）
- 关闭当前电源方案的 **USB 选择性挂起**（`powercfg /setacvalueindex SCHEME_CURRENT <USB子组> <选择性挂起设置> 0`，已对交流/直流均设为 0）。保留关闭无害。

---

## 4. 日志位置

- 插件：`%LOCALAPPDATA%\opendeck\logs\plugins\st.lynx.plugins.opendeck-akp153.sdPlugin.log`
- 主程序：`%LOCALAPPDATA%\opendeck\logs\OpenDeck.log`
- 进程状态：`Get-Process opendeck, opendeck-akp153-win`

---

## 5. 已被排查并排除的方向（**后续 agent 不要再重复！**）

- ❌ **USB 选择性挂起**：已关闭，但设备仍楔死 → 不是主因（保留关闭无害，但别再把它当根因）。
- ❌ **时钟刷新频率**：已降到 1Hz，设备仍楔死 → 频率只是诱因，不是根因；根因是句柄泄漏。
- ❌ **device_sleep（设备空闲休眠）**：`settings.json` 中 `sleep_timeout_minutes: 0` 已禁用，排除。
- ❌ **系统睡眠/唤醒**：本次非唤醒场景。注意 `src-tauri/src/elgato.rs` 的 `reinitialise_devices()` / `DEVICE_EPOCH` 修复是针对**原生 `elgato` 路径的唤醒恢复**，**在 `disableelgato: true` 配置下整条不执行**，别被带偏去改 `main.rs`/`elgato.rs`/`power_events.rs`。
- ❌ **主程序 `elgato` 路径**：`disableelgato=true` 时 `initialise_devices` 提前返回，不持有 HID；设备生命完全在 `opendeck-akp153` 插件内。排查应聚焦插件，而非主程序 HID 代码。
- ⚠️ **`delayed_restart: spawning launcher for D:\OpenDeck\opendeck.exe`**：这是 OpenDeck 自动更新/重启机制的正常日志，不是故障，别把它当错误。

---

## 6. 未来出现问题时的排查决策树

1. **进程还活着吗？** `Get-Process opendeck, opendeck-akp153-win`。
   - 都活着但设备无响应 → 是设备/插件层问题（见下），不是主程序崩溃。
   - 主程序死了 → 看 `OpenDeck.log` 崩溃栈，方向与本问题无关。
2. **读 akp153 插件日志尾部**（注意 UTC）：
   - 出现 `Device error triggered rescan` → 自动恢复机制**在工作**，错误被捕获并重连。
   - 出现 `Timed out initializing device ...` → `connect()` 超时（句柄仍楔死或设备物理断连）。
   - 日志在 `Running device task` 后**完全停止、无新条目** → `connect()` 卡死（应确认 8s 超时已生效/已部署）；或设备已恢复且无错误（看是否有后续 `Registering device`）。
   - 持续每 ~1s 出现 `error triggered rescan` → 设备反复复位（固件/物理层），软件只能不停重连，无法根治。
3. **若仍楔死**：优先怀疑 (a) HID 句柄未关闭（检查 `device_task` 清理是否显式 `shutdown`）；(b) 设备物理断连/供电不足。用 `hid` 探测工具或**物理重插**验证设备是否可达。
4. **软件恢复的边界**：软件层目标是在"固件正常的偶发 HID 抖动"下自愈（~1s 重连）。若设备**固件楔死**（物理层），只有物理重插 / 换后置直连主板的 USB 口 / 换线 / 带供电 hub 能恢复——软件无法打开一个固件已死的 HID 设备。

---

## 7. 关键文件速查

| 仓库 | 文件 | 关注点 |
|------|------|--------|
| `opendeck-akp153` | `src/device.rs` | `handle_error`（置 `NEEDS_RESCAN`，不再移 `DEVICES`）、`device_task`（8s 超时 + 退出显式 `shutdown`）、`connect` |
| `opendeck-akp153` | `src/watcher.rs` | `NEEDS_RESCAN` 标志、1s 轮询重扫、`WOKE_UP` |
| `streamdeck-onairclock` | `fail.marc.onairclock.sdPlugin/app.js` | `updateClock` 的 `setInterval` 频率、`drawClockImg` 的变化检测 |
| OpenDeck 主仓库 | `src-tauri/src/elgato.rs` | `reinitialise_devices`/`DEVICE_EPOCH`：**仅原生路径、disableelgato 下不生效**，勿在此浪费时间 |
| OpenDeck 主仓库 | `src-tauri/src/device_sleep.rs` | 空闲休眠逻辑；本机已 `sleep_timeout_minutes=0` 禁用 |
