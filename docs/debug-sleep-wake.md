# Sleep/Wake Recovery Debugging Reference

> 2026-06-30: onairclock and akp153 plugins remain non-functional after system sleep, despite two rounds of fixes.

## Symptoms

| Plugin | Type | Post-wake behavior |
|--------|------|-------------------|
| `fail.marc.onairclock` (`onairclock`) | JS webview (CodePath: index.html) | Clock shows one frame of current time, then stops ticking |
| `st.lynx.plugins.opendeck-akp153` (`akp153`) | Rust native binary (CodePathWin: opendeck-akp153-win.exe) | Completely dead — no response |

## Fix Attempts (both in `main` branch as of 2026-06-30)

### Fix 1: Restart all plugins on wake
**File:** `src-tauri/src/power_events.rs` (commit `88983df`)
**Change:** Replaced `reload_webview_plugins()` with `deactivate_plugins()` + loop over plugins dir calling `initialise_plugin()` for each.
**Why:** `reload_webview_plugins()` only handled webview plugins, and had a race condition (eval lands on old page before reload completes).

### Fix 2: Generation counter for PLUGIN_SOCKETS cleanup
**File:** `src-tauri/src/events/mod.rs` (commit `4efaafc`)
**Change:** Added per-UUID generation counter (`PLUGIN_CONN_GENERATION`, `PI_CONN_GENERATION`) to prevent stale cleanup tasks from removing newly registered socket entries.
**Why:** Race condition — when old TCP connection dies, its cleanup `tokio::spawn` task calls `PLUGIN_SOCKETS.remove(uuid)`, which could remove the NEW connection's entry if the new plugin reconnected before the old task completed.

## Key Files

### Sleep/wake flow
| File | Purpose |
|------|---------|
| `src-tauri/src/power_events.rs` | PSP power event monitor, `handle_wake()` entry point |
| `src-tauri/src/plugins/mod.rs` | `deactivate_plugins()`, `initialise_plugin()`, `reload_webview_plugins()`, `INSTANCES` map |
| `src-tauri/src/events/mod.rs` | `PLUGIN_SOCKETS`, `PROPERTY_INSPECTOR_SOCKETS`, `register_plugin()`, generation counters |
| `src-tauri/src/events/outbound/mod.rs` | `send_to_all_plugins()`, `send_to_plugin()` — WebSocket delivery |
| `src-tauri/src/events/outbound/misc.rs` | `system_did_wake_up()` — sends `{event: "systemDidWakeUp"}` over WebSocket |
| `src-tauri/src/elgato.rs` | `invalidate_hidapi()` (line 188), `initialise_devices()` (line 194) |
| `src-tauri/src/device_sleep.rs` | `note_activity()`, sleep/wake device management |

### Plugin spawning
| File | Purpose |
|------|---------|
| `src-tauri/src/plugins/manifest.rs` | Manifest parsing |
| `src-tauri/src/plugins/info_param.rs` | Info parameter generation for plugin launch |
| `src-tauri/src/plugins/webserver.rs` | `tiny_http` static file server for plugin assets |
| `src-tauri/src/events/frontend/plugins.rs` | Tauri commands: `reload_plugin()`, `install_plugin()`, etc. |

## Installed Plugin Paths

```
%APPDATA%/opendeck/plugins/
├── fail.marc.onairclock.sdPlugin/
│   ├── manifest.json      (CodePath: index.html, no CodePathWin)
│   ├── index.html          (loads app.js)
│   ├── app.js              (main plugin logic, has heartbeat + gLastSuccessTime fix)
│   ├── common/             (SDK common files: common.js, etc.)
│   └── propertyinspector/
├── st.lynx.plugins.opendeck-akp153.sdPlugin/
│   ├── manifest.json       (CodePathWin: opendeck-akp153-win.exe, PluginUUID different from dir name)
│   ├── opendeck-akp153-win.exe  (the binary)
│   └── assets/
├── com.amansprojects.starterpack.sdPlugin/
└── com.fredemmott.audiooutputswitch.sdPlugin/
```

**Note:** akp153's directory name is `st.lynx.plugins.opendeck-akp153.sdPlugin` but manifest `PluginUUID` is `st.lynx.plugins.opendeck-akp153` (without `.sdPlugin` suffix). The directory name is used as the UUID in `INSTANCES` and `PLUGIN_SOCKETS`.

## Source Repos

| Plugin | Path |
|--------|------|
| onairclock | `G:\Codes\opendeck\streamdeck-onairclock\fail.marc.onairclock.sdPlugin\` |
| akp153 | `G:\Codes\opendeck\opendeck-akp153\` |

## onairclock Modifications (installed version)

The installed `%APPDATA%/opendeck/plugins/fail.marc.onairclock.sdPlugin/app.js` differs from upstream:
1. **Removed `common/timers.js`** from index.html — native timers survive sleep better
2. **Added `gEventsRegistered` guard** — prevents duplicate event handlers on reconnect
3. **Added heartbeat reconnect** — polls `readyState` every 5s, reconnects if dead
4. **Added `gLastSuccessTime` gap detection** — detects sleep by wall-clock gap > 7s between heartbeats
5. **Added connection parameter caching** (`gCachedPort`, `gCachedUUID`, `gCachedInfoJSON`)

## Log File Locations

```
Main log:    %LOCALAPPDATA%/opendeck/logs/OpenDeck.log
Plugin logs: %LOCALAPPDATA%/opendeck/logs/plugins/<uuid>.log
```

**Key patterns to search in OpenDeck.log:**
- `"System wake detected"` — confirms PSP power event fired
- `"Re-init already in progress"` — duplicate wake guard hit
- `"Failed to re-init plugin after wake"` — initialise_plugin failed
- `"Registered plugin"` — plugin WebSocket reconnected
- `"Failed to initialise plugin"` — spawn failed
- `audiooutputswitch` — another plugin that may also be dead (test indicator)

**Key patterns in plugin logs:**
- akp153: `%LOCALAPPDATA%/opendeck/logs/plugins/st.lynx.plugins.opendeck-akp153.sdPlugin.log`

## Remaining Hypotheses

1. **akp153 binary doesn't start:** `initialise_plugin()` reads manifest → resolves code path → sends spawn request. Check plugin log for startup errors. After wake, does the `opendeck-akp153-win.exe` process exist in Task Manager? If not, the spawn failed. If it does exist but doesn't work, the WebSocket reconnect is broken.

2. **IP/TCP conflict:** After deactivate+reactivate, the new plugin process might try to connect before the old socket is fully released by the OS (TIME_WAIT). This is unlikely with `SO_REUSEADDR` but worth checking.

3. **`deactivate_plugins()` not firing:** Check if the `#[cfg(windows)]` block in `handle_wake()` is actually executing. Search logs for "System wake detected" and then for any errors related to plugin deactivation.

4. **Spawner thread stuck:** If the `std::sync::mpsc::Receiver` in the spawner thread is blocked on a previous spawn, new requests are queued but not processed.

## Fix 3: Atomic socket registration + webview reload (2026-06-30)

Two root causes identified and fixed:

### Fix 3a: Webview window close+recreate race (`power_events.rs`)

**Root cause:** `deactivate_plugins()` → `deactivate_plugin()` called `window.close()` on webview windows, then `initialise_plugin()` called `WebviewWindowBuilder::new()` with the **same label**. Tauri returns the existing (stale) window when a label already exists — the page is never reloaded. The JS eval creates a new WebSocket, but the clock's `setInterval` (running via the Stream Deck SDK's dead Web Worker) remains frozen.

**Fix:** Split the wake recovery path:
- **Webview plugins:** use `reload_webview_plugins()` — keeps the window handle, reloads the page via `window.reload()`, restores native timers via the iframe trick, and reconnects the WebSocket.
- **Native plugins:** kill via `deactivate_plugin()` + spawn fresh via `initialise_plugin()`.

### Fix 3b: Socket/generation race condition (`events/mod.rs`)

**Root cause:** `PLUGIN_SOCKETS` (socket map) and `PLUGIN_CONN_GEN` (generation counter) used separate locks. An old cleanup task could check the generation under one lock, then a new connection could insert a socket AND increment the generation, then the old cleanup removes the entry under the other lock — removing the NEW socket.

**Fix:** Merged socket and generation into a single `ConnEntry { socket, generation }` struct under one `Mutex`. Insert + generation increment happen atomically, and the cleanup check also happens atomically under the same lock.

### Fix 3c: `system_did_wake_up()` moved after plugin recovery

Previously this event was sent BEFORE plugin recovery (in the main `handle_wake()`), hitting dead WebSocket connections — the event was lost. Moved to after plugin recovery inside the spawned task, so it hits live connections.

### Files changed
- `src-tauri/src/events/mod.rs` — merged `PLUGIN_SOCKETS`/`PLUGIN_CONN_GEN` into `ConnEntry`
- `src-tauri/src/events/outbound/mod.rs` — updated socket field access
- `src-tauri/src/power_events.rs` — split webview/native recovery, moved `system_did_wake_up()`
- `src-tauri/src/plugins/mod.rs` — made `PluginInstance` and `INSTANCES` public

## Debug Checklist

- [ ] After wake, check Task Manager for `opendeck-akp153-win.exe` process
- [ ] Check `OpenDeck.log` for "System wake detected" and error messages
- [ ] Check plugin `.log` files for startup errors
- [ ] Test with audiooutputswitch plugin — does it also die? (if yes, systemic; if no, plugin-specific)
- [ ] Test onairclock separately: does the heartbeat log message `"[app.js] sleep detected"` appear in devtools console?
- [ ] Check if `deactivate_plugins()` actually removes entries from INSTANCES (can't verify without code instrumentation)
