# Bug Fixes & Lessons Learned

This document records bugs encountered and resolved during development, with root causes, fixes, and lessons learned. When a bug fix is confirmed, add an entry here so future work can reference past experience.

---

## BF-001: WebView plugin freezes after system sleep/wake

**Date:** 2026-06  
**Files:** `src-tauri/src/plugins/mod.rs` (`reload_webview_plugins()`)  
**Symptom:** After Windows sleep/wake, webview-based plugins (e.g. onairclock) stop updating. The clock freezes, animations stop, but no errors in the log.

**Root cause:** `window.eval("location.reload()")` goes through the JS engine, which can be in a degraded state after system sleep. The `eval()` call returns `Ok` (the command was injected), but the page navigation never actually happens. Logs showed "Page reload triggered" followed by "Reconnect JS evaluated" but the plugin remained frozen. This worked on the first wake but failed on subsequent wakes.

**Fix:** Replace `window.eval("location.reload()")` with Tauri's native `window.reload()`. This goes through Rust -> WebView2 controller (ICoreWebView2), bypassing the page's JS engine entirely. Equivalent to pressing the browser's refresh button from outside the page.

**Lesson:** Never rely on JS-level page navigation for recovery after system sleep. The JS engine itself may be degraded. Always prefer native/host-level APIs (Tauri's `WebviewWindow::reload()`, `navigate()`) for critical recovery paths.

---

## BF-002: Infinite reconnection loop in webview plugin

**Date:** 2026-06  
**Files:** Plugin JS code (`app.js`)  
**Symptom:** After sleep/wake, the plugin registers itself 20+ times per second, flooding the WebSocket with "Handshake not finished" errors. CPU spikes.

**Root cause:** The plugin's `connected()` callback registers event handlers every time it fires (on initial connect AND every reconnect). When OpenDeck's watchdog broadcasts `systemDidWakeUp`, the plugin's wake handler reconnects the WebSocket, which triggers `connected()` again, which registers a new wake handler, which fires again on the next wake event — an infinite loop.

**Fix:** Add a `gEventsRegistered` guard flag so `connected()` only registers event handlers once:
```javascript
var gEventsRegistered = false;
function connected(jsn) {
    if (gEventsRegistered) return;
    gEventsRegistered = true;
    $SD.on('someEvent', handler);
    // ...
}
```

**Lesson:** Any callback that runs on reconnect must guard against re-registering handlers. Idempotent registration is essential when connections can be recycled.

---

## BF-003: Elgato SDK `timers.js` kills timers after sleep

**Date:** 2026-06  
**Files:** Plugin HTML (`index.html`), Elgato SDK (`common/timers.js`)  
**Symptom:** After sleep/wake, all `setInterval`/`setTimeout` calls in the plugin stop firing, even if the WebSocket is reconnected. Clock animations freeze.

**Root cause:** The Elgato SDK's `timers.js` overrides `window.setInterval` and `window.setTimeout` with Web Worker-based replacements (`ESDTimerWorker`). The Worker thread dies during system sleep and does not resume. All timer-based code silently stops working.

**Fix:** Remove the `<script src="common/timers.js"></script>` include from the plugin's HTML. This lets the browser's native `setInterval`/`setTimeout` remain in effect, which survive system sleep and resume correctly after wake.

**Lesson:** Web Workers do not survive system sleep. Any SDK/library that replaces native browser APIs with Worker-based alternatives will break after sleep. Audit third-party SDKs for Worker usage when sleep/wake resilience is required.

---

## BF-004: Tray "Restart" only closes the app, doesn't relaunch

**Date:** 2026-06  
**Files:** `src-tauri/src/main.rs`  
**Symptom:** Right-click tray icon -> Restart closes OpenDeck but does not start a new instance. Behaves identically to Quit.

**Root cause:** Two interacting bugs:

1. Tauri's `app.restart()` spawns the new process BEFORE the old one exits. The `tauri-plugin-single-instance` uses a named mutex; the new process finds the mutex still held by the old process and exits immediately.

2. `app.restart()` internally calls `exit()`, which triggers `RunEvent::Exit`. But the new process is already dead by this point.

**Fix:** Replace `app.restart()` with a custom `delayed_restart()` that:
1. Spawns a detached `cmd.exe` with a 3-second ping delay
2. After the delay, uses `start /B` to launch the new process
3. Calls `app.exit(0)` to cleanly shut down the old process

By the time the new process starts (3s later), the old process has fully exited and released the mutex.

**Lesson:** `app.restart()` is incompatible with single-instance plugins on Windows. The spawn-before-exit ordering causes a mutex race. Custom restart logic (exit first, then launch with delay) is required.

---

## BF-005: `tokio::spawn` in `RunEvent::Exit` never executes

**Date:** 2026-06  
**Files:** `src-tauri/src/main.rs` (exit handler)  
**Symptom:** Device reset (`elgato::reset_devices()`) was supposed to run on exit but never actually executed. Devices remained in their last state after app exit.

**Root cause:** `tokio::spawn(elgato::reset_devices())` fires the task into the Tokio scheduler, but `RunEvent::Exit` fires during runtime shutdown. The runtime is already tearing down — spawned tasks never get polled. The original code likely intended fire-and-forget, but the "forget" part was the only thing that happened.

**Fix:** Replace `tokio::spawn()` with `futures::executor::block_on()`, consistent with how `deactivate_plugins()` was already handled on the line above:
```rust
futures::executor::block_on(plugins::deactivate_plugins());
futures::executor::block_on(elgato::reset_devices());
```

**Lesson:** In shutdown/exit handlers, the async runtime is already shutting down. `tokio::spawn` becomes a no-op. Use `block_on` (synchronous blocking) for any async work that must complete before exit.

---

## BF-006: `timeout.exe` silently fails under `CREATE_NO_WINDOW`

**Date:** 2026-06  
**Files:** `src-tauri/src/main.rs` (`delayed_restart()`)  
**Symptom:** The delayed restart cmd.exe was spawned successfully, but the `timeout` command produced no delay and the exe was never launched. Restart appeared to do nothing.

**Root cause:** `timeout.exe` is a console application that reads from stdin (for the "Press any key to continue" prompt) and writes to stdout. When spawned with `CREATE_NO_WINDOW` (0x08000000), it has no console attached. It silently fails and exits immediately, so the `&&` chain never reaches the exe launch.

**Fix:** Replace `timeout /t N /nobreak` with `ping -n N+1 127.0.0.1 > nul`. `ping` is a network utility that works without a console. Each ping to loopback takes ~1 second, giving a reliable delay.

**Lesson:** Console applications (`timeout`, `pause`, `choice`, etc.) require a console to function. When using `CREATE_NO_WINDOW`, substitute with non-console utilities (`ping`, `powershell -Command Start-Sleep`). Always test spawned commands under the same creation flags as production.

---

## BF-007: Stream Deck device lost after sleep — HIDAPI cache + double-wake race + silent reader loop

**Date:** 2026-06  
**Files:** `src-tauri/src/elgato.rs`, `src-tauri/src/system_sleep_watchdog.rs`  
**Symptom:** After system sleep/wake, the Stream Deck device disappears. Profile save fails with `device not found`. After manual restart, the `akp153` plugin (device hardware) doesn't register. Log showed `System sleep/wake detected` followed by webview plugin reconnection, but no `Registered plugin` for the device itself.

**Root cause:** Three interacting bugs:

1. **HIDAPI stale cache:** `elgato.rs` caches `HidApi` in a static `HIDAPI: RwLock<Option<Arc<HidApi>>>`. After sleep, the USB subsystem re-enumerates devices with new handles, but the cached instance holds stale handles. `list_devices_async(&hid)` with stale handles either returns empty or fails to connect.

2. **Double-wake race:** The watchdog's `last_check` is reset only after re-init completes. If a second wake fires during the 2s stabilization delay + re-init time, it triggers another concurrent `initialise_devices()`, potentially interfering with the first.

3. **Silent reader loop exit:** `init()` had `Err(_) => break` in the reader loop with no logging. When the device handle died after sleep, the reader exited silently, removing the device from `ELGATO_DEVICES` and deregistering it — with zero diagnostic output.

**Fix:**
1. Added `invalidate_hidapi()` — clears the cached `HidApi` so `initialise_devices()` creates a fresh instance with current USB handles. Called from watchdog before `initialise_devices()`.
2. Added `AtomicBool` guard (`REINIT_IN_PROGRESS`) with `compare_exchange` to prevent concurrent re-init.
3. Added `log::warn!` on reader loop error and `log::info!` on device deregistration.

**Lesson:** `hidapi`'s `HidApi` instance holds OS-level USB handles that become invalid after sleep. Any code that caches `HidApi` across sleep boundaries must invalidate and recreate it on wake. Silent error swallowing in device I/O loops makes post-sleep failures invisible — always log errors at device boundaries.

---

## General Patterns

**Sleep/wake resilience:** System sleep degrades more than just network connections. The JS engine, Web Workers, WebSocket connections, and even some OS-level console APIs can become unreliable. Recovery code should use the most native/host-level APIs available.

**Exit handler hygiene:** In any process exit handler, assume the async runtime is already dead. Use synchronous blocking for all cleanup. Never fire-and-forget.

**Single-instance + restart:** On Windows, named mutexes are released when the process fully terminates, not when `exit()` is called. Any restart mechanism must ensure the old process is completely gone before the new one starts.

**CREATE_NO_WINDOW pitfalls:** Many Windows utilities assume a console exists. When hiding windows, test every command in the chain individually under the same flags.

**USB handle caching:** Libraries like `hidapi` cache OS-level device handles. After system sleep, USB re-enumeration gives devices new handles, but cached instances remain stale. Any static/cached handle must be invalidated on wake before re-enumeration. Silent error swallowing in I/O loops (e.g. `Err(_) => break`) hides post-sleep failures — always log errors at device boundaries.
