//! Exports for `C` library

use async_std::{future::poll_fn, task::Poll};
use futures::{channel::mpsc::Sender, stream::StreamExt};
use once_cell::sync::OnceCell;
use std::convert::TryInto;
use std::sync::Mutex;

use primitives::{
    ffi::{ByteArray, KeyPair},
    LogLevel, PlainTextMessage,
};

static EVENTS_SENDER: OnceCell<Mutex<Sender<IncomingEvent>>> = OnceCell::new();
const CHANNEL_BUFFER_SIZE: usize = 10;

enum IncomingEvent {
    Message(PlainTextMessage),
}

/// Starts the networking process in the background.
/// ## Arguments
///
///
/// `secret_array` - a Sec256k1 private key bytes
///
/// `name` - your name as seen to other peers
///
/// `callback` - triggered on any event with bytes representing
/// serialized json event (`primitives::PeerEvent`).
///
/// `enable_logs` - enables or disables logs
///
/// `log_level` - the level of the log
#[no_mangle]
pub extern "C" fn start_network(
    secret_array: ByteArray,
    name: ByteArray,
    callback: extern "C" fn(ByteArray),
    enable_logs: bool,
    log_level: LogLevel,
) -> bool {
    let name: Result<String, _> = name.try_into();
    let name = match name {
        Ok(name) => name,
        Err(e) => {
            log::error!("Error parsing peer name: {}", e);
            return false;
        }
    };
    let secret_bytes: Vec<u8> = secret_array.into();
    let secret = match libp2p::identity::secp256k1::SecretKey::from_bytes(secret_bytes) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Error: {}", e);
            return false;
        }
    };
    if enable_logs {
        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .init();
    }
    log::debug!("Starting network layer");
    let (mut swarm, mut out_events) = match crate::create_swarm(secret, name) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Error creating swarm: {}", e);
            return false;
        }
    };
    let (in_events_tx, mut in_events_rx) = futures::channel::mpsc::channel(CHANNEL_BUFFER_SIZE);
    if let Err(_e) = EVENTS_SENDER.set(Mutex::new(in_events_tx)) {
        log::error!("Error setting global in_events_tx");
    }

    async_std::task::spawn(poll_fn(move |cx| {
        loop {
            match out_events.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => {
                    match event.try_into() {
                        Ok(bytes) => callback(bytes),
                        Err(e) => log::error!("Error serializing out event: {}", e),
                    };
                }
                _ => break,
            }
        }
        loop {
            match in_events_rx.poll_next_unpin(cx) {
                Poll::Ready(Some(IncomingEvent::Message(message))) => {
                    if let Err(e) = swarm.private_chat.send_message(message) {
                        log::error!("Error sending message: {}", e);
                    };
                }
                _ => break,
            }
        }
        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(None) => {
                    log::error!("Swarm is finished");
                    return Poll::Ready(());
                }
                _ => break,
            }
        }
        Poll::Pending
    }));

    true
}

/// Free allocated ByteArray memory. This needs to be called e.g. after start function for `secret_array`
/// if you're using the library from C.
#[no_mangle]
pub extern "C" fn free_array(array: ByteArray) {
    unsafe {
        array.free();
    }
}

/// Send a message to peer.
///
/// ## Arguments
///
/// `to_peer_id` - base58 Libp2p peer_id. This one is taken from discovery events from `start_network`.
///
/// `message` - utf8 text content of the message
///
/// `timestamp` - unix timestamp, essentially an id of the message
#[no_mangle]
pub extern "C" fn send_message(to_peer_id: ByteArray, message: ByteArray, timestamp: u64) -> bool {
    if let Some(sender_mutex) = EVENTS_SENDER.get() {
        if let Ok(mut sender) = sender_mutex.lock() {
            let to = match to_peer_id.try_into() {
                Ok(to) => to,
                Err(e) => {
                    log::error!("Error converting `peer_id` bytearray: {}", e);
                    return false;
                }
            };
            let text = match message.try_into() {
                Ok(text) => text,
                Err(e) => {
                    log::error!("Error converting `message` bytearray: {}", e);
                    return false;
                }
            };
            let message = PlainTextMessage {
                to,
                timestamp,
                text,
            };
            if let Err(e) = sender.try_send(IncomingEvent::Message(message)) {
                log::error!("Error sending message to tx: {}", e);
            }
            return true;
        }
    }
    log::error!("Couldn't extract swarm from static cell");
    false
}

/// Generate secret keypair (to derive PeerId, i.e. p2p identity)
#[no_mangle]
pub extern "C" fn generate_keypair() -> KeyPair {
    let (secret, peer_id) = super::utils::generate_secret();
    let secret_bytes = secret.to_bytes().to_vec();
    let peer_id_bytes = peer_id.into_bytes();
    KeyPair {
        secret: secret_bytes.into(),
        peer_id: peer_id_bytes.into(),
    }
}
