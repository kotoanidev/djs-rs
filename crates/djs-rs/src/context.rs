//! The [`Context`] passed to event handlers.

use std::sync::Arc;

use djs_gateway::MessageSender;
use djs_model::gateway::ShardId;
use djs_rest::Rest;

/// Everything an event handler needs to act: the REST client and the shard that received the
/// event.
///
/// Cheap to clone (a single reference count increment).
///
/// discord.js counterpart: the `client` reachable from every structure (`message.client`).
#[derive(Clone, Debug)]
pub struct Context {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    rest: Rest,
    shard: ShardId,
    sender: MessageSender,
}

impl Context {
    pub(crate) fn new(rest: Rest, shard: ShardId, sender: MessageSender) -> Self {
        Self {
            inner: Arc::new(Inner {
                rest,
                shard,
                sender,
            }),
        }
    }

    /// Returns the REST client.
    #[must_use]
    pub fn rest(&self) -> &Rest {
        &self.inner.rest
    }

    /// Returns the ID of the shard that received the event.
    #[must_use]
    pub fn shard_id(&self) -> ShardId {
        self.inner.shard
    }

    /// Returns a sender for gateway commands on the shard that received the event.
    #[must_use]
    pub fn shard(&self) -> &MessageSender {
        &self.inner.sender
    }
}
