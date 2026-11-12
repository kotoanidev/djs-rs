//! [Interactions](https://docs.discord.com/developers/interactions/receiving-and-responding):
//! what a user does with an app (commands, components, modals), and the app's responses.
//!
//! An [`Interaction`] holds the fields every interaction has, and its [`data`](Interaction::data)
//! is an enum keyed by the interaction type (D-067). [`InteractionResponse`] is what the app
//! sends back (D-068).

mod data;
mod getters;
mod response;

use core::{fmt, ops::Deref};

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Unexpected, Visitor},
};

pub use self::{
    data::{
        CommandData, CommandDataOption, CommandOptionValue, ComponentData, InteractionData,
        ModalSubmitData, Resolved, SubmittedActionRow, SubmittedCheckbox, SubmittedComponent,
        SubmittedEntitySelect, SubmittedFileUpload, SubmittedLabel, SubmittedRadioGroup,
        SubmittedSelect, SubmittedTextDisplay, SubmittedTextInput, UnknownSubmittedComponent,
    },
    getters::Mentionable,
    response::{
        ActivityInstanceResource, AutocompleteCallbackData, AutocompleteChoices, CallbackResource,
        DeferredCallbackData, InteractionCallback, InteractionCallbackResponse,
        InteractionCallbackType, InteractionResponse, MessageCallbackData, ModalCallbackData,
    },
};
use crate::{
    channel::{ChannelFlags, ChannelType, ThreadMetadata},
    entitlement::Entitlement,
    guild::{Feature, MemberFlags, PartialMember},
    id::{ApplicationId, ChannelId, GuildId, InteractionId, MessageId, RoleId, UserId},
    macros::int_enum,
    message::Message,
    permissions::Permissions,
    user::{AvatarDecorationData, Collectibles, User},
    util::{ImageHash, Locale, Timestamp},
};

/// An [interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object):
/// a user used one of the app's commands, components or modals (or Discord checks the app's
/// HTTP endpoint).
///
/// The [`data`](Self::data) enum tells which kind it is (D-067). The interaction is answered
/// with an [`InteractionResponse`] within 3 seconds; its [`token`](Self::token) then stays
/// valid for follow-up messages for 15 minutes. `Debug` does not print the token.
///
/// Contains floats (number options), so it is `PartialEq` but not `Eq` (D-064).
///
/// discord.js counterpart: `BaseInteraction` and its subclasses.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub struct Interaction {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The application the interaction is for.
    pub application_id: ApplicationId,
    /// The kind of interaction and its kind-specific data (D-067).
    pub data: InteractionData,
    /// The guild it was triggered in (a partial object).
    pub guild: Option<InteractionGuild>,
    /// The guild it was triggered in.
    pub guild_id: Option<GuildId>,
    /// The channel it was triggered in (a partial object).
    pub channel: Option<Box<InteractionChannel>>,
    /// The channel it was triggered in.
    pub channel_id: Option<ChannelId>,
    /// The invoking member, with their permissions in the channel (guild interactions).
    pub member: Option<Box<InteractionMember>>,
    /// The invoking user (DM interactions; in guilds use [`member`](Self::member), or
    /// [`author`](Self::author) for both).
    pub user: Option<Box<User>>,
    /// The token to respond with; secret, valid for 15 minutes.
    pub token: CompactString,
    /// Always `1`.
    pub version: u8,
    /// The message the component was on (component interactions and modals opened by one).
    pub message: Option<Box<Message>>,
    /// The app's permissions in the channel (with the extra permissions apps get in DMs).
    pub app_permissions: Permissions,
    /// The invoking user's language (every interaction except pings).
    pub locale: Option<Locale>,
    /// The guild's preferred locale.
    pub guild_locale: Option<Locale>,
    /// The user's and guild's entitlements to the app's SKUs (monetized apps).
    pub entitlements: Vec<Entitlement>,
    /// The installations that authorized the interaction.
    pub authorizing_integration_owners: AuthorizingIntegrationOwners,
    /// Where the interaction was triggered.
    pub context: Option<InteractionContextType>,
    /// The maximum size of attachments in a response, in bytes.
    pub attachment_size_limit: Option<u64>,
}

impl Interaction {
    /// Returns the interaction type.
    #[must_use]
    pub const fn kind(&self) -> InteractionType {
        self.data.kind()
    }

    /// Returns the invoking user, from [`member`](Self::member) in guilds or
    /// [`user`](Self::user) in DMs.
    #[must_use]
    pub fn author(&self) -> Option<&User> {
        self.member
            .as_ref()
            .and_then(|member| member.user.as_ref())
            .or(self.user.as_deref())
    }

    /// Returns the command data if a user ran one of the app's commands (not for
    /// autocomplete, unlike [`InteractionData::command`]; D-249).
    ///
    /// discord.js counterpart: `BaseInteraction#isCommand`.
    #[must_use]
    pub fn command(&self) -> Option<&CommandData> {
        match &self.data {
            InteractionData::ApplicationCommand(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the command data if a user is typing an option with autocomplete (D-249); the
    /// typed option is [`CommandData::focused`].
    ///
    /// discord.js counterpart: `BaseInteraction#isAutocomplete`.
    #[must_use]
    pub fn autocomplete(&self) -> Option<&CommandData> {
        match &self.data {
            InteractionData::Autocomplete(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the component data if a user clicked a button or chose in a select menu
    /// (D-249).
    ///
    /// discord.js counterpart: `BaseInteraction#isMessageComponent`.
    #[must_use]
    pub fn component(&self) -> Option<&ComponentData> {
        match &self.data {
            InteractionData::MessageComponent(data) => Some(data),
            _ => None,
        }
    }

    /// Returns the submitted fields if a user submitted one of the app's modals (D-249).
    ///
    /// discord.js counterpart: `BaseInteraction#isModalSubmit`.
    #[must_use]
    pub fn modal(&self) -> Option<&ModalSubmitData> {
        match &self.data {
            InteractionData::ModalSubmit(data) => Some(data),
            _ => None,
        }
    }
}

impl fmt::Debug for Interaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interaction")
            .field("id", &self.id)
            .field("application_id", &self.application_id)
            .field("data", &self.data)
            .field("guild", &self.guild)
            .field("guild_id", &self.guild_id)
            .field("channel", &self.channel)
            .field("channel_id", &self.channel_id)
            .field("member", &self.member)
            .field("user", &self.user)
            .field("token", &"<redacted>")
            .field("version", &self.version)
            .field("message", &self.message)
            .field("app_permissions", &self.app_permissions)
            .field("locale", &self.locale)
            .field("guild_locale", &self.guild_locale)
            .field("entitlements", &self.entitlements)
            .field(
                "authorizing_integration_owners",
                &self.authorizing_integration_owners,
            )
            .field("context", &self.context)
            .field("attachment_size_limit", &self.attachment_size_limit)
            .finish()
    }
}

/// Every interaction field, decoded in one pass; `data` is converted by `type` afterwards
/// (D-067).
#[derive(Deserialize)]
struct RawInteraction {
    id: InteractionId,
    application_id: ApplicationId,
    #[serde(rename = "type")]
    kind: InteractionType,
    #[serde(default)]
    data: Option<data::RawData>,
    #[serde(default)]
    guild: Option<InteractionGuild>,
    #[serde(default)]
    guild_id: Option<GuildId>,
    #[serde(default)]
    channel: Option<Box<InteractionChannel>>,
    #[serde(default)]
    channel_id: Option<ChannelId>,
    #[serde(default)]
    member: Option<Box<InteractionMember>>,
    #[serde(default)]
    user: Option<Box<User>>,
    token: CompactString,
    #[serde(default = "default_version")]
    version: u8,
    #[serde(default)]
    message: Option<Box<Message>>,
    #[serde(default)]
    app_permissions: Permissions,
    #[serde(default)]
    locale: Option<Locale>,
    #[serde(default)]
    guild_locale: Option<Locale>,
    #[serde(default)]
    entitlements: Vec<Entitlement>,
    #[serde(default)]
    authorizing_integration_owners: AuthorizingIntegrationOwners,
    #[serde(default)]
    context: Option<InteractionContextType>,
    #[serde(default)]
    attachment_size_limit: Option<u64>,
}

const fn default_version() -> u8 {
    1
}

impl<'de> Deserialize<'de> for Interaction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawInteraction::deserialize(deserializer)?;
        Ok(Self {
            id: raw.id,
            application_id: raw.application_id,
            data: data::RawData::into_data(raw.data, raw.kind)?,
            guild: raw.guild,
            guild_id: raw.guild_id,
            channel: raw.channel,
            channel_id: raw.channel_id,
            member: raw.member,
            user: raw.user,
            token: raw.token,
            version: raw.version,
            message: raw.message,
            app_permissions: raw.app_permissions,
            locale: raw.locale,
            guild_locale: raw.guild_locale,
            entitlements: raw.entitlements,
            authorizing_integration_owners: raw.authorizing_integration_owners,
            context: raw.context,
            attachment_size_limit: raw.attachment_size_limit,
        })
    }
}

impl Serialize for Interaction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Flat<'a> {
            id: InteractionId,
            application_id: ApplicationId,
            #[serde(rename = "type")]
            kind: InteractionType,
            #[serde(skip_serializing_if = "Option::is_none")]
            data: Option<data::DataRef<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            guild: Option<&'a InteractionGuild>,
            #[serde(skip_serializing_if = "Option::is_none")]
            guild_id: Option<GuildId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            channel: Option<&'a InteractionChannel>,
            #[serde(skip_serializing_if = "Option::is_none")]
            channel_id: Option<ChannelId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            member: Option<&'a InteractionMember>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user: Option<&'a User>,
            token: &'a str,
            version: u8,
            #[serde(skip_serializing_if = "Option::is_none")]
            message: Option<&'a Message>,
            app_permissions: Permissions,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: Option<&'a Locale>,
            #[serde(skip_serializing_if = "Option::is_none")]
            guild_locale: Option<&'a Locale>,
            entitlements: &'a [Entitlement],
            authorizing_integration_owners: &'a AuthorizingIntegrationOwners,
            #[serde(skip_serializing_if = "Option::is_none")]
            context: Option<InteractionContextType>,
            #[serde(skip_serializing_if = "Option::is_none")]
            attachment_size_limit: Option<u64>,
        }

        Flat {
            id: self.id,
            application_id: self.application_id,
            kind: self.kind(),
            data: self.data.as_ref(),
            guild: self.guild.as_ref(),
            guild_id: self.guild_id,
            channel: self.channel.as_deref(),
            channel_id: self.channel_id,
            member: self.member.as_deref(),
            user: self.user.as_deref(),
            token: &self.token,
            version: self.version,
            message: self.message.as_deref(),
            app_permissions: self.app_permissions,
            locale: self.locale.as_ref(),
            guild_locale: self.guild_locale.as_ref(),
            entitlements: &self.entitlements,
            authorizing_integration_owners: &self.authorizing_integration_owners,
            context: self.context,
            attachment_size_limit: self.attachment_size_limit,
        }
        .serialize(serializer)
    }
}

/// The [partial guild](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-interaction-structure)
/// an interaction was triggered in (D-051).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InteractionGuild {
    /// The guild's ID.
    pub id: GuildId,
    /// The guild's preferred locale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<Locale>,
    /// The guild's features.
    #[serde(default)]
    pub features: Vec<Feature>,
}

/// The partial channel an interaction was triggered in, or a channel in [`Resolved`] data
/// (D-051): the fields Discord sends there, plus the invoking member's permissions in it.
///
/// discord.js counterpart: the channel of an interaction (`Interaction#channel`) and the
/// resolved channels of command options.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InteractionChannel {
    /// The channel's ID.
    pub id: ChannelId,
    /// The channel's type.
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// The channel's name (absent for DMs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The guild, for guild channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The invoking member's permissions in the channel (resolved channels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// The app's permissions in the channel (resolved channels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_permissions: Option<Permissions>,
    /// The parent category, or the parent channel of a thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ChannelId>,
    /// The sorting position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    /// The channel's topic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<CompactString>,
    /// Whether the channel is age-restricted.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub nsfw: bool,
    /// The last message sent in the channel (may not exist any more).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_message_id: Option<MessageId>,
    /// When the last message was pinned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<Timestamp>,
    /// The slowmode in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
    /// The channel's flags.
    #[serde(default)]
    pub flags: ChannelFlags,
    /// The thread's metadata, for threads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_metadata: Option<ThreadMetadata>,
    /// The recipients of a DM or group DM.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipients: Vec<User>,
}

/// A member in an interaction: the invoking [`member`](Interaction::member) (with its
/// [`user`](Self::user)) or a member in [`Resolved`] data (without user), together with their
/// [`permissions`](Self::permissions) in the channel.
///
/// Dereferences to the [`PartialMember`], so `member.nick` works. Decoded in one pass, like
/// [`Mention`](crate::message::Mention) (D-060).
///
/// discord.js counterpart: `GuildMember` (`Interaction#member`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct InteractionMember {
    /// The member's guild-specific data.
    pub member: PartialMember,
    /// The member's user (the invoking member; absent in resolved data, where the user is in
    /// [`Resolved::users`]).
    pub user: Option<User>,
    /// The member's permissions in the channel, including overwrites.
    pub permissions: Permissions,
}

impl Deref for InteractionMember {
    type Target = PartialMember;

    fn deref(&self) -> &PartialMember {
        &self.member
    }
}

/// The fields of a [`PartialMember`] plus `user` and `permissions`, decoded in one pass. Must
/// list the same fields with the same serde attributes as [`PartialMember`].
#[derive(Deserialize)]
struct RawInteractionMember {
    #[serde(default)]
    nick: Option<CompactString>,
    #[serde(default)]
    avatar: Option<ImageHash>,
    #[serde(default)]
    banner: Option<ImageHash>,
    roles: Vec<RoleId>,
    joined_at: Option<Timestamp>,
    #[serde(default)]
    premium_since: Option<Timestamp>,
    #[serde(default)]
    deaf: bool,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    flags: MemberFlags,
    #[serde(default)]
    pending: bool,
    #[serde(default)]
    communication_disabled_until: Option<Timestamp>,
    #[serde(default)]
    avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    #[serde(default)]
    collectibles: Option<Box<Collectibles>>,
    #[serde(default)]
    user: Option<User>,
    #[serde(default)]
    permissions: Permissions,
}

impl<'de> Deserialize<'de> for InteractionMember {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawInteractionMember::deserialize(deserializer)?;
        Ok(Self {
            member: PartialMember {
                nick: raw.nick,
                avatar: raw.avatar,
                banner: raw.banner,
                roles: raw.roles,
                joined_at: raw.joined_at,
                premium_since: raw.premium_since,
                deaf: raw.deaf,
                mute: raw.mute,
                flags: raw.flags,
                pending: raw.pending,
                communication_disabled_until: raw.communication_disabled_until,
                avatar_decoration_data: raw.avatar_decoration_data,
                collectibles: raw.collectibles,
            },
            user: raw.user,
            permissions: raw.permissions,
        })
    }
}

impl Serialize for InteractionMember {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Flat<'a> {
            #[serde(flatten)]
            member: &'a PartialMember,
            #[serde(skip_serializing_if = "Option::is_none")]
            user: Option<&'a User>,
            permissions: Permissions,
        }

        Flat {
            member: &self.member,
            user: self.user.as_ref(),
            permissions: self.permissions,
        }
        .serialize(serializer)
    }
}

int_enum! {
    /// The [type of an interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-interaction-type).
    ///
    /// discord.js counterpart: `InteractionType`.
    pub enum InteractionType: u8 {
        /// Discord checks that an HTTP interactions endpoint works.
        Ping = 1,
        /// A slash command or a context menu command.
        ApplicationCommand = 2,
        /// A button, select menu or other message component.
        MessageComponent = 3,
        /// Autocomplete suggestions for a command option.
        ApplicationCommandAutocomplete = 4,
        /// A submitted modal.
        ModalSubmit = 5,
    }
}

int_enum! {
    /// [Where an interaction can be used](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-interaction-context-types),
    /// or was triggered from.
    ///
    /// discord.js counterpart: `InteractionContextType`.
    pub enum InteractionContextType: u8 {
        /// In guilds.
        Guild = 0,
        /// In DMs with the app's bot user.
        BotDm = 1,
        /// In group DMs and DMs other than with the app's bot user.
        PrivateChannel = 2,
    }
}

/// The [interaction a message responds to](https://docs.discord.com/developers/interactions/receiving-and-responding#message-interaction-object)
/// (`Message::interaction`).
///
/// **Deprecated** by Discord in favor of
/// [`Message::interaction_metadata`](crate::message::Message::interaction_metadata).
///
/// discord.js counterpart: `MessageInteraction` (`Message#interaction`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MessageInteraction {
    /// The interaction's ID.
    pub id: InteractionId,
    /// The kind of interaction.
    #[serde(rename = "type")]
    pub kind: InteractionType,
    /// The name of the command, including subcommands and subcommand groups.
    pub name: CompactString,
    /// The user who invoked the interaction.
    pub user: User,
    /// The member who invoked the interaction, in guilds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Box<PartialMember>>,
}

/// The [installations that authorized an interaction](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-object-authorizing-integration-owners-object),
/// keyed by [`IntegrationType`](crate::application::IntegrationType).
///
/// A key is only present when the app is installed in that context, the interaction's context
/// supports it and, for commands, the command supports that installation context.
///
/// discord.js counterpart: `authorizingIntegrationOwners`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuthorizingIntegrationOwners {
    /// The guild install (key `"0"`).
    #[serde(rename = "0", default, skip_serializing_if = "Option::is_none")]
    pub guild_install: Option<GuildInstallOwner>,
    /// The ID of the user who installed the app (key `"1"`).
    #[serde(rename = "1", default, skip_serializing_if = "Option::is_none")]
    pub user_install: Option<UserId>,
}

/// The owner of a guild install in [`AuthorizingIntegrationOwners`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GuildInstallOwner {
    /// The interaction happened in this guild.
    Guild(GuildId),
    /// The interaction happened in a DM with the app's bot user (Discord sends `"0"`).
    BotDm,
}

impl GuildInstallOwner {
    /// Returns the guild ID, unless the interaction happened in a DM with the bot.
    #[must_use]
    pub const fn guild_id(self) -> Option<GuildId> {
        match self {
            Self::Guild(id) => Some(id),
            Self::BotDm => None,
        }
    }
}

impl Serialize for GuildInstallOwner {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Guild(id) => id.serialize(serializer),
            Self::BotDm => serializer.serialize_str("0"),
        }
    }
}

impl<'de> Deserialize<'de> for GuildInstallOwner {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OwnerVisitor;

        impl Visitor<'_> for OwnerVisitor {
            type Value = GuildInstallOwner;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a guild ID or \"0\"")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<GuildInstallOwner, E> {
                if value == "0" {
                    return Ok(GuildInstallOwner::BotDm);
                }
                value
                    .parse()
                    .map(GuildInstallOwner::Guild)
                    .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(OwnerVisitor)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use super::*;

    /// Size guards (D-054): interactions are decoded on every `INTERACTION_CREATE`.
    #[test]
    fn sizes() {
        assert!(
            size_of::<Interaction>() <= 288,
            "{}",
            size_of::<Interaction>()
        );
        assert_eq!(size_of::<InteractionData>(), 16);
        assert!(
            size_of::<CommandDataOption>() <= 56,
            "{}",
            size_of::<CommandDataOption>()
        );
        assert!(
            size_of::<SubmittedComponent>() <= 64,
            "{}",
            size_of::<SubmittedComponent>()
        );
        assert!(
            size_of::<InteractionResponse>() <= 32,
            "{}",
            size_of::<InteractionResponse>()
        );
    }

    #[test]
    fn debug_redacts_the_token() {
        let interaction: Interaction = serde_json::from_str(
            r#"{"id":"1","application_id":"2","type":1,"token":"secret-token","version":1}"#,
        )
        .unwrap();
        let debug = format!("{interaction:?}");
        assert!(!debug.contains("secret-token"), "{debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
    }
}
