use futures::prelude::*;
use futures_timer::Delay;
use libp2p::core::upgrade;
use libp2p::swarm::{
    KeepAlive, NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerEvent,
    ProtocolsHandlerUpgrErr, SubstreamProtocol,
};
use primitives::{ErrorMessage, Event, PlainTextMessage, Timestamp};

use super::protocol::{HandshakeMetadata, PrivateChatProtocol};
use crate::error::{Error, Result};
use futures_codec::{Framed, LengthCodec};
use std::{
    collections::{HashMap, VecDeque},
    error::Error as StdError,
    time::Duration,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

const INITIAL_RETRY: Duration = Duration::from_secs(1);
const RETRY_EXP: u32 = 2;
const MAX_RETRY: Duration = Duration::from_secs(120);

pub struct PrivateChatHandler {
    local_metadata: HandshakeMetadata,
    stream: Option<Framed<NegotiatedSubstream, LengthCodec>>,
    pending_metadata: Option<HandshakeMetadata>,
    pending_sending_messages: VecDeque<PlainTextMessage>,
    outgoing_messages:
        HashMap<u64, Pin<Box<dyn Future<Output = std::result::Result<(), std::io::Error>> + Send>>>,
    errors: VecDeque<ErrorMessage>,
    retry: Option<Delay>,
    retry_value: Duration,
}

impl ProtocolsHandler for PrivateChatHandler {
    type InEvent = ();
    type OutEvent = Event;
    type Error = Error;
    type InboundProtocol = PrivateChatProtocol;
    type OutboundProtocol = PrivateChatProtocol;
    type OutboundOpenInfo = ();
    type InboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<PrivateChatProtocol, ()> {
        SubstreamProtocol::new(PrivateChatProtocol::new(self.local_metadata), ())
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: (HandshakeMetadata, NegotiatedSubstream),
        _: (),
    ) {
        let (metadata, stream) = protocol;
        self.stream = Some(Framed::new(stream, LengthCodec {}));
        self.pending_metadata = Some(metadata);
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        protocol: (HandshakeMetadata, NegotiatedSubstream),
        _: (),
    ) {
        let (metadata, stream) = protocol;
        self.stream = Some(Framed::new(stream, LengthCodec {}));
        self.pending_metadata = Some(metadata);
    }

    fn inject_event(&mut self, _: ()) {}

    fn inject_dial_upgrade_error(&mut self, _info: (), error: ProtocolsHandlerUpgrErr<Error>) {
        log::error!("Error upgrading connection: {}", error);
        self.stream = None;
        self.errors.push_front(ErrorMessage::Unreachable);
        self.retry = Some(Delay::new(self.retry_value));
        self.retry_value *= RETRY_EXP;
        // Stop trying
        if self.retry_value > MAX_RETRY {
            self.retry_value = INITIAL_RETRY;
            self.retry = None;
        }
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ProtocolsHandlerEvent<PrivateChatProtocol, (), Self::OutEvent, Self::Error>> {
        if let Some(error) = self.errors.pop_back() {
            return Poll::Ready(ProtocolsHandlerEvent::Custom(Event::Error(
                ErrorMessage::Unreachable,
            )));
        }
        if let Some(stream) = self.stream {
            let timestamps = self.outgoing_messages.keys().collect::<Vec<_>>();
            for &timestamp in timestamps {
                if let Some(future) = self.outgoing_messages.get(&timestamp) {
                    match future.poll_unpin(cx) {
                        Poll::Pending => (),
                        Poll::Ready(_) => {
                            self.outgoing_messages.remove(&timestamp);
                            log::debug!("Message with timestamps `{}` sent", timestamp);
                            return Poll::Ready(ProtocolsHandlerEvent::Custom(
                                Event::SentPlainTextMessage(Timestamp { timestamp }),
                            ));
                        }
                    }
                }
            }
            for message in self.pending_sending_messages.drain(..) {
                let bytes: Vec<u8> = serde_json::to_vec(&message).expect("Infallible; qed");
                log::debug!("Sending message with timestamp `{}`", message.timestamp);
                let future = stream.send(bytes.into());
                self.outgoing_messages
                    .insert(message.timestamp, future.boxed());
            }
            match stream.poll_next_unpin(cx) {
                Poll::Pending => (),
                Poll::Ready(Some(Ok(bytes))) => {
                    match serde_json::from_slice::<PlainTextMessage>(&bytes) {
                        Ok(message) => {
                            log::debug!("Received message: {:?}", message);
                            return Poll::Ready(ProtocolsHandlerEvent::Custom(
                                Event::ReceivedPlainTextMessage(message),
                            ));
                        }
                        Err(e) => {
                            log::error!("Error parsing bytes to json: {:?}", bytes);
                            return Poll::Ready(ProtocolsHandlerEvent::Custom(Event::Error(
                                ErrorMessage::Parse,
                            )));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    log::error!("Network error: {:?}", e);
                    return Poll::Ready(ProtocolsHandlerEvent::Custom(Event::Error(
                        ErrorMessage::Network,
                    )));
                }
                Poll::Ready(None) => {
                    log::debug!("Stream is closed");
                }
            }
        }
        Poll::Pending
    }
}

impl PrivateChatHandler {
    pub fn new(local_metadata: HandshakeMetadata) -> PrivateChatHandler {
        PrivateChatHandler {
            local_metadata,
            pending_metadata: None,
            pending_sending_messages: VecDeque::new(),
            outgoing_messages: HashMap::new(),
            stream: None,
            errors: VecDeque::new(),
            retry: None,
            retry_value: INITIAL_RETRY,
        }
    }

    pub fn send(&self, message: PlainTextMessage) {
        self.pending_sending_messages.push_back(message)
    }
}

enum State {
    Initialized,
    Open,
}

#[derive(Debug)]
enum PrivateChatError {
    Finished,
    Other(Box<dyn StdError + Send + 'static>),
}

impl std::fmt::Display for PrivateChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivateChatError::Finished => write!(f, "PrivateChat error: Finished")?,
            PrivateChatError::Other(e) => write!(f, "PrivateChat error: {}", e)?,
        }
        Ok(())
    }
}

impl StdError for PrivateChatError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            PrivateChatError::Finished => None,
            PrivateChatError::Other(e) => Some(&**e),
        }
    }
}
