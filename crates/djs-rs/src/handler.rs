//! Type-erased storage of event handlers.

use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

use djs_cache::Backend;

use crate::{
    Context,
    event::{Dispatch, Event, EventKind},
};

/// The error type event handlers may return. Any error type converts into it with `?`.
pub type HandlerError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// A registered handler with its event type erased.
pub(crate) trait ErasedHandler<B: Backend>: Send + Sync {
    /// Spawns the handler on the tokio runtime if `dispatch` is its event. The handler future is
    /// spawned directly (no extra boxing).
    fn spawn(&self, ctx: &Context<B>, dispatch: &Dispatch);

    /// Returns the handler future for sequential execution, if `dispatch` is its event.
    fn call(&self, ctx: &Context<B>, dispatch: &Dispatch) -> Option<BoxFuture>;
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

impl<B, E, F, Fut> ErasedHandler<B> for Handler<E, F>
where
    B: Backend,
    E: Event,
    F: Fn(Context<B>, Arc<E::Payload>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
{
    fn spawn(&self, ctx: &Context<B>, dispatch: &Dispatch) {
        if let Some(payload) = E::payload(dispatch) {
            tokio::spawn(run::<E>((self.handler)(ctx.clone(), Arc::clone(payload))));
        }
    }

    fn call(&self, ctx: &Context<B>, dispatch: &Dispatch) -> Option<BoxFuture> {
        let payload = E::payload(dispatch)?;
        Some(Box::pin(run::<E>((self.handler)(
            ctx.clone(),
            Arc::clone(payload),
        ))))
    }
}

/// Handlers per event kind. Replaced as a whole (copy-on-write) when a handler is added, so
/// dispatching only needs to clone an `Arc`.
pub(crate) struct Registry<B: Backend> {
    handlers: [Vec<Arc<dyn ErasedHandler<B>>>; EventKind::COUNT],
}

impl<B: Backend> Default for Registry<B> {
    fn default() -> Self {
        Self {
            handlers: core::array::from_fn(|_| Vec::new()),
        }
    }
}

impl<B: Backend> Clone for Registry<B> {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
        }
    }
}

impl<B: Backend> Registry<B> {
    pub(crate) fn with(&self, kind: EventKind, handler: Arc<dyn ErasedHandler<B>>) -> Self {
        let mut registry = self.clone();
        registry.handlers[kind as usize].push(handler);
        registry
    }

    pub(crate) fn get(&self, kind: EventKind) -> &[Arc<dyn ErasedHandler<B>>] {
        &self.handlers[kind as usize]
    }
}
