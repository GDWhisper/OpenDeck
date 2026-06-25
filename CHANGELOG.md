# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## [2.12.2] - 2026-06-22

### Changes

- Replace wall-clock polling sleep detection with [psp](https://github.com/pewsheen/psp) crate for native OS power event callbacks

### Bug Fixes

- **2026-06-22**: Fix Stream Deck device lost after system sleep — invalidate cached `HidApi` instance on wake so `initialise_devices()` gets fresh USB handles; add `AtomicBool` guard to prevent concurrent re-init on double-wake; add error logging to device reader loop
- **2026-06-21**: Fix tray "Restart" not relaunching the application — replace `app.restart()` with `delayed_restart()` that uses `ping -n 4` delay + `start /B` to avoid `tauri-plugin-single-instance` named mutex race (`timeout.exe` silently fails under `CREATE_NO_WINDOW`)
- **2026-06-21**: Fix `tokio::spawn(elgato::reset_devices())` in `RunEvent::Exit` handler never executing — replace with `futures::executor::block_on()` since the tokio runtime is already shutting down
- Add plugin child process watchdog to auto-restart plugins that exit unexpectedly (e.g. after system sleep/resume)
