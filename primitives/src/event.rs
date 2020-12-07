use crate::ffi::{ByteArray, Event as FFIEvent, EventTag};

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// Plain text message sent by peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlainTextMessage {
    pub from: String,
    pub text: String,
}

impl Into<ByteArray> for PlainTextMessage {
    fn into(self) -> ByteArray {
        serde_json::to_vec(&self)
            .expect("infallible conversion; qed")
            .into()
    }
}

/// A peer discovered or gone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDiscoveryMessage {
    pub peer_id: String,
}

impl Into<ByteArray> for PeerDiscoveryMessage {
    fn into(self) -> ByteArray {
        serde_json::to_vec(&self)
            .expect("infallible conversion; qed")
            .into()
    }
}

/// A metadata from peer message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataMessage {
    pub peer_id: String,
}

impl Into<ByteArray> for MetadataMessage {
    fn into(self) -> ByteArray {
        serde_json::to_vec(&self)
            .expect("infallible conversion; qed")
            .into()
    }
}

/// Event generated by network layer
#[derive(Debug, Clone)]
pub enum Event {
    /// Plain text message sent by peer
    PlainTextMessage(PlainTextMessage),
    /// Metadata message sent by peer
    Metadata(MetadataMessage),
    /// A new peer discovered
    PeerDiscovered(PeerDiscoveryMessage),
    /// A peer is gone
    PeerGone(PeerDiscoveryMessage),
}

impl TryFrom<FFIEvent> for Event {
    type Error = serde_json::error::Error;
    fn try_from(ev: FFIEvent) -> Result<Event, serde_json::error::Error> {
        let FFIEvent { tag, data } = ev;
        let data: Vec<u8> = data.into();
        let res = match tag {
            EventTag::PlainTextMessage => {
                let payload = serde_json::from_slice(&data)?;
                Event::PlainTextMessage(payload)
            }
            EventTag::PeerDiscovered => {
                let payload = serde_json::from_slice(&data)?;
                Event::PeerDiscovered(payload)
            }
            EventTag::PeerGone => {
                let payload = serde_json::from_slice(&data)?;
                Event::PeerGone(payload)
            }
            EventTag::Metadata => {
                let payload = serde_json::from_slice(&data)?;
                Event::Metadata(payload)
            }
        };
        Ok(res)
    }
}

impl Into<FFIEvent> for Event {
    fn into(self) -> FFIEvent {
        match self {
            Event::PlainTextMessage(data) => FFIEvent {
                tag: EventTag::PlainTextMessage,
                data: data.into(),
            },
            Event::PeerDiscovered(data) => FFIEvent {
                tag: EventTag::PeerDiscovered,
                data: data.into(),
            },
            Event::PeerGone(data) => FFIEvent {
                tag: EventTag::PeerGone,
                data: data.into(),
            },
            Event::Metadata(data) => FFIEvent {
                tag: EventTag::Metadata,
                data: data.into(),
            },
        }
    }
}
