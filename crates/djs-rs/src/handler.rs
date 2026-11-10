//! Type-erased storage of event handlers.

use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

use crate::{
    Context,
    event::{Dispatch, Event, EventKind},
};

/// The error type event handlers may return. Any error type converts into it with `?`.
pub type HandlerError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// A registered handler with its event type erased.
pub(crate) trait ErasedHandler: Send + Sync {
    /// Spawns the handler on the tokio runtime if `dispatch` is its event. The handler future is
    /// spawned directly (no extra boxing).
    fn spawn(&self, ctx: &Context, dispatch: &Dispatch);

    /// Returns the handler future for sequential execution, if `dispatch` is its event.
    fn call(&self, ctx: &Context, dispatch: &Dispatch) -> Option<BoxFuture>;
}

pub(crate) struct Handler<E, F> {
    handler: F,
    event: PhantomData<fn() -> E>,
}

impl<E, F> Handler<E, F> {
    pub(crate) const fn new(handler: F) -> Self {
        Self {
            handler,
            event: PhantomData,
        }
    }
}

/// Runs a handler future and reports its error, if any.
async fn run<E: Event>(future: impl Future<Output = Result<(), HandlerError>>) {
    if let Err(error) = future.await {
        tracing::error!(event = E::NAME, %error, "event handler returned an error");
    }
}

impl<E, F, Fut> ErasedHandler for Handler<E, F>
where
    E: Event,
    F: Fn(Context, Arc<E::Payload>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
{
    fn spawn(&self, ctx: &Context, dispatch: &Dispatch) {
        if let Some(payload) = E::payload(dispatch) {
            tokio::spawn(run::<E>((self.handler)(ctx.clone(), Arc::clone(payload))));
        }
    }

    fn call(&self, ctx: &Context, dispatch: &Dispatch) -> Option<BoxFuture> {
        let payload = E::payload(dispatch)?;
        Some(Box::pin(run::<E>((self.handler)(
            ctx.clone(),
            Arc::clone(payload),
        ))))
    }
}

/// Handlers per event kind. Replaced as a whole (copy-on-write) when a handler is added, so
/// dispatching only needs to clone an `Arc`.
#[derive(Clone, Default)]
pub(crate) struct Registry {
    handlers: [Vec<Arc<dyn ErasedHandler>>; EventKind::COUNT],
}

impl Registry {
    pub(crate) fn with(&self, kind: EventKind, handler: Arc<dyn ErasedHandler>) -> Self {
        let mut registry = self.clone();
        registry.handlers[kind as usize].push(handler);
        registry
    }

    pub(crate) fn get(&self, kind: EventKind) -> &[Arc<dyn ErasedHandler>] {
        &self.handlers[kind as usize]
    }
}
