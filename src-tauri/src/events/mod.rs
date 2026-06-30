pub mod frontend;
pub mod inbound;
pub mod outbound;

use inbound::RegisterEvent;

use std::collections::HashMap;
use std::sync::LazyLock;

use futures::{SinkExt, StreamExt, stream::SplitSink};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

struct ConnEntry {
    socket: SplitSink<WebSocketStream<TcpStream>, Message>,
    generation: u64,
}

type Sockets = LazyLock<Mutex<HashMap<String, ConnEntry>>>;
static PLUGIN_SOCKETS: Sockets = LazyLock::new(|| Mutex::new(HashMap::new()));
static PROPERTY_INSPECTOR_SOCKETS: Sockets = LazyLock::new(|| Mutex::new(HashMap::new()));
static PLUGIN_QUEUES: LazyLock<RwLock<HashMap<String, Vec<Message>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static PROPERTY_INSPECTOR_QUEUES: LazyLock<RwLock<HashMap<String, Vec<Message>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub async fn registered_plugins() -> Vec<String> {
	PLUGIN_SOCKETS.lock().await.keys().map(|x| x.to_owned()).collect()
}

/// Register a plugin or property inspector to send and receive events with its WebSocket.
pub async fn register_plugin(event: RegisterEvent, stream: WebSocketStream<TcpStream>) {
	let (mut read, write) = stream.split();
	match event {
		RegisterEvent::RegisterPlugin { uuid } => {
			log::debug!("Registered plugin {}", uuid);
			if let Some(queue) = PLUGIN_QUEUES.read().await.get(&uuid) {
				for message in queue {
					let _ = read.feed(message.clone()).await;
				}
				let _ = read.flush().await;
			}
			let generation = {
				let mut sockets = PLUGIN_SOCKETS.lock().await;
				let e = sockets.entry(uuid.clone()).or_insert_with(|| ConnEntry {
					socket: read,
					generation: 0,
				});
				e.generation += 1;
				e.generation
			};
			tokio::spawn(async move {
				write.for_each(|event| inbound::process_incoming_message(event, &uuid, false)).await;
				// Only clean up if no newer connection has replaced ours
				let mut sockets = PLUGIN_SOCKETS.lock().await;
				if sockets.get(&uuid).is_some_and(|e| e.generation == generation) {
					sockets.remove(&uuid);
				}
			});
		}
		RegisterEvent::RegisterPropertyInspector { uuid } => {
			if let Some(queue) = PROPERTY_INSPECTOR_QUEUES.read().await.get(&uuid) {
				for message in queue {
					let _ = read.feed(message.clone()).await;
				}
				let _ = read.flush().await;
			}
			let generation = {
				let mut sockets = PROPERTY_INSPECTOR_SOCKETS.lock().await;
				let e = sockets.entry(uuid.clone()).or_insert_with(|| ConnEntry {
					socket: read,
					generation: 0,
				});
				e.generation += 1;
				e.generation
			};
			tokio::spawn(async move {
				write.for_each(|event| inbound::process_incoming_message_pi(event, &uuid)).await;
				// Only clean up if no newer connection has replaced ours
				let mut sockets = PROPERTY_INSPECTOR_SOCKETS.lock().await;
				if sockets.get(&uuid).is_some_and(|e| e.generation == generation) {
					sockets.remove(&uuid);
				}
			});
		}
	};
}
