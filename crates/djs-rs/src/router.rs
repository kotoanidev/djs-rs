//! The interaction router (D-251–D-255): handlers registered by command name or by component
//! and modal `custom_id`, next to [`Events::InteractionCreate`](crate::Events::InteractionCreate).
//!
//! | Registration | Interactions | Handler argument |
//! |---|---|---|
//! | [`Client::command`](crate::Client::command) | commands (slash and context menu) | [`CommandInteraction`] |
//! | [`Client::autocomplete`](crate::Client::autocomplete) | autocomplete of a command's options | [`AutocompleteInteraction`] |
//! | [`Client::component`](crate::Client::component) | buttons and select menus | [`ComponentInteraction`] |
//! | [`Client::modal`](crate::Client::modal) | modal submissions | [`ModalInteraction`] |
//!
//! - **Commands** are routed by their full name as Discord shows it (D-253): `"ping"`,
//!   `"mod ban"`, `"mod config set"`, or the name of a context menu command (`"Report
//!   Message"`). A route for `"mod"` (or `"mod config"`) catches the subcommands that have no
//!   route of their own. Slash and context menu commands sharing a name share the route; the
//!   handler can tell them apart by [`kind`](djs_model::interaction::CommandData::kind).
//! - **Components and modals** are routed by the part of their `custom_id` before the first
//!   `:` (D-254): the route `"vote"` receives `vote` and `vote:42`, and
//!   [`args`](ComponentInteraction::args) returns `42`.
//! - Routes only route (D-255): commands are registered with Discord through
//!   [`ctx.commands()`](crate::Context::commands), and
//!   [`Events::InteractionCreate`](crate::Events::InteractionCreate) is still emitted for every
//!   interaction, routed or not.
//!
//! Every wrapper holds the event's `Arc` (no copy), dereferences to the typed data and
//! implements [`InteractionExt`](crate::ext::InteractionExt), so `cmd.get_string("reason")`
//! and `cmd.reply(&ctx, …)` both work.
//!
//! discord.js has no router: its counterpart is a `switch` on `interaction.commandName` in the
//! `interactionCreate` listener.

use core::{fmt, ops::Deref};
use std::{collections::HashMap, future::Future, sync::Arc};

use djs_cache::Backend;
use djs_model::{
    gateway::payload::InteractionCreate,
    interaction::{CommandData, ComponentData, Interaction, InteractionData, ModalSubmitData},
};

use crate::{
    Context,
    handler::{BoxFuture, HandlerError},
};

/// Defines a wrapper over an interaction of one kind, dereferencing to its data.
macro_rules! wrapper {
    (
        $(#[$doc:meta])*
        $name:ident => $variant:ident($data:ty)
    ) => {
        $(#[$doc])*
        #[derive(Clone)]
        pub struct $name {
            event: Arc<InteractionCreate>,
        }

        impl $name {
            /// Wraps `event`, if it is an interaction of this kind.
            fn new(event: &Arc<InteractionCreate>) -> Option<Self> {
                matches!(event.data, InteractionData::$variant(_)).then(|| Self {
                    event: Arc::clone(event),
                })
            }

            /// Returns the whole interaction: its guild, channel, member, locale, …
            #[must_use]
            pub fn interaction(&self) -> &Interaction {
                &self.event
            }

            /// Returns the event payload, shared with the
            /// [`InteractionCreate`](crate::Events::InteractionCreate) handlers.
            #[must_use]
            pub const fn event(&self) -> &Arc<InteractionCreate> {
                &self.event
            }
        }

        impl Deref for $name {
            type Target = $data;

            fn deref(&self) -> &$data {
                match &self.event.data {
                    InteractionData::$variant(data) => data,
                    // `new` checked the kind, and the shared payload cannot change.
                    _ => unreachable!(concat!("a ", stringify!($name), " of another kind")),
                }
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // The interaction's `Debug` does not print its token.
                f.debug_tuple(stringify!($name)).field(self.interaction()).finish()
            }
        }
    };
}

wrapper! {
    /// A command interaction given to a [`Client::command`](crate::Client::command) route
    /// (D-252): dereferences to the [`CommandData`] (`get_string`, `subcommand`, …) and
    /// answers through [`InteractionExt`](crate::ext::InteractionExt) (`reply`,
    /// `defer_reply`, …).
    ///
    /// discord.js counterpart: `ChatInputCommandInteraction` and the context menu command
    /// interactions.
    CommandInteraction => ApplicationCommand(CommandData)
}

wrapper! {
    /// An autocomplete interaction given to a [`Client::autocomplete`](crate::Client::autocomplete)
    /// route (D-252): dereferences to the [`CommandData`] (`focused_value`, `get_string`, …)
    /// and answers with [`respond`](crate::ext::InteractionExt::respond).
    ///
    /// discord.js counterpart: `AutocompleteInteraction`.
    AutocompleteInteraction => Autocomplete(CommandData)
}

wrapper! {
    /// A component interaction given to a [`Client::component`](crate::Client::component)
    /// route (D-252): dereferences to the [`ComponentData`] (`custom_id`, `values`, `users`,
    /// …), with the [`args`](Self::args) of its custom ID, and answers through
    /// [`InteractionExt`](crate::ext::InteractionExt) (`update`, `reply`, `show_modal`, …).
    ///
    /// discord.js counterpart: `ButtonInteraction` and the select menu interactions.
    ComponentInteraction => MessageComponent(ComponentData)
}

wrapper! {
    /// A modal-submit interaction given to a [`Client::modal`](crate::Client::modal) route
    /// (D-252): dereferences to the [`ModalSubmitData`] (`text_value`, `selected_users`, …),
    /// with the [`args`](Self::args) of its custom ID, and answers through
    /// [`InteractionExt`](crate::ext::InteractionExt).
    ///
    /// discord.js counterpart: `ModalSubmitInteraction`.
    ModalInteraction => ModalSubmit(ModalSubmitData)
}

/// The part of a custom ID after the route name and its `:` (D-254), `""` if none.
fn args(custom_id: &str) -> &str {
    custom_id.split_once(':').map_or("", |(_, args)| args)
}

/// The route name of a custom ID: the part before the first `:`.
fn route_name(custom_id: &str) -> &str {
    custom_id
        .split_once(':')
        .map_or(custom_id, |(name, _)| name)
}

impl ComponentInteraction {
    /// Returns the part of the custom ID after the route name and its `:` (D-254): `"42"`
    /// for `vote:42`, `"a:b"` for `vote:a:b`, `""` for `vote`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use djs_rs::prelude::*;
    /// # let client = Client::builder("token").build();
    /// // A button with the custom ID `vote:42` votes for option 42.
    /// client.component("vote", |ctx, button| async move {
    ///     let option: u32 = button.args().parse()?;
    ///     button.reply(&ctx, format!("You voted for {option}")).await?;
    ///     Ok(())
    /// });
    /// ```
    #[must_use]
    pub fn args(&self) -> &str {
        args(&self.custom_id)
    }
}

impl ModalInteraction {
    /// Returns the part of the modal's custom ID after the route name and its `:` (D-254),
    /// `""` if none.
    #[must_use]
    pub fn args(&self) -> &str {
        args(&self.custom_id)
    }
}

/// A route's handler with its interaction type erased.
pub(crate) trait ErasedRoute<B: Backend, W>: Send + Sync {
    /// Spawns the handler on the tokio runtime (no extra boxing).
    fn spawn(&self, ctx: &Context<B>, interaction: W);

    /// Returns the handler future, for sequential execution.
    fn call(&self, ctx: &Context<B>, interaction: W) -> BoxFuture;
}

/// A registered route: its name (for error logs) and its handler.
pub(crate) struct Route<F> {
    name: Arc<str>,
    handler: F,
}

impl<F> Route<F> {
    pub(crate) fn new(name: &str, handler: F) -> Self {
        Self {
            name: name.into(),
            handler,
        }
    }
}

/// Runs a route's handler future and reports its error, if any.
async fn run(route: Arc<str>, future: impl Future<Output = Result<(), HandlerError>>) {
    if let Err(error) = future.await {
        tracing::error!(route = &*route, %error, "interaction route handler returned an error");
    }
}

impl<B, W, F, Fut> ErasedRoute<B, W> for Route<F>
where
    B: Backend,
    F: Fn(Context<B>, W) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), HandlerError>> + Send + 'static,
{
    fn spawn(&self, ctx: &Context<B>, interaction: W) {
        let future = (self.handler)(ctx.clone(), interaction);
        tokio::spawn(run(Arc::clone(&self.name), future));
    }

    fn call(&self, ctx: &Context<B>, interaction: W) -> BoxFuture {
        let future = (self.handler)(ctx.clone(), interaction);
        Box::pin(run(Arc::clone(&self.name), future))
    }
}

/// The routes of one interaction kind, by name.
type RouteMap<B, W> = HashMap<Box<str>, Arc<dyn ErasedRoute<B, W>>>;

/// Every route; part of the copy-on-write handler [`Registry`](crate::handler::Registry), so
/// dispatching needs no lock.
pub(crate) struct Routes<B: Backend> {
    pub(crate) commands: RouteMap<B, CommandInteraction>,
    pub(crate) autocomplete: RouteMap<B, AutocompleteInteraction>,
    pub(crate) components: RouteMap<B, ComponentInteraction>,
    pub(crate) modals: RouteMap<B, ModalInteraction>,
}

impl<B: Backend> Default for Routes<B> {
    fn default() -> Self {
        Self {
            commands: HashMap::new(),
            autocomplete: HashMap::new(),
            components: HashMap::new(),
            modals: HashMap::new(),
        }
    }
}

impl<B: Backend> Clone for Routes<B> {
    fn clone(&self) -> Self {
        Self {
            commands: self.commands.clone(),
            autocomplete: self.autocomplete.clone(),
            components: self.components.clone(),
            modals: self.modals.clone(),
        }
    }
}

/// The route an interaction goes to, with its wrapper.
enum Found<'a, B: Backend> {
    Command(
        &'a dyn ErasedRoute<B, CommandInteraction>,
        CommandInteraction,
    ),
    Autocomplete(
        &'a dyn ErasedRoute<B, AutocompleteInteraction>,
        AutocompleteInteraction,
    ),
    Component(
        &'a dyn ErasedRoute<B, ComponentInteraction>,
        ComponentInteraction,
    ),
    Modal(&'a dyn ErasedRoute<B, ModalInteraction>, ModalInteraction),
}

impl<B: Backend> Routes<B> {
    /// Whether no route is registered.
    pub(crate) fn is_empty(&self) -> bool {
        self.commands.is_empty()
            && self.autocomplete.is_empty()
            && self.components.is_empty()
            && self.modals.is_empty()
    }

    /// Spawns the handler of the route `event` goes to, if any.
    pub(crate) fn spawn(&self, ctx: &Context<B>, event: &Arc<InteractionCreate>) {
        match self.find(event) {
            Some(Found::Command(route, interaction)) => route.spawn(ctx, interaction),
            Some(Found::Autocomplete(route, interaction)) => route.spawn(ctx, interaction),
            Some(Found::Component(route, interaction)) => route.spawn(ctx, interaction),
            Some(Found::Modal(route, interaction)) => route.spawn(ctx, interaction),
            None => {}
        }
    }

    /// Returns the handler future of the route `event` goes to, if any.
    pub(crate) fn call(
        &self,
        ctx: &Context<B>,
        event: &Arc<InteractionCreate>,
    ) -> Option<BoxFuture> {
        Some(match self.find(event)? {
            Found::Command(route, interaction) => route.call(ctx, interaction),
            Found::Autocomplete(route, interaction) => route.call(ctx, interaction),
            Found::Component(route, interaction) => route.call(ctx, interaction),
            Found::Modal(route, interaction) => route.call(ctx, interaction),
        })
    }

    /// The route `event` goes to and its wrapper.
    fn find(&self, event: &Arc<InteractionCreate>) -> Option<Found<'_, B>> {
        Some(match &event.data {
            InteractionData::ApplicationCommand(data) => Found::Command(
                &**command_route(&self.commands, data)?,
                CommandInteraction::new(event)?,
            ),
            InteractionData::Autocomplete(data) => Found::Autocomplete(
                &**command_route(&self.autocomplete, data)?,
                AutocompleteInteraction::new(event)?,
            ),
            InteractionData::MessageComponent(data) => Found::Component(
                &**self.components.get(route_name(&data.custom_id))?,
                ComponentInteraction::new(event)?,
            ),
            InteractionData::ModalSubmit(data) => Found::Modal(
                &**self.modals.get(route_name(&data.custom_id))?,
                ModalInteraction::new(event)?,
            ),
            _ => return None,
        })
    }
}

/// The route of a command (D-253): its full path (`mod config set`), else the group
/// (`mod config`), else the command (`mod`).
fn command_route<'a, B: Backend, W>(
    routes: &'a RouteMap<B, W>,
    data: &CommandData,
) -> Option<&'a Arc<dyn ErasedRoute<B, W>>> {
    if routes.is_empty() {
        return None;
    }
    let mut path = [data.name.as_str(), "", ""];
    let mut len = 1;
    for part in [data.subcommand_group(), data.subcommand()]
        .into_iter()
        .flatten()
    {
        path[len] = part;
        len += 1;
    }
    (2..=len)
        .rev()
        .find_map(|len| joined(&path[..len], |key| routes.get(key)))
        .or_else(|| routes.get(path[0]))
}

/// Calls `f` with `parts` joined by spaces, built on the stack when it fits (command names
/// are at most 32 characters).
fn joined<R>(parts: &[&str], f: impl FnOnce(&str) -> R) -> R {
    let len = parts.iter().map(|part| part.len() + 1).sum::<usize>() - 1;
    let mut buffer = [0; 256];
    if len > buffer.len() {
        return f(&parts.join(" "));
    }
    let mut at = 0;
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            buffer[at] = b' ';
            at += 1;
        }
        buffer[at..at + part.len()].copy_from_slice(part.as_bytes());
        at += part.len();
    }
    // Strings joined by an ASCII space are valid UTF-8.
    f(core::str::from_utf8(&buffer[..at]).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_ids_are_split_at_the_first_colon() {
        assert_eq!((route_name("vote"), args("vote")), ("vote", ""));
        assert_eq!((route_name("vote:42"), args("vote:42")), ("vote", "42"));
        assert_eq!((route_name("vote:a:b"), args("vote:a:b")), ("vote", "a:b"));
        assert_eq!((route_name(":x"), args(":x")), ("", "x"));
    }

    #[test]
    fn paths_are_joined_on_the_stack_or_the_heap() {
        assert_eq!(
            joined(&["mod", "config", "set"], str::to_owned),
            "mod config set"
        );
        assert_eq!(joined(&["ping"], str::to_owned), "ping");
        let long = "ż".repeat(100);
        assert_eq!(
            joined(&[&long, &long], str::to_owned),
            format!("{long} {long}")
        );
    }
}
