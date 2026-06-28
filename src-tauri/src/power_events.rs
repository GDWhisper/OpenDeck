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

	let _ = crate::events::outbound::misc::system_did_wake_up().await;

	tokio::spawn(async {
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;

		#[cfg(windows)]
		{
			let app = match crate::APP_HANDLE.get() {
				Some(app) => app,
				None => return,
			};

			// After system sleep, both webview plugins (e.g. clock displays)
			// and native plugins (Rust binaries) lose their WebSocket connections.
			// Deactivate and re-initialise all plugins so they start fresh.
			crate::plugins::deactivate_plugins().await;

			let plugins_dir = crate::shared::config_dir().join("plugins");
			let spawner_tx =
				(*app.state::<std::sync::mpsc::Sender<crate::plugins::SpawnRequest>>()).clone();

			if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
				for entry in entries.flatten() {
					let path = entry.path();
					if path.is_dir() {
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
	});

	REINIT_IN_PROGRESS.store(false, Ordering::SeqCst);

	Ok(())
}
