//! Typed getters over interaction data (D-250): command options by name and type, the
//! selections of select menus and modals with their [`Resolved`] objects.
//!
//! Every getter returns `None` when the option or component is missing or of another type,
//! so a handler can use `?` or `let … else`; nothing panics.
//!
//! discord.js counterpart: `CommandInteractionOptionResolver`, `ModalSubmitFields` and the
//! `users` / `members` / `roles` / `channels` of the select menu interactions.

use compact_str::CompactString;

use super::{
    CommandData, CommandDataOption, CommandOptionValue, ComponentData, InteractionChannel,
    InteractionMember, ModalSubmitData, Resolved, SubmittedComponent,
};
use crate::{
    guild::Role,
    id::{
        AttachmentId, GenericId, Id, MessageId, RoleId, UserId,
        marker::{ChannelMarker, RoleMarker, UserMarker},
    },
    message::{Attachment, Message},
    user::User,
};

/// A user or a role chosen in a mentionable option or select menu, with its resolved data.
///
/// discord.js counterpart: the `User`, `GuildMember` or `Role` returned by
/// `CommandInteractionOptionResolver#getMentionable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mentionable<'a> {
    /// A user.
    User {
        /// The user.
        user: &'a User,
        /// Their member data, when chosen in a guild.
        member: Option<&'a InteractionMember>,
    },
    /// A role.
    Role(&'a Role),
}

impl Mentionable<'_> {
    /// Returns the ID of the user or role.
    #[must_use]
    pub const fn id(&self) -> GenericId {
        match self {
            Self::User { user, .. } => user.id.cast(),
            Self::Role(role) => role.id.cast(),
        }
    }
}

/// The resolved objects by ID, unboxed (D-256).
impl Resolved {
    fn user(&self, id: UserId) -> Option<&User> {
        self.users.get(&id).map(|user| &**user)
    }

    fn member(&self, id: UserId) -> Option<&InteractionMember> {
        self.members.get(&id).map(|member| &**member)
    }

    fn role(&self, id: RoleId) -> Option<&Role> {
        self.roles.get(&id).map(|role| &**role)
    }

    fn channel(&self, id: Id<ChannelMarker>) -> Option<&InteractionChannel> {
        self.channels.get(&id).map(|channel| &**channel)
    }

    fn message(&self, id: MessageId) -> Option<&Message> {
        self.messages.get(&id).map(|message| &**message)
    }

    fn attachment(&self, id: AttachmentId) -> Option<&Attachment> {
        self.attachments.get(&id).map(|attachment| &**attachment)
    }
}

/// The resolved user `id` and their member data, if both are present.
fn user_and_member(resolved: Option<&Resolved>, id: UserId) -> Option<(&User, &InteractionMember)> {
    let resolved = resolved?;
    Some((resolved.user(id)?, resolved.member(id)?))
}

/// The resolved user or role `id`.
fn mentionable(resolved: Option<&Resolved>, id: GenericId) -> Option<Mentionable<'_>> {
    let resolved = resolved?;
    if let Some(user) = resolved.user(id.cast()) {
        return Some(Mentionable::User {
            user,
            member: resolved.member(id.cast()),
        });
    }
    resolved.role(id.cast()).map(Mentionable::Role)
}

/// The subcommand option among `options` (a subcommand is always the only option).
fn subcommand_option(options: &[CommandDataOption]) -> Option<&CommandDataOption> {
    match options {
        [
            option @ CommandDataOption {
                value: CommandOptionValue::SubCommand(_),
                ..
            },
        ] => Some(option),
        _ => None,
    }
}

/// Reads command options by name and type (D-250).
///
/// The getters look at the options of the subcommand that was run ([`leaf_options`]), so
/// `get_string("reason")` works the same for `/ban reason:…` and `/mod ban reason:…`. User,
/// member, role, channel and attachment getters return the [`resolved`](Self::resolved)
/// object behind the option's ID.
///
/// # Examples
///
/// ```
/// use djs_model::interaction::Interaction;
///
/// # let json = r#"{"id":"1","application_id":"2","type":2,"token":"t","version":1,
/// #   "data":{"id":"3","name":"mod","type":1,"options":[{"name":"warn","type":1,"options":[
/// #   {"name":"reason","type":3,"value":"spam"},{"name":"days","type":4,"value":7}]}]}}"#;
/// // `/mod warn reason:spam days:7`
/// let interaction: Interaction = serde_json::from_str(json).unwrap();
/// let data = interaction.command().unwrap();
/// assert_eq!(data.subcommand(), Some("warn"));
/// assert_eq!(data.get_string("reason"), Some("spam"));
/// assert_eq!(data.get_integer("days"), Some(7));
/// assert_eq!(data.get_boolean("silent"), None); // not filled in
/// assert_eq!(data.get_string("days"), None); // another type
/// ```
///
/// [`leaf_options`]: Self::leaf_options
impl CommandData {
    /// Returns the options of the subcommand that was run, or the command's own options if it
    /// has no subcommands: the options every getter searches.
    ///
    /// discord.js counterpart: the hoisted options of `CommandInteractionOptionResolver`.
    #[must_use]
    pub fn leaf_options(&self) -> &[CommandDataOption] {
        let mut options = self.options.as_slice();
        while let [
            CommandDataOption {
                value:
                    CommandOptionValue::SubCommandGroup(inner) | CommandOptionValue::SubCommand(inner),
                ..
            },
        ] = options
        {
            options = inner;
        }
        options
    }

    /// Returns the name of the subcommand that was run (also inside a group).
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getSubcommand`.
    #[must_use]
    pub fn subcommand(&self) -> Option<&str> {
        let option =
            subcommand_option(&self.options).or_else(|| match self.options.as_slice() {
                [
                    CommandDataOption {
                        value: CommandOptionValue::SubCommandGroup(inner),
                        ..
                    },
                ] => subcommand_option(inner),
                _ => None,
            })?;
        Some(&option.name)
    }

    /// Returns the name of the subcommand group of the subcommand that was run.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getSubcommandGroup`.
    #[must_use]
    pub fn subcommand_group(&self) -> Option<&str> {
        match self.options.as_slice() {
            [
                CommandDataOption {
                    name,
                    value: CommandOptionValue::SubCommandGroup(_),
                },
            ] => Some(name),
            _ => None,
        }
    }

    /// Returns the value of the option `name` (among the [`leaf_options`](Self::leaf_options)),
    /// whatever its type.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#get`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CommandOptionValue> {
        self.leaf_options()
            .iter()
            .find(|option| option.name == name)
            .map(|option| &option.value)
    }

    /// Returns the value of the string option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getString`.
    #[must_use]
    pub fn get_string(&self, name: &str) -> Option<&str> {
        match self.get(name)? {
            CommandOptionValue::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value of the integer option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getInteger`.
    #[must_use]
    pub fn get_integer(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            CommandOptionValue::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value of the number (floating-point) option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getNumber`.
    #[must_use]
    pub fn get_number(&self, name: &str) -> Option<f64> {
        match self.get(name)? {
            CommandOptionValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value of the boolean option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getBoolean`.
    #[must_use]
    pub fn get_boolean(&self, name: &str) -> Option<bool> {
        match self.get(name)? {
            CommandOptionValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// The user ID of the user or mentionable option `name`.
    fn user_id(&self, name: &str) -> Option<UserId> {
        match self.get(name)? {
            CommandOptionValue::User(id) => Some(*id),
            CommandOptionValue::Mentionable(id) => Some(id.cast()),
            _ => None,
        }
    }

    /// Returns the user chosen in the user (or mentionable) option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getUser`.
    #[must_use]
    pub fn get_user(&self, name: &str) -> Option<&User> {
        self.resolved.as_deref()?.user(self.user_id(name)?)
    }

    /// Returns the user chosen in the user (or mentionable) option `name` with their member
    /// data; `None` outside guilds or if the user is not a member.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getMember`.
    #[must_use]
    pub fn get_member(&self, name: &str) -> Option<(&User, &InteractionMember)> {
        user_and_member(self.resolved.as_deref(), self.user_id(name)?)
    }

    /// Returns the role chosen in the role (or mentionable) option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getRole`.
    #[must_use]
    pub fn get_role(&self, name: &str) -> Option<&Role> {
        let id: RoleId = match self.get(name)? {
            CommandOptionValue::Role(id) => *id,
            CommandOptionValue::Mentionable(id) => id.cast(),
            _ => return None,
        };
        self.resolved.as_deref()?.role(id)
    }

    /// Returns the channel chosen in the channel option `name` (a partial channel with the
    /// invoking member's permissions in it).
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getChannel`.
    #[must_use]
    pub fn get_channel(&self, name: &str) -> Option<&InteractionChannel> {
        let CommandOptionValue::Channel(id) = self.get(name)? else {
            return None;
        };
        self.resolved.as_deref()?.channel(*id)
    }

    /// Returns the file uploaded in the attachment option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getAttachment`.
    #[must_use]
    pub fn get_attachment(&self, name: &str) -> Option<&Attachment> {
        let CommandOptionValue::Attachment(id) = self.get(name)? else {
            return None;
        };
        self.resolved.as_deref()?.attachment(*id)
    }

    /// Returns the user or role chosen in the mentionable option `name`.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getMentionable`.
    #[must_use]
    pub fn get_mentionable(&self, name: &str) -> Option<Mentionable<'_>> {
        let CommandOptionValue::Mentionable(id) = self.get(name)? else {
            return None;
        };
        mentionable(self.resolved.as_deref(), *id)
    }

    /// Returns what the user has typed so far in the [`focused`](Self::focused) option of an
    /// autocomplete interaction.
    ///
    /// discord.js counterpart: `CommandInteractionOptionResolver#getFocused`.
    #[must_use]
    pub fn focused_value(&self) -> Option<&str> {
        match &self.focused()?.value {
            CommandOptionValue::Focused { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Returns the user a user command was used on.
    ///
    /// discord.js counterpart: `UserContextMenuCommandInteraction#targetUser`.
    #[must_use]
    pub fn target_user(&self) -> Option<&User> {
        self.resolved.as_deref()?.user(self.target_id?.cast())
    }

    /// Returns the user a user command was used on with their member data (in guilds).
    ///
    /// discord.js counterpart: `UserContextMenuCommandInteraction#targetMember`.
    #[must_use]
    pub fn target_member(&self) -> Option<(&User, &InteractionMember)> {
        user_and_member(self.resolved.as_deref(), self.target_id?.cast())
    }

    /// Returns the message a message command was used on.
    ///
    /// discord.js counterpart: `MessageContextMenuCommandInteraction#targetMessage`.
    #[must_use]
    pub fn target_message(&self) -> Option<&Message> {
        self.resolved.as_deref()?.message(self.target_id?.cast())
    }
}

/// The selections of entity select menus with their resolved objects (D-250), in the order
/// of [`values`](Self::values).
///
/// In a mentionable select menu, [`users`](Self::users) and [`roles`](Self::roles) each
/// return their part of the selection.
impl ComponentData {
    /// Returns the users chosen in a user or mentionable select menu.
    ///
    /// discord.js counterpart: `UserSelectMenuInteraction#users`.
    pub fn users(&self) -> impl Iterator<Item = &User> + '_ {
        let resolved = self.resolved.as_deref();
        self.ids::<UserMarker>()
            .filter_map(move |id| resolved?.user(id))
    }

    /// Returns the users chosen in a user or mentionable select menu in a guild, with their
    /// member data.
    ///
    /// discord.js counterpart: `UserSelectMenuInteraction#members`.
    pub fn members(&self) -> impl Iterator<Item = (&User, &InteractionMember)> + '_ {
        let resolved = self.resolved.as_deref();
        self.ids::<UserMarker>()
            .filter_map(move |id| user_and_member(resolved, id))
    }

    /// Returns the roles chosen in a role or mentionable select menu.
    ///
    /// discord.js counterpart: `RoleSelectMenuInteraction#roles`.
    pub fn roles(&self) -> impl Iterator<Item = &Role> + '_ {
        let resolved = self.resolved.as_deref();
        self.ids::<RoleMarker>()
            .filter_map(move |id| resolved?.role(id))
    }

    /// Returns the channels chosen in a channel select menu.
    ///
    /// discord.js counterpart: `ChannelSelectMenuInteraction#channels`.
    pub fn channels(&self) -> impl Iterator<Item = &InteractionChannel> + '_ {
        let resolved = self.resolved.as_deref();
        self.ids::<ChannelMarker>()
            .filter_map(move |id| resolved?.channel(id))
    }

    /// Returns the users and roles chosen in a mentionable select menu.
    ///
    /// discord.js counterpart: `MentionableSelectMenuInteraction` (`users`, `members`,
    /// `roles`).
    pub fn mentionables(&self) -> impl Iterator<Item = Mentionable<'_>> + '_ {
        let resolved = self.resolved.as_deref();
        self.ids().filter_map(move |id| mentionable(resolved, id))
    }
}

/// Reads submitted modal fields by custom ID and component type (D-250); see also
/// [`get`](Self::get) and [`text_value`](Self::text_value).
///
/// Each getter returns `None` when no component has the custom ID or it is of another type;
/// the `selected_*` getters return the [`resolved`](Self::resolved) objects in the order they
/// were selected.
///
/// discord.js counterpart: `ModalSubmitFields`.
impl ModalSubmitData {
    /// Returns the values chosen in the string select menu `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getStringSelectValues`.
    #[must_use]
    pub fn string_select_values(&self, custom_id: &str) -> Option<&[CompactString]> {
        match self.get(custom_id)? {
            SubmittedComponent::StringSelect(select) => Some(&select.values),
            _ => None,
        }
    }

    /// Returns the values of the checked boxes of the checkbox group `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getCheckboxGroup`.
    #[must_use]
    pub fn checkbox_group_values(&self, custom_id: &str) -> Option<&[CompactString]> {
        match self.get(custom_id)? {
            SubmittedComponent::CheckboxGroup(group) => Some(&group.values),
            _ => None,
        }
    }

    /// Returns the value chosen in the radio group `custom_id` (`None` also when nothing was
    /// chosen).
    ///
    /// discord.js counterpart: `ModalSubmitFields#getRadioGroup`.
    #[must_use]
    pub fn radio_group_value(&self, custom_id: &str) -> Option<&str> {
        match self.get(custom_id)? {
            SubmittedComponent::RadioGroup(group) => group.value.as_deref(),
            _ => None,
        }
    }

    /// Returns whether the checkbox `custom_id` was checked.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getCheckbox`.
    #[must_use]
    pub fn checkbox_value(&self, custom_id: &str) -> Option<bool> {
        match self.get(custom_id)? {
            SubmittedComponent::Checkbox(checkbox) => Some(checkbox.value),
            _ => None,
        }
    }

    /// The user IDs selected in the user or mentionable select menu `custom_id`.
    fn user_ids<'a>(&'a self, custom_id: &str) -> Option<impl Iterator<Item = UserId> + use<'a>> {
        let (users, mentionables): (&[UserId], &[GenericId]) = match self.get(custom_id)? {
            SubmittedComponent::UserSelect(select) => (&select.values, &[]),
            SubmittedComponent::MentionableSelect(select) => (&[], &select.values),
            _ => return None,
        };
        Some(
            users
                .iter()
                .copied()
                .chain(mentionables.iter().map(|id| id.cast())),
        )
    }

    /// Returns the users chosen in the user or mentionable select menu `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getSelectedUsers`.
    #[must_use]
    pub fn selected_users<'a>(
        &'a self,
        custom_id: &str,
    ) -> Option<impl Iterator<Item = &'a User> + use<'a>> {
        let resolved = self.resolved.as_deref();
        Some(
            self.user_ids(custom_id)?
                .filter_map(move |id| resolved?.user(id)),
        )
    }

    /// Returns the users chosen in the user or mentionable select menu `custom_id`, with their
    /// member data (in guilds).
    ///
    /// discord.js counterpart: `ModalSubmitFields#getSelectedMembers`.
    #[must_use]
    pub fn selected_members<'a>(
        &'a self,
        custom_id: &str,
    ) -> Option<impl Iterator<Item = (&'a User, &'a InteractionMember)> + use<'a>> {
        let resolved = self.resolved.as_deref();
        Some(
            self.user_ids(custom_id)?
                .filter_map(move |id| user_and_member(resolved, id)),
        )
    }

    /// Returns the roles chosen in the role or mentionable select menu `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getSelectedRoles`.
    #[must_use]
    pub fn selected_roles<'a>(
        &'a self,
        custom_id: &str,
    ) -> Option<impl Iterator<Item = &'a Role> + use<'a>> {
        let (roles, mentionables): (&[RoleId], &[GenericId]) = match self.get(custom_id)? {
            SubmittedComponent::RoleSelect(select) => (&select.values, &[]),
            SubmittedComponent::MentionableSelect(select) => (&[], &select.values),
            _ => return None,
        };
        let resolved = self.resolved.as_deref();
        Some(
            roles
                .iter()
                .copied()
                .chain(mentionables.iter().map(|id| id.cast()))
                .filter_map(move |id| resolved?.role(id)),
        )
    }

    /// Returns the channels chosen in the channel select menu `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getSelectedChannels`.
    #[must_use]
    pub fn selected_channels<'a>(
        &'a self,
        custom_id: &str,
    ) -> Option<impl Iterator<Item = &'a InteractionChannel> + use<'a>> {
        let SubmittedComponent::ChannelSelect(select) = self.get(custom_id)? else {
            return None;
        };
        let resolved = self.resolved.as_deref();
        Some(
            select
                .values
                .iter()
                .filter_map(move |id: &Id<ChannelMarker>| resolved?.channel(*id)),
        )
    }

    /// Returns the users and roles chosen in the mentionable select menu `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getSelectedMentionables`.
    #[must_use]
    pub fn selected_mentionables<'a>(
        &'a self,
        custom_id: &str,
    ) -> Option<impl Iterator<Item = Mentionable<'a>> + use<'a>> {
        let SubmittedComponent::MentionableSelect(select) = self.get(custom_id)? else {
            return None;
        };
        let resolved = self.resolved.as_deref();
        Some(
            select
                .values
                .iter()
                .filter_map(move |id| mentionable(resolved, *id)),
        )
    }

    /// Returns the files uploaded in the file upload `custom_id`.
    ///
    /// discord.js counterpart: `ModalSubmitFields#getUploadedFiles`.
    #[must_use]
    pub fn uploaded_files<'a>(
        &'a self,
        custom_id: &str,
    ) -> Option<impl Iterator<Item = &'a Attachment> + use<'a>> {
        let SubmittedComponent::FileUpload(upload) = self.get(custom_id)? else {
            return None;
        };
        let resolved = self.resolved.as_deref();
        Some(
            upload
                .values
                .iter()
                .filter_map(move |id| resolved?.attachment(*id)),
        )
    }
}
