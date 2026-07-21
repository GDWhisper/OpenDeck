# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Bug Fixes

- **2026-07-21**: Fix Stream Deck frozen after system sleep — `handle_wake()` now force re-enumerates devices via `reinitialise_devices()` instead of skipping re-connection of devices still present in `ELGATO_DEVICES` with a stale USB handle. A device epoch counter lets superseded reader loops exit without deregistering the freshly re-added device, so the device is recovered on every wake instead of being lost permanently. Also migrate profiles saved under a legacy device id (`99-<serial>-153`) to the current `sd-<serial>` scheme on startup so upgrades don't orphan existing configuration.

## [2.12.2] - 2026-06-22

### Bug Fixes

- **2026-06-22**: Fix Stream Deck device lost after system sleep — invalidate cached `HidApi` instance on wake so `initialise_devices()` gets fresh USB handles; add `AtomicBool` guard to prevent concurrent re-init on double-wake; add error logging to device reader loop
- **2026-06-21**: Fix tray "Restart" not relaunching the application — replace `app.restart()` with `delayed_restart()` that uses `ping -n 4` delay + `start /B` to avoid `tauri-plugin-single-instance` named mutex race (`timeout.exe` silently fails under `CREATE_NO_WINDOW`)
- **2026-06-21**: Fix `tokio::spawn(elgato::reset_devices())` in `RunEvent::Exit` handler never executing — replace with `futures::executor::block_on()` since the tokio runtime is already shutting down
- Add plugin child process watchdog to auto-restart plugins that exit unexpectedly (e.g. after system sleep/resume)
