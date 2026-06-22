use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

/// Guard to prevent concurrent re-init when multiple wake events fire in
/// quick succession (e.g. a short sleep followed by an immediate second wake).
static REINIT_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Initialise the system sleep/wake watchdog.
///
/// This spawns a background task that monitors system wall-clock time to detect
/// when the system has been in sleep or hibernation. Upon detecting a wake event,
/// it waits for the USB subsystem to stabilise, then reinitialises connected
/// devices and wakes any that were put to sleep before the system slept.
pub fn init_watchdog() {
	tokio::spawn(async {
		// Poll at 1 Hz — cheap and gives us sub-second responsiveness on wake.
		let poll_interval = Duration::from_secs(1);
		// Anything above 3 s of wall-clock time between consecutive polls means
		// the system was almost certainly in sleep/hibernation.  This is well
		// above the 1 s poll interval even with moderate scheduler noise.
		let sleep_threshold = Duration::from_secs(3);
		// Give the USB subsystem time to re-enumerate devices after resume.
		let stabilization_delay = Duration::from_secs(2);

		let mut last_check = SystemTime::now();

		loop {
			tokio::time::sleep(poll_interval).await;

			let now = SystemTime::now();
			// `unwrap_or(Duration::ZERO)` gracefully handles system clock
			// adjustments (e.g. NTP) that cause `now` to precede `last_check`.
			let elapsed = now.duration_since(last_check).unwrap_or(Duration::ZERO);

			if elapsed > sleep_threshold {
				log::info!("System sleep/wake detected (elapsed: {:?}), reinitialising devices", elapsed,);

				// Prevent concurrent re-init if another wake fires while we
				// are still handling this one (double-wake race).
				if REINIT_IN_PROGRESS.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
					log::info!("Re-init already in progress, skipping duplicate wake handling");
					last_check = SystemTime::now();
					continue;
				}

				// Invalidate the cached HidApi instance — USB handles are
				// stale after sleep.  The next call to initialise_devices()
				// will create a fresh HidApi with current device handles.
				crate::elgato::invalidate_hidapi().await;

				// Wait for USB to stabilise after resume so that devices are
				// properly re-enumerated before we try to connect to them.
				tokio::time::sleep(stabilization_delay).await;

				// Re-enumerate Elgato / Stream Deck devices.  This is
				// idempotent — already-registered devices are skipped.
				crate::elgato::initialise_devices().await;

				// Wake any devices that were put to sleep before the system
				// slept (idle-timeout via device_sleep).
				for device in crate::shared::DEVICES.iter() {
					let _ = crate::device_sleep::note_activity(&device.id).await;
				}

				// Notify all plugins that the system has woken up, so they
				// can reinitialise their own devices if needed.
				let _ = crate::events::outbound::misc::system_did_wake_up().await;

				// Reload webview plugins whose WebSocket connections died
				// during sleep.  The Elgato/openaction SDK has no built-in
				// reconnection, so we close and recreate the webview window.
				// A short delay lets the WebSocket server and other subsystems
				// finish settling before we reconnect.
				tokio::spawn(async {
					tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					crate::plugins::reload_webview_plugins().await;
				});

				// Release the re-init guard.
				REINIT_IN_PROGRESS.store(false, Ordering::SeqCst);

				// Reset last_check after handling to prevent the next poll
				// from re-triggering wake detection due to time spent in
				// the stabilization delay and device reinitialisation.
				last_check = SystemTime::now();
			} else {
				last_check = now;
			}
		}
	});
}
