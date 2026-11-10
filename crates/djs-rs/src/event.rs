//! Typed client events.
//!
//! Every event is a zero-sized value on [`Events`] — `Events::MessageCreate`,
//! `Events::ClientReady`, … — whose type determines the payload a handler receives:
//!
//! ```no_run
//! # use djs_rs::prelude::*;
//! # let client = Client::builder("token").build();
//! client.on(Events::MessageCreate, |_ctx, message| async move {
//!     // `message` is an `Arc<MessageCreate>`, which dereferences to `Message`.
//!     println!("{}", message.content);
//!     Ok(())
//! });
//! ```
//!
//! discord.js counterpart: the `Events` enum.

use std::sync::Arc;

use djs_model::gateway::{
    ShardId,
    payload::{MessageCreate as MessageCreatePayload, Ready},
};

/// The events a [`Client`](crate::Client) can emit, as typed values.
///
/// Pass one of these to [`Client::on`](crate::Client::on). The naming follows discord.js
/// (`Events.MessageCreate`, `Events.ClientReady`, …).
///
/// **Coverage:** Phase 1 provides the events of the vertical slice; the full discord.js event
/// set arrives in Phase 8.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Events;

#[expect(
    non_upper_case_globals,
    reason = "event names mirror discord.js' `Events.MessageCreate` style"
)]
impl Events {
    /// The client's session is ready (`READY`). Fired again after every full reconnect that
    /// re-identifies (but not after resumes).
    ///
    /// Payload: [`Ready`]. discord.js counterpart: `Events.ClientReady`.
    pub const ClientReady: ClientReady = ClientReady;

    /// A message was sent (`MESSAGE_CREATE`).
    ///
    /// Receiving guild message content requires the privileged
    /// [`MESSAGE_CONTENT`](djs_model::gateway::Intents::MESSAGE_CONTENT) intent.
    ///
    /// Payload: [`MessageCreate`](djs_model::gateway::payload::MessageCreate) (dereferences to
    /// [`Message`](djs_model::message::Message)). discord.js counterpart:
    /// `Events.MessageCreate`.
    pub const MessageCreate: MessageCreate = MessageCreate;
}

/// The type of [`Events::ClientReady`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ClientReady;

/// The type of [`Events::MessageCreate`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct MessageCreate;

/// A kind of event that handlers can be registered for. Implemented by the types of the
/// [`Events`] constants; it cannot be implemented outside this crate.
pub trait Event: Copy + Send + Sync + 'static + sealed::Sealed {
    /// The payload handlers of this event receive (wrapped in an [`Arc`]).
    type Payload: Send + Sync + 'static;

    /// The event name used in logs, e.g. `"messageCreate"`.
    const NAME: &'static str;

    #[doc(hidden)]
    const KIND: EventKind;

    #[doc(hidden)]
    fn payload(dispatch: &Dispatch) -> Option<&Arc<Self::Payload>>;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ClientReady {}
    impl Sealed for super::MessageCreate {}
}

impl Event for ClientReady {
    type Payload = Ready;
    const NAME: &'static str = "clientReady";
    const KIND: EventKind = EventKind::ClientReady;

    fn payload(dispatch: &Dispatch) -> Option<&Arc<Ready>> {
        match dispatch {
            Dispatch::Ready(ready) => Some(ready),
            _ => None,
        }
    }
}

impl Event for MessageCreate {
    type Payload = MessageCreatePayload;
    const NAME: &'static str = "messageCreate";
    const KIND: EventKind = EventKind::MessageCreate;

    fn payload(dispatch: &Dispatch) -> Option<&Arc<MessageCreatePayload>> {
        match dispatch {
            Dispatch::MessageCreate(message) => Some(message),
            _ => None,
        }
    }
}

/// Internal index of an event kind in the handler registry.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    ClientReady = 0,
    MessageCreate = 1,
}

impl EventKind {
    pub(crate) const COUNT: usize = 2;
}

/// A dispatched event with its payload shared behind an [`Arc`], as delivered to handlers and
/// to [`EventStream`](crate::EventStream) subscribers.
///
/// Cloning is cheap.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Dispatch {
    /// `READY`.
    Ready(Arc<Ready>),
    /// `MESSAGE_CREATE`.
    MessageCreate(Arc<MessageCreatePayload>),
    /// `RESUMED`.
    Resumed,
    /// An event this version of the library does not decode yet, with its name.
    Unknown(Arc<str>),
}

impl Dispatch {
    /// Returns the Discord event name, e.g. `"MESSAGE_CREATE"`.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Ready(_) => "READY",
            Self::MessageCreate(_) => "MESSAGE_CREATE",
            Self::Resumed => "RESUMED",
            Self::Unknown(name) => name,
        }
    }

    pub(crate) const fn kind(&self) -> Option<EventKind> {
        match self {
            Self::Ready(_) => Some(EventKind::ClientReady),
            Self::MessageCreate(_) => Some(EventKind::MessageCreate),
            Self::Resumed | Self::Unknown(_) => None,
        }
    }
}

/// An event received from a shard, as yielded by [`EventStream`](crate::EventStream).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ShardEvent {
    /// The shard that received the event.
    pub shard: ShardId,
    /// The event.
    pub event: Dispatch,
}
