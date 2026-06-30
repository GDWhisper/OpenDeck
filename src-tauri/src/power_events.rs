use std::sync::atomic::{AtomicBool, Ordering};

use psp::monitor::{PowerMonitor, PowerState};
use tauri::Manager;

static REINIT_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

pub fn init_power_events() {
	let power_monitor = Box::leak(Box::new(PowerMonitor::new()));
	let receiver = power_monitor.event_receiver();
	if let Err(error) = power_monitor.start_listening() {
		log::error!("Failed to start listening for power events: {error}");
		return;
	}

	std::thread::spawn(move || {
		while let Ok(event) = receiver.recv() {
			match event {
				PowerState::ScreenLocked => {
					tauri::async_runtime::spawn(async {
						if let Err(error) = crate::device_sleep::sleep_for_computer_lock().await {
							log::error!("Failed to sleep devices due to screen lock: {error}");
						}
					});
				}
				PowerState::ScreenUnlocked => {
					tauri::async_runtime::spawn(async {
						if let Err(error) = crate::device_sleep::wake_from_computer_lock().await {
							log::error!("Failed to wake devices due to screen unlock: {error}");
						}
					});
				}
				PowerState::Resume => {
					tauri::async_runtime::spawn(async {
						if let Err(error) = handle_wake().await {
							log::error!("Failed to handle system wake: {error}");
						}
					});
				}
				PowerState::Suspend | PowerState::Shutdown | PowerState::Unknown => {}
			}
		}
	});
}

async fn handle_wake() -> Result<(), anyhow::Error> {
	if REINIT_IN_PROGRESS.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
		log::info!("Re-init already in progress, skipping duplicate wake handling");
		return Ok(());
	}

	log::info!("System wake detected via psp, reinitialising devices");

	crate::elgato::invalidate_hidapi().await;
	tokio::time::sleep(std::time::Duration::from_secs(2)).await;
	crate::elgato::initialise_devices().await;

	for device in crate::shared::DEVICES.iter() {
		let _ = crate::device_sleep::note_activity(&device.id).await;
	}


	tokio::spawn(async {
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;

		#[cfg(windows)]
		{
			// After system sleep, plugin WebSocket connections are dead.
			//
			// Webview plugins: reload the page and re-eval the connection JS.
			// We do NOT close the window because Tauri may return the stale
			// window when creating a new one with the same label.  Reloading
			// gives a fresh JS environment while keeping the window handle.
			crate::plugins::reload_webview_plugins().await;

			// Native plugins: kill the old process and spawn a fresh one.
			use crate::plugins::PluginInstance;
			let app = match crate::APP_HANDLE.get() {
				Some(app) => app,
				None => return,
			};
			let spawner_tx =
				(*app.state::<std::sync::mpsc::Sender<crate::plugins::SpawnRequest>>()).clone();

			let native_uuids: Vec<String> = {
				crate::plugins::INSTANCES
					.lock()
					.await
					.iter()
					.filter(|(_, inst)| !matches!(inst, PluginInstance::Webview))
					.map(|(uuid, _)| uuid.clone())
					.collect()
			};

			for uuid in &native_uuids {
				log::info!("Deactivating native plugin after wake: {}", uuid);
				let _ = crate::plugins::deactivate_plugin(app, uuid).await;
			}

			let plugins_dir = crate::shared::config_dir().join("plugins");
			if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
				for entry in entries.flatten() {
					let path = entry.path();
					if !path.is_dir() {
						continue;
					}
					let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
					if native_uuids.contains(&dir_name.to_owned()) {
						let tx = spawner_tx.clone();
						if let Err(e) = crate::plugins::initialise_plugin(path, tx).await {
							log::error!("Failed to re-init plugin after wake: {:#}", e);
						}
					}
				}
			}
		}

		#[cfg(not(windows))]
		crate::plugins::reload_webview_plugins().await;
		// Send after plugin recovery so the event hits live connections.
		let _ = crate::events::outbound::misc::system_did_wake_up().await;
	});

	REINIT_IN_PROGRESS.store(false, Ordering::SeqCst);

	Ok(())
}
