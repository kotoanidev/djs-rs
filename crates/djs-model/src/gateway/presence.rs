//! Presence data sent over the gateway: activities and statuses.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{ApplicationId, EmojiId, GuildId, UserId},
    macros::{int_enum, int_flags, str_enum},
    user::{AvatarDecorationData, Collectibles, PrimaryGuild, UserFlags},
    util::{ImageHash, Timestamp},
};

/// A user's [presence](https://docs.discord.com/developers/events/gateway-events#presence-update):
/// their status and activities in a guild.
///
/// Received in `PRESENCE_UPDATE`, `GUILD_CREATE` (without `guild_id`), `GUILD_MEMBERS_CHUNK`
/// and `THREAD_MEMBERS_UPDATE`. Requires the `GUILD_PRESENCES` intent.
///
/// discord.js counterpart: `Presence`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Presence {
    /// The user (only `id` is guaranteed, D-074).
    pub user: PresenceUser,
    /// The guild (absent in `GUILD_CREATE`, where the guild is known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// The overall status.
    #[serde(default = "offline")]
    pub status: Status,
    /// The current activities.
    #[serde(default, deserialize_with = "crate::serde_util::null_as_default")]
    pub activities: Vec<Activity>,
    /// The status on each platform.
    #[serde(default)]
    pub client_status: ClientStatus,
}

fn offline() -> Status {
    Status::Offline
}

/// The partial user of a [`Presence`] (D-074): only [`id`](Self::id) is guaranteed; the other
/// fields are present when they changed. `None` means "not sent" (or `null`).
///
/// discord.js counterpart: the `user` of the `presenceUpdate` data, used to update `User`s.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PresenceUser {
    /// The user's ID.
    pub id: UserId,
    /// The username.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<CompactString>,
    /// The legacy tag (`0` for migrated users).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::user::option_discriminator"
    )]
    pub discriminator: Option<u16>,
    /// The display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_name: Option<CompactString>,
    /// The avatar hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<ImageHash>,
    /// Whether the user is a bot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<bool>,
    /// Whether the user is a Discord system user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
    /// The banner hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    /// The banner color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    /// The public flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_flags: Option<UserFlags>,
    /// The avatar decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<Box<AvatarDecorationData>>,
    /// The collectibles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collectibles: Option<Box<Collectibles>>,
    /// The displayed server tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_guild: Option<Box<PrimaryGuild>>,
}

/// A user's [status per platform](https://docs.discord.com/developers/events/gateway-events#client-status-object);
/// a platform is `None` when the user is offline or invisible there.
///
/// discord.js counterpart: `ClientPresenceStatusData`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClientStatus {
    /// The desktop app session's status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<Status>,
    /// The mobile app session's status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mobile: Option<Status>,
    /// The web (browser, bot) session's status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<Status>,
    /// The virtual reality app session's status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vr: Option<Status>,
}

/// An [activity](https://docs.discord.com/developers/events/gateway-events#activity-object): a
/// game, stream, song, custom status, … shown in a user's presence.
///
/// Bots can only set [`name`](Self::name), [`state`](Self::state), [`kind`](Self::kind) and
/// [`url`](Self::url) (see [`Activity::new`]).
///
/// discord.js counterpart: `Activity`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Activity {
    /// The activity's name.
    pub name: CompactString,
    /// The kind of activity.
    #[serde(rename = "type")]
    pub kind: ActivityType,
    /// The stream URL ([`Streaming`](ActivityType::Streaming); Twitch and YouTube only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<CompactString>,
    /// When the activity was added to the user's session.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_unix_millis"
    )]
    pub created_at: Option<Timestamp>,
    /// When the game started and/or ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<ActivityTimestamps>,
    /// The game's application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
    /// Which field the member list shows as status text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_display_type: Option<StatusDisplayType>,
    /// What the player is doing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<CompactString>,
    /// The URL linked from the details text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details_url: Option<CompactString>,
    /// The party status, or the text of a custom status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<CompactString>,
    /// The URL linked from the state text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_url: Option<CompactString>,
    /// The emoji of a custom status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<ActivityEmoji>,
    /// The player's party.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party: Option<ActivityParty>,
    /// The images and their hover texts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<Box<ActivityAssets>>,
    /// The Rich Presence secrets for joining and spectating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Box<ActivitySecrets>>,
    /// Whether the activity is an instanced game session.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub instance: bool,
    /// What the activity supports.
    #[serde(default, skip_serializing_if = "ActivityFlags::is_empty")]
    pub flags: ActivityFlags,
    /// The labels of the custom buttons (max 2; bots cannot see the URLs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<CompactString>,
}

impl Activity {
    /// Creates an activity with the fields bots can set: a name and kind (set
    /// [`state`](Self::state) for a custom status text, [`url`](Self::url) for a stream).
    #[must_use]
    pub fn new(kind: ActivityType, name: impl Into<CompactString>) -> Self {
        Self {
            name: name.into(),
            kind,
            url: None,
            created_at: None,
            timestamps: None,
            application_id: None,
            status_display_type: None,
            details: None,
            details_url: None,
            state: None,
            state_url: None,
            emoji: None,
            party: None,
            assets: None,
            secrets: None,
            instance: false,
            flags: ActivityFlags::empty(),
            buttons: Vec::new(),
        }
    }

    /// "Playing {name}".
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::{Activity, ActivityType};
    ///
    /// let activity = Activity::playing("with Rust");
    /// assert_eq!((activity.kind, activity.name.as_str()), (ActivityType::Playing, "with Rust"));
    /// ```
    #[must_use]
    pub fn playing(name: impl Into<CompactString>) -> Self {
        Self::new(ActivityType::Playing, name)
    }

    /// "Streaming {name}", linking to `url`. Discord only accepts `https://twitch.tv/` and
    /// `https://youtube.com/` URLs.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::Activity;
    ///
    /// let activity = Activity::streaming("Rust live", "https://twitch.tv/example");
    /// assert_eq!(
    ///     serde_json::to_string(&activity).unwrap(),
    ///     r#"{"name":"Rust live","type":1,"url":"https://twitch.tv/example"}"#
    /// );
    /// ```
    #[must_use]
    pub fn streaming(name: impl Into<CompactString>, url: impl Into<CompactString>) -> Self {
        Self {
            url: Some(url.into()),
            ..Self::new(ActivityType::Streaming, name)
        }
    }

    /// "Listening to {name}".
    #[must_use]
    pub fn listening(name: impl Into<CompactString>) -> Self {
        Self::new(ActivityType::Listening, name)
    }

    /// "Watching {name}".
    #[must_use]
    pub fn watching(name: impl Into<CompactString>) -> Self {
        Self::new(ActivityType::Watching, name)
    }

    /// "Competing in {name}".
    #[must_use]
    pub fn competing(name: impl Into<CompactString>) -> Self {
        Self::new(ActivityType::Competing, name)
    }

    /// A custom status showing `text` (stored in [`state`](Self::state); the name is
    /// `"Custom Status"`, as Discord's clients send).
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::gateway::Activity;
    ///
    /// let activity = Activity::custom("Fixing bugs");
    /// assert_eq!(activity.state.as_deref(), Some("Fixing bugs"));
    /// ```
    #[must_use]
    pub fn custom(text: impl Into<CompactString>) -> Self {
        Self {
            state: Some(text.into()),
            ..Self::new(ActivityType::Custom, "Custom Status")
        }
    }
}

int_enum! {
    /// The [kind of an activity](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-types).
    ///
    /// discord.js counterpart: `ActivityType`.
    pub enum ActivityType: u8 {
        /// "Playing {name}".
        Playing = 0,
        /// "Streaming {details}".
        Streaming = 1,
        /// "Listening to {name}".
        Listening = 2,
        /// "Watching {name}".
        Watching = 3,
        /// "{emoji} {state}": a custom status.
        Custom = 4,
        /// "Competing in {name}".
        Competing = 5,
    }
}

int_enum! {
    /// [Which field is shown](https://docs.discord.com/developers/events/gateway-events#activity-object-status-display-types)
    /// as the status text in the member list.
    ///
    /// discord.js counterpart: `StatusDisplayType`.
    pub enum StatusDisplayType: u8 {
        /// The activity's name ("Listening to Spotify").
        Name = 0,
        /// The state ("Listening to Rick Astley").
        State = 1,
        /// The details ("Listening to Never Gonna Give You Up").
        Details = 2,
    }
}

/// When an [`Activity`] [started and ends](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-timestamps)
/// (both: a time bar for listening and watching).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityTimestamps {
    /// When the activity started.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_unix_millis"
    )]
    pub start: Option<Timestamp>,
    /// When the activity ends.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::serde_util::option_unix_millis"
    )]
    pub end: Option<Timestamp>,
}

/// The [emoji of a custom status](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-emoji).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityEmoji {
    /// The emoji's name (the character of a standard emoji).
    pub name: CompactString,
    /// The custom emoji's ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<EmojiId>,
    /// Whether the custom emoji is animated.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub animated: bool,
}

/// The player's [party](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-party).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityParty {
    /// The party's ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<CompactString>,
    /// The party's `(current, maximum)` size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<(u32, u32)>,
}

/// The [images of an activity](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-assets)
/// and their hover texts.
///
/// Images are an application asset ID or `mp:{image_id}` for an image of the media proxy
/// (`https://media.discordapp.net/{image_id}`); they are user-provided and not sanitized.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityAssets {
    /// The large image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_image: Option<CompactString>,
    /// The large image's hover text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_text: Option<CompactString>,
    /// The URL opened by clicking the large image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_url: Option<CompactString>,
    /// The small image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_image: Option<CompactString>,
    /// The small image's hover text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_text: Option<CompactString>,
    /// The URL opened by clicking the small image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_url: Option<CompactString>,
    /// The banner of a game invite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_cover_image: Option<CompactString>,
}

/// The [Rich Presence secrets](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-secrets)
/// of an activity.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivitySecrets {
    /// The secret for joining the party.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<CompactString>,
    /// The secret for spectating the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spectate: Option<CompactString>,
    /// The secret for an instanced match.
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_: Option<CompactString>,
}

impl core::fmt::Debug for ActivitySecrets {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let redact = |secret: &Option<CompactString>| secret.as_ref().map(|_| "<redacted>");
        f.debug_struct("ActivitySecrets")
            .field("join", &redact(&self.join))
            .field("spectate", &redact(&self.spectate))
            .field("match", &redact(&self.match_))
            .finish()
    }
}

int_flags! {
    /// [Flags of an activity](https://docs.discord.com/developers/events/gateway-events#activity-object-activity-flags):
    /// what the activity supports (joining, spectating, …).
    ///
    /// discord.js counterpart: `ActivityFlags` / `ActivityFlagsBitField`.
    pub struct ActivityFlags: u64 {
        /// The activity is an instanced game session.
        const INSTANCE = 1 << 0;
        /// Others can join the activity.
        const JOIN = 1 << 1;
        /// Others can spectate the activity.
        const SPECTATE = 1 << 2;
        /// Others can ask to join the activity.
        const JOIN_REQUEST = 1 << 3;
        /// A Spotify-style synced activity ("Listen along").
        const SYNC = 1 << 4;
        /// The activity can be played.
        const PLAY = 1 << 5;
        /// Party privacy: only friends can join.
        const PARTY_PRIVACY_FRIENDS = 1 << 6;
        /// Party privacy: members of the same voice channel can join.
        const PARTY_PRIVACY_VOICE_CHANNEL = 1 << 7;
        /// The activity is an embedded app (Activity) running in Discord.
        const EMBEDDED = 1 << 8;
    }
}

str_enum! {
    /// The [online status](https://docs.discord.com/developers/events/gateway-events#update-presence-status-types)
    /// of a user.
    ///
    /// discord.js counterpart: `PresenceStatus` / `PresenceUpdateStatus`.
    pub enum Status {
        /// Online.
        Online = "online",
        /// Do not disturb.
        DoNotDisturb = "dnd",
        /// Idle (AFK).
        Idle = "idle",
        /// Invisible: shown to others as offline. Only used when setting the bot's own status.
        Invisible = "invisible",
        /// Offline.
        Offline = "offline",
    }
}
