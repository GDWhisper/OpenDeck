> **Fork Notice**: This is a Windows-focused fork of [nekename/OpenDeck](https://github.com/nekename/OpenDeck).
> The main addition is system sleep/wake recovery for Stream Deck devices and webview plugins —
> a scenario the upstream does not yet handle. All other architecture and conventions remain identical to upstream.

# OpenDeck Development Guide

## Architecture Overview

OpenDeck is a Tauri desktop application for controlling Elgato Stream Deck devices. It's built with:
- **Backend**: Rust (Tauri v2) - device communication, plugin management, WebSocket/HTTP servers
- **Frontend**: SvelteKit + TypeScript + Tailwind CSS v4 - UI rendered in webview
- **Build Tool**: Deno (not Node.js) - manages tasks and dependencies

### Core Architecture Pattern

OpenDeck acts as a **host application** that communicates with **plugins** (separate processes):
1. Plugins connect via WebSocket (port dynamically allocated, starting from 57116)
2. Static assets served via `tiny_http` webserver (port = WebSocket port + 2)
3. Plugin property inspectors (HTML/JS) run in iframes and use separate WebSocket connections
4. Device button presses/releases trigger events sent to plugins via WebSocket

Key data flow: `Device (elgato-streamdeck crate) → Rust event handlers → WebSocket → Plugin process`

### Sleep/Wake Recovery

OpenDeck detects system sleep/wake by polling wall-clock time (`SystemTime::now()`) at 1 Hz. If the gap between consecutive polls exceeds 3 seconds, the system likely slept. A 2-second USB stabilization delay follows, then all devices are re-enumerated and any idle-slept devices are woken. This handles USB device disconnection/reconnection that occurs on many systems during sleep (cf. issues #335, #38).

**Webview plugin recovery** (`reload_webview_plugins()` in `plugins/mod.rs`): After device re-init, webview-based plugins (e.g. clock displays) are reconnected in two steps:
1. **Tauri native `window.reload()`** — goes through Rust → WebView2 controller (ICoreWebView2), bypassing the JS engine which may be degraded after sleep. JS-level `location.reload()` is unreliable: `eval()` returns Ok but the navigation never happens on subsequent wakes.
2. **Re-evaluate connection init JS** — restores native browser timers via a hidden iframe (to counter Elgato SDK's Web Worker timer override dying during sleep), then re-runs `connectElgatoStreamDeckSocket` to establish a fresh WebSocket and send `registerPlugin`.

The two layers provide redundancy: Tauri reload handles page-level recovery; plugin-side WebSocket heartbeats (if implemented) handle connection-level self-recovery.

## Project Structure

```
src-tauri/src/              # Rust backend
├── main.rs                 # Entry point, Tauri setup, tray icon
├── elgato.rs               # Direct hardware communication (elgato-streamdeck crate)
├── plugins/                # Plugin lifecycle, WebSocket/HTTP servers
├── events/                 # Event routing (inbound/outbound/frontend)
├── store/                  # JSON file-based persistence (profiles, settings)
├── application_watcher.rs  # Auto-switch profiles based on active window
├── device_sleep.rs         # Idle-timeout screen sleep for devices
└── system_sleep_watchdog.rs # Detect system sleep/wake, reinit devices on resume

src/                        # SvelteKit frontend
├── lib/                    # TypeScript types mirroring Rust structs
├── components/             # Svelte UI components
└── routes/                 # SvelteKit routing (currently single-page app)

plugins/com.amansprojects.starterpack.sdPlugin/  # Plugin with basic actions
├── assets/manifest.json                         # Plugin metadata
├── assets/propertyInspector/                    # HTML UIs for action settings
└── src/                                         # Rust plugin using openaction crate
```

## Critical Workflows

### Development Commands

```bash
# Frontend dev server (Vite HMR on port 5173)
deno task dev

# Run Tauri app in dev mode (spawns frontend + Rust app)
deno task tauri dev

# Build production bundle
deno task tauri build

# Build Rust backend only (without deno, e.g. during CI or for quick testing)
npm run build                                    # Build frontend first
cd src-tauri && cargo build --features custom-protocol  # Build + run
```

### Pre-commit Requirements

Before commits, **always** run:
1. `cargo clippy` (no violations allowed)
2. `cargo fmt` (in both `src-tauri/` and plugin directories)
3. `deno check`, `deno task check` and `deno lint` (no violations)

These are project standards, not suggestions.

### Built-in Plugins

Built-in plugins included in OpenDeck are Rust binaries. The `build.ts` script in each plugin compiles for multiple targets (x86_64/aarch64) and organizes binaries by OS.

## Key Conventions

### Type Synchronization

TypeScript types in `src/lib/` **must** mirror Rust structs in `src-tauri/src/shared.rs`:
- `Context`, `ActionInstance`, `ActionState`, `DeviceInfo`, `Profile`
- Changes to Rust structs require updating corresponding TypeScript types

### Context System

A `Context` identifies a button/encoder position:
```rust
struct Context {
    device: String,    // Device vendor prefix and serial number
    profile: String,   // Profile name
    controller: String, // "Keypad" or "Encoder"
    position: u8,      // Key index or encoder number
}
```

An `ActionContext` extends this with an action instance index for nested actions (e.g., multi-actions).

### State Management

- **Backend**:
  - `DEVICES` (DashMap): Thread-safe device registry, keyed by device ID
  - `CATEGORIES` (RwLock): Plugin actions organized by category for UI
  - `Store<T>`: Generic JSON persistence with file locking, backup, and atomic writes
  - Profile locks: Use `acquire_locks()` (read) or `acquire_locks_mut()` (write) before accessing profiles
- **Frontend**:
  - Svelte stores (`propertyInspector.ts`): `inspectedInstance`, `copiedContext`, `openContextMenu` for UI state
  - Tauri `invoke()` for backend calls - returns Promises with typed results
- **Persistence**: JSON files in config dir (see `store/mod.rs`), with `.temp` and `.bak` for crash recovery

### Plugin Communication

- **WebSocket protocol**: Plugins/PIs connect to `localhost:PORT_BASE`, send JSON messages with `event` field
- **Message routing**: `inbound::InboundEventType` enum handles all incoming events, `outbound::` modules send to plugins
- **Outbound event types**: `willAppear`, `keyDown`, `keyUp`, `dialRotate`, etc. (Stream Deck SDK compatible)
- **Authentication**: Context validation ensures plugins can only access their own action instances
- **Plugin manifests** (`manifest.json`: Stream Deck SDK format + extensions):
  - `CodePathLin`: Linux binary path
  - `CodePaths`: Map of Rust target triples to binaries
  - Platform overrides: `manifest.{os}.json` files merged via `json-patch`
- **Property inspectors**: Communicate with plugins via `sendToPlugin`/`sendToPropertyInspector`

### Cross-Platform Considerations

- **Wine support**: Plugins compiled for Windows can run on Linux/macOS via Wine (spawned as child processes)
- **Device access**: Linux requires udev rules (`40-streamdeck.rules`), installed automatically with .deb/.rpm
- **Flatpak**: Special handling for paths (`is_flatpak()` checks), Wine must be installed natively

## Common Patterns

### Adding a New Tauri Command

1. Define handler in `src-tauri/src/events/frontend.rs`
2. Add to `invoke_handler![]` macro in `main.rs`
3. Call from frontend: `await invoke<ReturnType>("command_name", { arg })`

### Profile Management

Profiles are device-specific JSON files in `<config_dir>/<device_id>/<profile_name>.json`:
```rust
// Read profile
let locks = crate::store::profiles::acquire_locks().await;
let profile = locks.profile_stores.get_profile_store(&device, "Default")?;

// Modify profile
let mut locks = crate::store::profiles::acquire_locks_mut().await;
let slot = crate::store::profiles::get_slot_mut(&context, &mut locks).await?;
*slot = Some(new_instance);
crate::store::profiles::save_profile(&device.id, &mut locks).await?;
```

Auto-switching: `application_watcher.rs` polls active window every 250ms, triggers profile changes via `SwitchProfileEvent` emitted to frontend.

### Event Flow Examples

**Button press**: `elgato.rs` → `outbound::keypad::key_down()` → WebSocket → Plugin's `key_down` handler
**Set image**: Plugin sends `setImage` → `inbound::states::set_image()` → `elgato::update_image()` → Device hardware
**Property inspector**: User edits in iframe → `sendToPlugin` → Plugin updates → `setSettings` → Profile saved

## Integration Points

### External Dependencies

- `elgato-streamdeck`: Async hardware communication via HID, image format conversion for different device types
- `tauri-plugin-*`: Dialog (file picker), logging (to file), autostart, single-instance, deep-link (opendeck:// URLs)
- `tokio-tungstenite`: WebSocket server for plugin communication
- `tiny_http`: Static file server for plugin assets (icons, property inspectors)
- `image`: Image loading/manipulation, format conversion for device displays
- `enigo`: Keyboard/mouse input simulation (starter pack plugin)
- `active-win-pos-rs`: Detect focused application for profile switching (polls every 250ms)
- `sysinfo`: Process monitoring for ApplicationsToMonitor feature

### WebSocket Protocol

Port allocation: `PORT_BASE` (WebSocket), `PORT_BASE + 2` (HTTP static files)
- Dynamic port selection: Tries ports starting at 57116 until both WebSocket and HTTP ports are available
- Registration: Plugins send `RegisterEvent::RegisterPlugin { uuid }`, property inspectors send `RegisterPropertyInspector`
- Message queuing: `PLUGIN_QUEUES` buffers messages until plugin connects
- Separate socket collections: `PLUGIN_SOCKETS` and `PROPERTY_INSPECTOR_SOCKETS` (HashMap of uuid → WebSocket sink)
- Plugin lifecycle: Socket registered → messages processed → socket removed on disconnect

### File Locations

```
Config:  %APPDATA%\opendeck\                          (Windows, e.g. C:\Users\<user>\AppData\Roaming\opendeck\)
         ~/.config/opendeck/                           (Linux)
         ~/Library/Application Support/opendeck/       (macOS)
Logs:    %LOCALAPPDATA%\opendeck\logs\                 (Windows, e.g. C:\Users\<user>\AppData\Local\opendeck\logs\)
         ~/.local/share/opendeck/logs/                 (Linux)
         ~/Library/Logs/opendeck/                      (macOS)
Plugins: <config_dir>/plugins/
```

Flatpak uses different paths with `~/.var/app/me.amankhanna.opendeck/` prefix.

## Testing & Debugging

- Run from terminal to see live logs: `deno task tauri dev`
- Plugin logs: Check `<log_dir>/plugins/<uuid>.log` (stdout/stderr captured from plugin processes)
- Debug logging: Uses Rust `log` crate (`log::debug!`)
- Frontend: Tauri devtools accessible via right-click → "Inspect Element"

## Known Pitfalls

### `custom-protocol` Feature

The Tauri webview on Windows serves embedded frontend assets via `tauri://localhost`, which requires the `custom-protocol` cargo feature:

```toml
# Cargo.toml
[features]
custom-protocol = ["tauri/custom-protocol"]
```

- **`cargo build` (without tauri CLI)**: Must pass `--features custom-protocol` explicitly. Without it, the webview shows "local 拒绝连接".
- **`cargo tauri build` / `deno task tauri build`**: Tauri CLI enables this automatically in release builds.

### Deno Required for Built-in Plugins

`build.rs` runs `deno task build` for each plugin in `../plugins/`. If deno isn't available:

- Debug mode (`cargo build`): Prints a warning and continues.
- Release mode (`cargo tauri build`): Panics and aborts the build.

**Installing deno on Windows**: `npm install -g deno` creates a `.cmd` wrapper at the npm global bin, but Rust's `Command::new("deno")` (used by `build.rs`) only finds `.exe` files. The real binary is inside the npm package, typically at `<npm-global>/node_modules/deno/deno.exe`. Add that directory to PATH before building:

```cmd
set PATH=<npm-global>\node_modules\deno;%PATH%
cargo tauri build --no-bundle
```

**Skipping plugin builds**: If the starterpack plugin can't compile (e.g. git dependencies need a proxy for GitHub), temporarily replace the `plugins/` directory with an empty one so `build.rs` iterates zero entries. Restore it after the build — the main OpenDeck crate doesn't depend on the starterpack binary.

To work around without deno entirely, modify `build.rs` to make the panic non-fatal (use `eprintln!` instead of `panic!`).

### Frontend Changes Not Picked Up After `cargo build`

Tauri's `generate_context!()` reads the frontend dist at compile time via proc macro. Cargo only recompiles when `.rs` files change — changes to `build/` are invisible. To force re-embed:

```bash
touch src/main.rs
cargo build --features custom-protocol
```

### Killing Processes Before Rebuild on Windows

After a background-run Tauri app exits, the process may linger and lock the binary. `taskkill /f /im opendeck.exe` may fail (git bash argument parsing). Use PowerShell:

```powershell
Get-Process opendeck | Stop-Process -Force
```

This includes child plugin processes (`opendeck-starterpack*`, `*-win.exe`).

### Creating Windows Installers

Tauri supports two installer formats on Windows:

| Format | Tool | Requirement |
|--------|------|-------------|
| MSI (`.msi`) | WiX Toolset v3 | Auto-downloaded by tauri CLI (39 MB zip), network-dependent |
| NSIS (`.exe`) | NSIS | Must be installed separately, **requires admin rights** |

In environments without admin privilege or reliable internet, skip the bundling step and use the portable exe:

```bash
npx tauri build --no-bundle
```

The release exe is at `src-tauri/target/release/opendeck.exe`.

### Whitespace in `build.rs`

`build.rs` uses tab indentation and CRLF line endings. The `Edit` tool's `old_string` matching is sensitive to these — use `Write` to rewrite the whole file if edits fail.

### `beforeBuildCommand` vs Deno

`tauri.conf.json` defaults to `beforeBuildCommand: "deno task build"`. If deno isn't installed, change it to the equivalent npm command:

```bash
beforeBuildCommand: "npm run build"
```

Don't forget to revert this change before committing (see `#pre-commit-requirements` above).

## Bug Fix Log

See [`docs/bug-fixes.md`](docs/bug-fixes.md) for a running log of bugs found and fixed during development, including root causes, fixes, and lessons learned. **Before fixing any bug, check that file first** — the same issue (or a similar pattern) may have been encountered before.

When a bug fix is confirmed working, add a new entry to `docs/bug-fixes.md` with: symptom, root cause, fix, and lesson learned.
