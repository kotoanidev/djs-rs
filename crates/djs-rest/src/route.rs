//! Typed REST API routes.

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use djs_model::{
    emoji::{Emoji, PartialEmoji},
    id::{
        ApplicationId, ChannelId, CommandId, EmojiId, GenericId, GuildId, Id, IntegrationId,
        InteractionId, MessageId, RoleId, UserId,
    },
};
use http::Method;

/// How the requests of a route are rate limited.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Limits {
    /// Whether the route counts towards the bot's global rate limit.
    pub(crate) global: bool,
    /// Whether requests are sent at once instead of one after another per bucket
    /// (interaction callbacks; like discord.js' `BurstHandler`). A 429 then only delays the
    /// request that received it.
    pub(crate) burst: bool,
}

impl Limits {
    /// A normal route: one request at a time per bucket, counted globally.
    pub(crate) const NORMAL: Self = Self {
        global: true,
        burst: false,
    };
    /// [Interaction endpoints](https://docs.discord.com/developers/topics/rate-limits#global-rate-limit)
    /// are not bound to the global rate limit.
    const INTERACTION: Self = Self {
        global: false,
        burst: false,
    };
    /// Interaction callbacks must be answered within 3 seconds, so they never wait for one
    /// another.
    const INTERACTION_CALLBACK: Self = Self {
        global: false,
        burst: true,
    };
}

/// How a route is grouped for rate limiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BucketRoute {
    /// A unique, static description of the route *without* its parameters, e.g.
    /// `"POST /channels/{channel_id}/messages"`. Routes sharing a template share a bucket hash.
    pub(crate) template: &'static str,
    /// The [major parameter](https://docs.discord.com/developers/topics/rate-limits) (channel,
    /// guild or webhook ID, or webhook ID + token), or `0` if the route has none. Rate limits
    /// are tracked separately per major parameter.
    pub(crate) major: u64,
    /// How the route is limited.
    pub(crate) limits: Limits,
}

/// Messages older than this are deleted under a separate, stricter rate limit.
const OLD_MESSAGE_AGE_MS: u64 = 14 * 24 * 60 * 60 * 1000;

/// The bucket template of [`Route::DeleteMessage`] for messages older than two weeks.
const OLD_MESSAGE_DELETE: &str =
    "DELETE /channels/{channel_id}/messages/{message_id} (older than two weeks)";

/// The emoji of a reaction endpoint (D-103), written into the path percent-encoded.
///
/// Discord identifies a Unicode emoji by the emoji itself and a custom emoji by `name:id`;
/// this type writes the right form, so a wrong format is impossible.
///
/// discord.js counterpart: `EmojiIdentifierResolvable`.
///
/// # Examples
///
/// ```
/// use djs_model::id::{ChannelId, EmojiId, MessageId};
/// use djs_rest::{ReactionEmoji, Route};
///
/// let route = |emoji| Route::CreateReaction {
///     channel_id: ChannelId::new(1),
///     message_id: MessageId::new(2),
///     emoji,
/// };
/// assert_eq!(
///     route(ReactionEmoji::Unicode("👍")).path(),
///     "/channels/1/messages/2/reactions/%F0%9F%91%8D/@me"
/// );
/// assert_eq!(
///     route(ReactionEmoji::Custom { name: "ferris", id: EmojiId::new(3) }).path(),
///     "/channels/1/messages/2/reactions/ferris%3A3/@me"
/// );
/// // A `&str` is a Unicode emoji.
/// assert_eq!(ReactionEmoji::from("🔥"), ReactionEmoji::Unicode("🔥"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReactionEmoji<'a> {
    /// A standard Unicode emoji such as `👍`.
    Unicode(&'a str),
    /// A custom (guild or application) emoji.
    Custom {
        /// The emoji's name.
        name: &'a str,
        /// The emoji's ID.
        id: EmojiId,
    },
}

impl<'a> From<&'a str> for ReactionEmoji<'a> {
    /// A Unicode emoji.
    fn from(emoji: &'a str) -> Self {
        Self::Unicode(emoji)
    }
}

impl<'a> From<&'a Emoji> for ReactionEmoji<'a> {
    fn from(emoji: &'a Emoji) -> Self {
        Self::Custom {
            name: &emoji.name,
            id: emoji.id,
        }
    }
}

impl<'a> From<&'a PartialEmoji> for ReactionEmoji<'a> {
    /// A custom emoji whose name Discord no longer knows gets the name `_` (only the ID
    /// identifies it).
    fn from(emoji: &'a PartialEmoji) -> Self {
        match emoji {
            PartialEmoji::Custom { id, name, .. } => Self::Custom {
                name: name.as_deref().unwrap_or("_"),
                id: *id,
            },
            PartialEmoji::Unicode { name } => Self::Unicode(name),
        }
    }
}

/// A path parameter of a route.
#[derive(Clone, Copy)]
enum Segment<'a> {
    Id(u64),
    Text(&'a str),
    Emoji(ReactionEmoji<'a>),
}

/// A value that can appear in a route's path.
trait Param: Copy {
    /// The value as a path segment.
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a;

    /// The value's contribution to the major parameter.
    fn major(self) -> u64;

    /// Writes the value for `Debug`, redacting tokens.
    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>);
}

impl<M> Param for Id<M> {
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a,
    {
        Segment::Id(self.get())
    }

    fn major(self) -> u64 {
        self.get()
    }

    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>) {
        out.field(name, &self.get());
    }
}

impl Param for &str {
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a,
    {
        Segment::Text(self)
    }

    fn major(self) -> u64 {
        // FNV-1a: a stable, cheap hash, so that every token gets its own bucket.
        self.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>) {
        if name.ends_with("token") {
            out.field(name, &"<redacted>");
        } else {
            out.field(name, &self);
        }
    }
}

impl Param for ReactionEmoji<'_> {
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a,
    {
        Segment::Emoji(self)
    }

    fn major(self) -> u64 {
        // Never a major parameter: all emojis of a message share the reaction buckets.
        0
    }

    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>) {
        out.field(name, &self);
    }
}

/// Combines major parameter parts (webhook ID + token).
const fn combine(hash: u64, part: u64) -> u64 {
    (hash.rotate_left(29) ^ part).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// Writes `text` as one path segment, percent-encoding everything but unreserved characters
/// (RFC 3986), so a parameter can never change the structure of the path.
pub(crate) fn write_text(out: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0xf)]));
        }
    }
}

/// Writes `template` with every `{name}` replaced by the matching parameter.
fn write_template(out: &mut String, template: &str, params: &[(&str, Segment<'_>)]) {
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('}').unwrap_or(after.len());
        let name = &after[..close];
        match params.iter().find(|(param, _)| *param == name) {
            Some((_, Segment::Id(id))) => {
                // Writing to a `String` cannot fail.
                let _ = fmt::Write::write_fmt(out, format_args!("{id}"));
            }
            Some((_, Segment::Text(text))) => write_text(out, text),
            Some((_, Segment::Emoji(ReactionEmoji::Unicode(emoji)))) => write_text(out, emoji),
            Some((_, Segment::Emoji(ReactionEmoji::Custom { name, id }))) => {
                write_text(out, name);
                let _ = fmt::Write::write_fmt(out, format_args!("%3A{id}"));
            }
            None => debug_assert!(false, "unknown route parameter {{{name}}} in {template}"),
        }
        rest = after.get(close + 1..).unwrap_or("");
    }
    out.push_str(rest);
}

macro_rules! routes {
    ($(
        $(#[doc = $doc:literal])*
        $variant:ident $({ $( $(#[doc = $field_doc:literal])* $field:ident: $ty:ty ),* $(,)? })?
            => $method:ident $template:literal
            $(, major: $($major:ident)+)?
            $(, limits: $limits:ident)?;
    )*) => {
        /// A Discord REST API endpoint together with its path parameters.
        ///
        /// Each route knows its HTTP method, its path and how it is
        /// [rate limited](https://docs.discord.com/developers/topics/rate-limits). Most code uses
        /// the typed methods on [`Rest`](crate::Rest) instead of building routes directly;
        /// routes are useful with the low-level [`Rest::request`](crate::Rest::request).
        ///
        /// Tokens (interaction and webhook tokens) are borrowed (D-090), percent-encoded in the
        /// path and redacted in the `Debug` output.
        ///
        /// discord.js counterpart: the `Routes` helper of discord-api-types.
        ///
        /// # Examples
        ///
        /// ```
        /// use djs_model::id::ChannelId;
        /// use djs_rest::Route;
        ///
        /// let route = Route::CreateMessage { channel_id: ChannelId::new(42) };
        /// assert_eq!(route.method(), http::Method::POST);
        /// assert_eq!(route.path(), "/channels/42/messages");
        /// ```
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Route<'a> {
            $(
                $(#[doc = $doc])*
                #[doc = ""]
                #[doc = concat!("`", stringify!($method), " ", $template, "`")]
                $variant $({ $( $(#[doc = $field_doc])* $field: $ty ),* })?,
            )*
        }

        impl Route<'_> {
            /// Returns the HTTP method of the route.
            #[must_use]
            pub fn method(self) -> Method {
                match self {
                    $(Self::$variant { .. } => Method::$method,)*
                }
            }

            /// Returns the path of the route, relative to the versioned API base
            /// (e.g. `/channels/42/messages`).
            #[must_use]
            pub fn path(self) -> String {
                let mut path = String::with_capacity(64);
                self.write_path(&mut path);
                path
            }

            /// Appends the path of the route to `out`.
            pub(crate) fn write_path(self, out: &mut String) {
                match self {
                    $(Self::$variant $({ $($field),* })? => write_template(
                        out,
                        $template,
                        &[$($( (stringify!($field), Param::to_segment($field)) ),*)?],
                    ),)*
                }
            }

            /// Returns the rate-limit grouping of the route, before route-specific exceptions.
            fn base_bucket(self) -> BucketRoute {
                match self {
                    $(
                        #[allow(unused_variables, reason = "not every route has a major parameter")]
                        Self::$variant $({ $($field),* })? => BucketRoute {
                            template: concat!(stringify!($method), " ", $template),
                            major: 0 $($( ^ combine(0, Param::major($major)) )+)?,
                            limits: routes!(@limits $($limits)?),
                        },
                    )*
                }
            }
        }

        impl fmt::Debug for Route<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match *self {
                    $(
                        #[allow(unused_mut, reason = "unit routes have no fields")]
                        Self::$variant $({ $($field),* })? => {
                            let mut out = f.debug_struct(stringify!($variant));
                            $($( Param::debug($field, stringify!($field), &mut out); )*)?
                            out.finish()
                        }
                    )*
                }
            }
        }
    };
    (@limits) => { Limits::NORMAL };
    (@limits $limits:ident) => { Limits::$limits };
}

impl Route<'_> {
    /// Returns the rate-limit grouping of the route.
    pub(crate) fn bucket(self) -> BucketRoute {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            });
        self.bucket_at(now_ms)
    }

    /// [`bucket`](Self::bucket) at the Unix time `now_ms` (milliseconds).
    fn bucket_at(self, now_ms: u64) -> BucketRoute {
        let mut bucket = self.base_bucket();
        // Deleting a message older than two weeks is limited separately from newer ones
        // (https://github.com/discord/discord-api-docs/issues/1295; discord.js does the same).
        if let Self::DeleteMessage { message_id, .. } = self
            && now_ms.saturating_sub(message_id.timestamp_ms()) > OLD_MESSAGE_AGE_MS
        {
            bucket.template = OLD_MESSAGE_DELETE;
        }
        bucket
    }
}

routes! {
    /// [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot).
    GetGatewayBot => GET "/gateway/bot";

    // Guilds.

    /// [Get Guild](https://docs.discord.com/developers/resources/guild#get-guild).
    GetGuild {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}", major: guild_id;

    /// [Get Guild Preview](https://docs.discord.com/developers/resources/guild#get-guild-preview).
    GetGuildPreview {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/preview", major: guild_id;

    /// [Modify Guild](https://docs.discord.com/developers/resources/guild#modify-guild).
    ModifyGuild {
        /// The guild.
        guild_id: GuildId,
    } => PATCH "/guilds/{guild_id}", major: guild_id;

    /// [Get Guild Channels](https://docs.discord.com/developers/resources/guild#get-guild-channels).
    GetGuildChannels {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/channels", major: guild_id;

    /// [Create Guild Channel](https://docs.discord.com/developers/resources/guild#create-guild-channel).
    CreateGuildChannel {
        /// The guild.
        guild_id: GuildId,
    } => POST "/guilds/{guild_id}/channels", major: guild_id;

    /// [Modify Guild Channel Positions](https://docs.discord.com/developers/resources/guild#modify-guild-channel-positions).
    ModifyGuildChannelPositions {
        /// The guild.
        guild_id: GuildId,
    } => PATCH "/guilds/{guild_id}/channels", major: guild_id;

    /// [List Active Guild Threads](https://docs.discord.com/developers/resources/guild#list-active-guild-threads).
    ListActiveGuildThreads {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/threads/active", major: guild_id;

    /// [Get Guild Member](https://docs.discord.com/developers/resources/guild#get-guild-member).
    GetGuildMember {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => GET "/guilds/{guild_id}/members/{user_id}", major: guild_id;

    /// [List Guild Members](https://docs.discord.com/developers/resources/guild#list-guild-members).
    ListGuildMembers {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/members", major: guild_id;

    /// [Search Guild Members](https://docs.discord.com/developers/resources/guild#search-guild-members).
    SearchGuildMembers {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/members/search", major: guild_id;

    /// [Get Guild Member Permissions](https://docs.discord.com/developers/resources/guild#get-guild-member-permissions).
    GetGuildMemberPermissions {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => GET "/guilds/{guild_id}/permissions/member/{user_id}", major: guild_id;

    /// [Add Guild Member](https://docs.discord.com/developers/resources/guild#add-guild-member).
    AddGuildMember {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => PUT "/guilds/{guild_id}/members/{user_id}", major: guild_id;

    /// [Modify Guild Member](https://docs.discord.com/developers/resources/guild#modify-guild-member).
    ModifyGuildMember {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => PATCH "/guilds/{guild_id}/members/{user_id}", major: guild_id;

    /// [Modify Current Member](https://docs.discord.com/developers/resources/guild#modify-current-member).
    ModifyCurrentMember {
        /// The guild.
        guild_id: GuildId,
    } => PATCH "/guilds/{guild_id}/members/@me", major: guild_id;

    /// [Add Guild Member Role](https://docs.discord.com/developers/resources/guild#add-guild-member-role).
    AddGuildMemberRole {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
        /// The role.
        role_id: RoleId,
    } => PUT "/guilds/{guild_id}/members/{user_id}/roles/{role_id}", major: guild_id;

    /// [Remove Guild Member Role](https://docs.discord.com/developers/resources/guild#remove-guild-member-role).
    RemoveGuildMemberRole {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
        /// The role.
        role_id: RoleId,
    } => DELETE "/guilds/{guild_id}/members/{user_id}/roles/{role_id}", major: guild_id;

    /// [Remove Guild Member](https://docs.discord.com/developers/resources/guild#remove-guild-member).
    RemoveGuildMember {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => DELETE "/guilds/{guild_id}/members/{user_id}", major: guild_id;

    /// [Get Guild Bans](https://docs.discord.com/developers/resources/guild#get-guild-bans).
    GetGuildBans {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/bans", major: guild_id;

    /// [Get Guild Ban](https://docs.discord.com/developers/resources/guild#get-guild-ban).
    GetGuildBan {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => GET "/guilds/{guild_id}/bans/{user_id}", major: guild_id;

    /// [Create Guild Ban](https://docs.discord.com/developers/resources/guild#create-guild-ban).
    CreateGuildBan {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => PUT "/guilds/{guild_id}/bans/{user_id}", major: guild_id;

    /// [Remove Guild Ban](https://docs.discord.com/developers/resources/guild#remove-guild-ban).
    RemoveGuildBan {
        /// The guild.
        guild_id: GuildId,
        /// The user.
        user_id: UserId,
    } => DELETE "/guilds/{guild_id}/bans/{user_id}", major: guild_id;

    /// [Bulk Guild Ban](https://docs.discord.com/developers/resources/guild#bulk-guild-ban).
    BulkGuildBan {
        /// The guild.
        guild_id: GuildId,
    } => POST "/guilds/{guild_id}/bulk-ban", major: guild_id;

    /// [Get Guild Roles](https://docs.discord.com/developers/resources/guild#get-guild-roles).
    GetGuildRoles {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/roles", major: guild_id;

    /// [Get Guild Role](https://docs.discord.com/developers/resources/guild#get-guild-role).
    GetGuildRole {
        /// The guild.
        guild_id: GuildId,
        /// The role.
        role_id: RoleId,
    } => GET "/guilds/{guild_id}/roles/{role_id}", major: guild_id;

    /// [Get Guild Role Permissions](https://docs.discord.com/developers/resources/guild#get-guild-role-permissions).
    GetGuildRolePermissions {
        /// The guild.
        guild_id: GuildId,
        /// The role.
        role_id: RoleId,
    } => GET "/guilds/{guild_id}/permissions/role/{role_id}", major: guild_id;

    /// [Get Guild Role Member Counts](https://docs.discord.com/developers/resources/guild#get-guild-role-member-counts).
    GetGuildRoleMemberCounts {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/roles/member-counts", major: guild_id;

    /// [Create Guild Role](https://docs.discord.com/developers/resources/guild#create-guild-role).
    CreateGuildRole {
        /// The guild.
        guild_id: GuildId,
    } => POST "/guilds/{guild_id}/roles", major: guild_id;

    /// [Modify Guild Role Positions](https://docs.discord.com/developers/resources/guild#modify-guild-role-positions).
    ModifyGuildRolePositions {
        /// The guild.
        guild_id: GuildId,
    } => PATCH "/guilds/{guild_id}/roles", major: guild_id;

    /// [Modify Guild Role](https://docs.discord.com/developers/resources/guild#modify-guild-role).
    ModifyGuildRole {
        /// The guild.
        guild_id: GuildId,
        /// The role.
        role_id: RoleId,
    } => PATCH "/guilds/{guild_id}/roles/{role_id}", major: guild_id;

    /// [Delete Guild Role](https://docs.discord.com/developers/resources/guild#delete-guild-role).
    DeleteGuildRole {
        /// The guild.
        guild_id: GuildId,
        /// The role.
        role_id: RoleId,
    } => DELETE "/guilds/{guild_id}/roles/{role_id}", major: guild_id;

    /// [Get Guild Prune Count](https://docs.discord.com/developers/resources/guild#get-guild-prune-count).
    GetGuildPruneCount {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/prune", major: guild_id;

    /// [Begin Guild Prune](https://docs.discord.com/developers/resources/guild#begin-guild-prune).
    BeginGuildPrune {
        /// The guild.
        guild_id: GuildId,
    } => POST "/guilds/{guild_id}/prune", major: guild_id;

    /// [Get Guild Voice Regions](https://docs.discord.com/developers/resources/guild#get-guild-voice-regions).
    GetGuildVoiceRegions {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/regions", major: guild_id;

    /// [Get Guild Invites](https://docs.discord.com/developers/resources/guild#get-guild-invites).
    GetGuildInvites {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/invites", major: guild_id;

    /// [Get Guild Integrations](https://docs.discord.com/developers/resources/guild#get-guild-integrations).
    GetGuildIntegrations {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/integrations", major: guild_id;

    /// [Delete Guild Integration](https://docs.discord.com/developers/resources/guild#delete-guild-integration).
    DeleteGuildIntegration {
        /// The guild.
        guild_id: GuildId,
        /// The integration.
        integration_id: IntegrationId,
    } => DELETE "/guilds/{guild_id}/integrations/{integration_id}", major: guild_id;

    /// [Get Guild Widget Settings](https://docs.discord.com/developers/resources/guild#get-guild-widget-settings).
    GetGuildWidgetSettings {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/widget", major: guild_id;

    /// [Modify Guild Widget](https://docs.discord.com/developers/resources/guild#modify-guild-widget).
    ModifyGuildWidget {
        /// The guild.
        guild_id: GuildId,
    } => PATCH "/guilds/{guild_id}/widget", major: guild_id;

    /// [Get Guild Widget](https://docs.discord.com/developers/resources/guild#get-guild-widget).
    GetGuildWidget {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/widget.json", major: guild_id;

    /// [Get Guild Vanity URL](https://docs.discord.com/developers/resources/guild#get-guild-vanity-url).
    GetGuildVanityUrl {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/vanity-url", major: guild_id;

    /// [Get Guild Welcome Screen](https://docs.discord.com/developers/resources/guild#get-guild-welcome-screen).
    GetGuildWelcomeScreen {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/welcome-screen", major: guild_id;

    /// [Modify Guild Welcome Screen](https://docs.discord.com/developers/resources/guild#modify-guild-welcome-screen).
    ModifyGuildWelcomeScreen {
        /// The guild.
        guild_id: GuildId,
    } => PATCH "/guilds/{guild_id}/welcome-screen", major: guild_id;

    /// [Get Guild Onboarding](https://docs.discord.com/developers/resources/guild#get-guild-onboarding).
    GetGuildOnboarding {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/onboarding", major: guild_id;

    /// [Modify Guild Onboarding](https://docs.discord.com/developers/resources/guild#modify-guild-onboarding).
    ModifyGuildOnboarding {
        /// The guild.
        guild_id: GuildId,
    } => PUT "/guilds/{guild_id}/onboarding", major: guild_id;

    /// [Modify Guild Incident Actions](https://docs.discord.com/developers/resources/guild#modify-guild-incident-actions).
    ModifyGuildIncidentActions {
        /// The guild.
        guild_id: GuildId,
    } => PUT "/guilds/{guild_id}/incident-actions", major: guild_id;

    // Channels and threads.

    /// [Get Channel](https://docs.discord.com/developers/resources/channel#get-channel).
    GetChannel {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}", major: channel_id;

    /// [Modify Channel](https://docs.discord.com/developers/resources/channel#modify-channel).
    ModifyChannel {
        /// The channel.
        channel_id: ChannelId,
    } => PATCH "/channels/{channel_id}", major: channel_id;

    /// [Delete/Close Channel](https://docs.discord.com/developers/resources/channel#deleteclose-channel).
    DeleteChannel {
        /// The channel.
        channel_id: ChannelId,
    } => DELETE "/channels/{channel_id}", major: channel_id;

    /// [Get Channel Member Permissions](https://docs.discord.com/developers/resources/channel#get-channel-member-permissions).
    GetChannelMemberPermissions {
        /// The channel.
        channel_id: ChannelId,
        /// The user.
        user_id: UserId,
    } => GET "/channels/{channel_id}/permissions/member/{user_id}", major: channel_id;

    /// [Get Channel Role Permissions](https://docs.discord.com/developers/resources/channel#get-channel-role-permissions).
    GetChannelRolePermissions {
        /// The channel.
        channel_id: ChannelId,
        /// The role.
        role_id: RoleId,
    } => GET "/channels/{channel_id}/permissions/role/{role_id}", major: channel_id;

    /// [Set Voice Channel Status](https://docs.discord.com/developers/resources/channel#set-voice-channel-status).
    SetVoiceChannelStatus {
        /// The channel.
        channel_id: ChannelId,
    } => PUT "/channels/{channel_id}/voice-status", major: channel_id;

    /// [Edit Channel Permissions](https://docs.discord.com/developers/resources/channel#edit-channel-permissions).
    EditChannelPermissions {
        /// The channel.
        channel_id: ChannelId,
        /// The role or user of the overwrite.
        overwrite_id: GenericId,
    } => PUT "/channels/{channel_id}/permissions/{overwrite_id}", major: channel_id;

    /// [Delete Channel Permission](https://docs.discord.com/developers/resources/channel#delete-channel-permission).
    DeleteChannelPermission {
        /// The channel.
        channel_id: ChannelId,
        /// The role or user of the overwrite.
        overwrite_id: GenericId,
    } => DELETE "/channels/{channel_id}/permissions/{overwrite_id}", major: channel_id;

    /// [Get Channel Invites](https://docs.discord.com/developers/resources/channel#get-channel-invites).
    GetChannelInvites {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/invites", major: channel_id;

    /// [Create Channel Invite](https://docs.discord.com/developers/resources/channel#create-channel-invite).
    CreateChannelInvite {
        /// The channel.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/invites", major: channel_id;

    /// [Follow Announcement Channel](https://docs.discord.com/developers/resources/channel#follow-announcement-channel).
    FollowAnnouncementChannel {
        /// The channel.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/followers", major: channel_id;

    /// [Trigger Typing Indicator](https://docs.discord.com/developers/resources/channel#trigger-typing-indicator).
    TriggerTypingIndicator {
        /// The channel.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/typing", major: channel_id;

    /// [Group DM Add Recipient](https://docs.discord.com/developers/resources/channel#group-dm-add-recipient).
    GroupDmAddRecipient {
        /// The channel.
        channel_id: ChannelId,
        /// The user.
        user_id: UserId,
    } => PUT "/channels/{channel_id}/recipients/{user_id}", major: channel_id;

    /// [Group DM Remove Recipient](https://docs.discord.com/developers/resources/channel#group-dm-remove-recipient).
    GroupDmRemoveRecipient {
        /// The channel.
        channel_id: ChannelId,
        /// The user.
        user_id: UserId,
    } => DELETE "/channels/{channel_id}/recipients/{user_id}", major: channel_id;

    /// [Start Thread from Message](https://docs.discord.com/developers/resources/channel#start-thread-from-message).
    StartThreadFromMessage {
        /// The channel.
        channel_id: ChannelId,
        /// The message the thread starts from.
        message_id: MessageId,
    } => POST "/channels/{channel_id}/messages/{message_id}/threads", major: channel_id;

    /// [Start Thread without Message](https://docs.discord.com/developers/resources/channel#start-thread-without-message).
    StartThreadWithoutMessage {
        /// The channel.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/threads", major: channel_id;

    /// [Start Thread in Forum or Media Channel](https://docs.discord.com/developers/resources/channel#start-thread-in-forum-or-media-channel):
    /// the same endpoint as [`StartThreadWithoutMessage`](Self::StartThreadWithoutMessage), with
    /// a message body.
    StartThreadInForumOrMediaChannel {
        /// The channel.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/threads", major: channel_id;

    /// [Join Thread](https://docs.discord.com/developers/resources/channel#join-thread).
    JoinThread {
        /// The thread.
        channel_id: ChannelId,
    } => PUT "/channels/{channel_id}/thread-members/@me", major: channel_id;

    /// [Add Thread Member](https://docs.discord.com/developers/resources/channel#add-thread-member).
    AddThreadMember {
        /// The thread.
        channel_id: ChannelId,
        /// The user.
        user_id: UserId,
    } => PUT "/channels/{channel_id}/thread-members/{user_id}", major: channel_id;

    /// [Leave Thread](https://docs.discord.com/developers/resources/channel#leave-thread).
    LeaveThread {
        /// The thread.
        channel_id: ChannelId,
    } => DELETE "/channels/{channel_id}/thread-members/@me", major: channel_id;

    /// [Remove Thread Member](https://docs.discord.com/developers/resources/channel#remove-thread-member).
    RemoveThreadMember {
        /// The thread.
        channel_id: ChannelId,
        /// The user.
        user_id: UserId,
    } => DELETE "/channels/{channel_id}/thread-members/{user_id}", major: channel_id;

    /// [Get Thread Member](https://docs.discord.com/developers/resources/channel#get-thread-member).
    GetThreadMember {
        /// The thread.
        channel_id: ChannelId,
        /// The user.
        user_id: UserId,
    } => GET "/channels/{channel_id}/thread-members/{user_id}", major: channel_id;

    /// [List Thread Members](https://docs.discord.com/developers/resources/channel#list-thread-members).
    ListThreadMembers {
        /// The thread.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/thread-members", major: channel_id;

    /// [List Public Archived Threads](https://docs.discord.com/developers/resources/channel#list-public-archived-threads).
    ListPublicArchivedThreads {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/threads/archived/public", major: channel_id;

    /// [List Private Archived Threads](https://docs.discord.com/developers/resources/channel#list-private-archived-threads).
    ListPrivateArchivedThreads {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/threads/archived/private", major: channel_id;

    /// [List Joined Private Archived Threads](https://docs.discord.com/developers/resources/channel#list-joined-private-archived-threads).
    ListJoinedPrivateArchivedThreads {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/users/@me/threads/archived/private", major: channel_id;

    // Messages.

    /// [Get Channel Messages](https://docs.discord.com/developers/resources/message#get-channel-messages).
    GetChannelMessages {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/messages", major: channel_id;

    /// [Search Guild Messages](https://docs.discord.com/developers/resources/message#search-guild-messages).
    SearchGuildMessages {
        /// The guild.
        guild_id: GuildId,
    } => GET "/guilds/{guild_id}/messages/search", major: guild_id;

    /// [Get Channel Message](https://docs.discord.com/developers/resources/message#get-channel-message).
    GetChannelMessage {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
    } => GET "/channels/{channel_id}/messages/{message_id}", major: channel_id;

    /// [Create Message](https://docs.discord.com/developers/resources/message#create-message).
    CreateMessage {
        /// The channel to send the message in.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/messages", major: channel_id;

    /// [Crosspost Message](https://docs.discord.com/developers/resources/message#crosspost-message).
    CrosspostMessage {
        /// The announcement channel.
        channel_id: ChannelId,
        /// The message to publish.
        message_id: MessageId,
    } => POST "/channels/{channel_id}/messages/{message_id}/crosspost", major: channel_id;

    /// [Create Reaction](https://docs.discord.com/developers/resources/message#create-reaction).
    CreateReaction {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
        /// The emoji.
        emoji: ReactionEmoji<'a>,
    } => PUT "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me",
        major: channel_id;

    /// [Delete Own Reaction](https://docs.discord.com/developers/resources/message#delete-own-reaction).
    DeleteOwnReaction {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
        /// The emoji.
        emoji: ReactionEmoji<'a>,
    } => DELETE "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me",
        major: channel_id;

    /// [Delete User Reaction](https://docs.discord.com/developers/resources/message#delete-user-reaction).
    DeleteUserReaction {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
        /// The emoji.
        emoji: ReactionEmoji<'a>,
        /// The user whose reaction is removed.
        user_id: UserId,
    } => DELETE "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/{user_id}",
        major: channel_id;

    /// [Get Reactions](https://docs.discord.com/developers/resources/message#get-reactions).
    GetReactions {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
        /// The emoji.
        emoji: ReactionEmoji<'a>,
    } => GET "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}",
        major: channel_id;

    /// [Delete All Reactions](https://docs.discord.com/developers/resources/message#delete-all-reactions).
    DeleteAllReactions {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
    } => DELETE "/channels/{channel_id}/messages/{message_id}/reactions", major: channel_id;

    /// [Delete All Reactions for Emoji](https://docs.discord.com/developers/resources/message#delete-all-reactions-for-emoji).
    DeleteAllReactionsForEmoji {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
        /// The emoji.
        emoji: ReactionEmoji<'a>,
    } => DELETE "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}",
        major: channel_id;

    /// [Edit Message](https://docs.discord.com/developers/resources/message#edit-message).
    EditMessage {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
    } => PATCH "/channels/{channel_id}/messages/{message_id}", major: channel_id;

    /// [Delete Message](https://docs.discord.com/developers/resources/message#delete-message).
    /// Messages older than two weeks are deleted under a separate rate limit.
    DeleteMessage {
        /// The channel of the message.
        channel_id: ChannelId,
        /// The message to delete.
        message_id: MessageId,
    } => DELETE "/channels/{channel_id}/messages/{message_id}", major: channel_id;

    /// [Bulk Delete Messages](https://docs.discord.com/developers/resources/message#bulk-delete-messages).
    BulkDeleteMessages {
        /// The channel.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/messages/bulk-delete", major: channel_id;

    /// [Get Channel Pins](https://docs.discord.com/developers/resources/message#get-channel-pins).
    GetChannelPins {
        /// The channel.
        channel_id: ChannelId,
    } => GET "/channels/{channel_id}/messages/pins", major: channel_id;

    /// [Pin Message](https://docs.discord.com/developers/resources/message#pin-message).
    PinMessage {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
    } => PUT "/channels/{channel_id}/messages/pins/{message_id}", major: channel_id;

    /// [Unpin Message](https://docs.discord.com/developers/resources/message#unpin-message).
    UnpinMessage {
        /// The channel.
        channel_id: ChannelId,
        /// The message.
        message_id: MessageId,
    } => DELETE "/channels/{channel_id}/messages/pins/{message_id}", major: channel_id;

    /// [Create Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#create-interaction-response):
    /// the initial response to an interaction (within 3 seconds). Not bound to the global rate
    /// limit, and never queued behind other callbacks.
    CreateInteractionResponse {
        /// The interaction.
        interaction_id: InteractionId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => POST "/interactions/{interaction_id}/{interaction_token}/callback",
        limits: INTERACTION_CALLBACK;

    /// [Get Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#get-original-interaction-response).
    GetOriginalInteractionResponse {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => GET "/webhooks/{application_id}/{interaction_token}/messages/@original",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Edit Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#edit-original-interaction-response).
    EditOriginalInteractionResponse {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => PATCH "/webhooks/{application_id}/{interaction_token}/messages/@original",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Delete Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#delete-original-interaction-response).
    DeleteOriginalInteractionResponse {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => DELETE "/webhooks/{application_id}/{interaction_token}/messages/@original",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Create Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#create-followup-message).
    CreateFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => POST "/webhooks/{application_id}/{interaction_token}",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Get Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#get-followup-message).
    GetFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
        /// The follow-up message.
        message_id: MessageId,
    } => GET "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Edit Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#edit-followup-message).
    EditFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
        /// The follow-up message.
        message_id: MessageId,
    } => PATCH "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Delete Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#delete-followup-message).
    DeleteFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
        /// The follow-up message.
        message_id: MessageId,
    } => DELETE "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        major: application_id interaction_token, limits: INTERACTION;

    // Application commands. The guild ID is not a major parameter here: Discord's major
    // parameters only lead a path (`/guilds/{guild_id}/...`), as discord.js also assumes.

    /// [Get Global Application Commands](https://docs.discord.com/developers/interactions/application-commands#get-global-application-commands).
    GetGlobalApplicationCommands {
        /// The application.
        application_id: ApplicationId,
    } => GET "/applications/{application_id}/commands";

    /// [Create Global Application Command](https://docs.discord.com/developers/interactions/application-commands#create-global-application-command).
    CreateGlobalApplicationCommand {
        /// The application.
        application_id: ApplicationId,
    } => POST "/applications/{application_id}/commands";

    /// [Get Global Application Command](https://docs.discord.com/developers/interactions/application-commands#get-global-application-command).
    GetGlobalApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The command.
        command_id: CommandId,
    } => GET "/applications/{application_id}/commands/{command_id}";

    /// [Edit Global Application Command](https://docs.discord.com/developers/interactions/application-commands#edit-global-application-command).
    EditGlobalApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The command.
        command_id: CommandId,
    } => PATCH "/applications/{application_id}/commands/{command_id}";

    /// [Delete Global Application Command](https://docs.discord.com/developers/interactions/application-commands#delete-global-application-command).
    DeleteGlobalApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The command.
        command_id: CommandId,
    } => DELETE "/applications/{application_id}/commands/{command_id}";

    /// [Bulk Overwrite Global Application Commands](https://docs.discord.com/developers/interactions/application-commands#bulk-overwrite-global-application-commands).
    BulkOverwriteGlobalApplicationCommands {
        /// The application.
        application_id: ApplicationId,
    } => PUT "/applications/{application_id}/commands";

    /// [Get Guild Application Commands](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-commands).
    GetGuildApplicationCommands {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
    } => GET "/applications/{application_id}/guilds/{guild_id}/commands";

    /// [Create Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#create-guild-application-command).
    CreateGuildApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
    } => POST "/applications/{application_id}/guilds/{guild_id}/commands";

    /// [Get Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-command).
    GetGuildApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
        /// The command.
        command_id: CommandId,
    } => GET "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}";

    /// [Edit Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#edit-guild-application-command).
    EditGuildApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
        /// The command.
        command_id: CommandId,
    } => PATCH "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}";

    /// [Delete Guild Application Command](https://docs.discord.com/developers/interactions/application-commands#delete-guild-application-command).
    DeleteGuildApplicationCommand {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
        /// The command.
        command_id: CommandId,
    } => DELETE "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}";

    /// [Bulk Overwrite Guild Application Commands](https://docs.discord.com/developers/interactions/application-commands#bulk-overwrite-guild-application-commands).
    BulkOverwriteGuildApplicationCommands {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
    } => PUT "/applications/{application_id}/guilds/{guild_id}/commands";

    /// [Get Guild Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#get-guild-application-command-permissions).
    GetGuildApplicationCommandPermissions {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
    } => GET "/applications/{application_id}/guilds/{guild_id}/commands/permissions";

    /// [Get Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#get-application-command-permissions).
    GetApplicationCommandPermissions {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
        /// The command.
        command_id: CommandId,
    } => GET "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions";

    /// [Edit Application Command Permissions](https://docs.discord.com/developers/interactions/application-commands#edit-application-command-permissions):
    /// needs a Bearer token of a user who can manage the guild and the command.
    EditApplicationCommandPermissions {
        /// The application.
        application_id: ApplicationId,
        /// The guild.
        guild_id: GuildId,
        /// The command.
        command_id: CommandId,
    } => PUT "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions";
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "aW50ZXJhY3Rpb24";

    #[test]
    fn paths() {
        assert_eq!(Route::GetGatewayBot.path(), "/gateway/bot");
        assert_eq!(
            Route::CreateMessage {
                channel_id: ChannelId::new(1)
            }
            .path(),
            "/channels/1/messages"
        );
        assert_eq!(
            Route::CreateInteractionResponse {
                interaction_id: InteractionId::new(2),
                interaction_token: TOKEN,
            }
            .path(),
            "/interactions/2/aW50ZXJhY3Rpb24/callback"
        );
        assert_eq!(
            Route::EditFollowupMessage {
                application_id: ApplicationId::new(3),
                interaction_token: TOKEN,
                message_id: MessageId::new(4),
            }
            .path(),
            "/webhooks/3/aW50ZXJhY3Rpb24/messages/4"
        );
    }

    #[test]
    fn text_parameters_cannot_change_the_path() {
        let route = Route::CreateFollowupMessage {
            application_id: ApplicationId::new(3),
            interaction_token: "a/../b?c=d#e f%",
        };
        assert_eq!(route.path(), "/webhooks/3/a%2F..%2Fb%3Fc%3Dd%23e%20f%25");
    }

    #[test]
    fn debug_redacts_tokens() {
        let route = Route::GetFollowupMessage {
            application_id: ApplicationId::new(3),
            interaction_token: TOKEN,
            message_id: MessageId::new(4),
        };
        let debug = format!("{route:?}");
        assert_eq!(
            debug,
            r#"GetFollowupMessage { application_id: 3, interaction_token: "<redacted>", message_id: 4 }"#
        );
        assert_eq!(format!("{:?}", Route::GetGatewayBot), "GetGatewayBot");
    }

    #[test]
    fn major_parameter() {
        let a = Route::CreateMessage {
            channel_id: ChannelId::new(1),
        }
        .bucket();
        let b = Route::CreateMessage {
            channel_id: ChannelId::new(2),
        }
        .bucket();
        assert_eq!(a.template, b.template);
        assert_ne!(a.major, b.major);
        assert_eq!(a.template, "POST /channels/{channel_id}/messages");
        assert_eq!(Route::GetGatewayBot.bucket().major, 0);

        // Webhook routes are limited per webhook ID + token.
        let followup = |token| {
            Route::CreateFollowupMessage {
                application_id: ApplicationId::new(3),
                interaction_token: token,
            }
            .bucket()
        };
        assert_ne!(followup("a").major, followup("b").major);
        assert_eq!(followup("a").major, followup("a").major);

        // A guild ID inside an application path is not a major parameter.
        let guild_command = |guild| {
            Route::CreateGuildApplicationCommand {
                application_id: ApplicationId::new(3),
                guild_id: GuildId::new(guild),
            }
            .bucket()
        };
        assert_eq!(guild_command(4), guild_command(5));
        assert_eq!(guild_command(4).major, 0);
    }

    #[test]
    fn old_message_deletes_have_their_own_bucket() {
        let created_ms = 1_794_398_400_000;
        let route = Route::DeleteMessage {
            channel_id: ChannelId::new(1),
            message_id: MessageId::from_timestamp_ms(created_ms).unwrap(),
        };
        assert_eq!(
            route.path(),
            format!(
                "/channels/1/messages/{}",
                MessageId::from_timestamp_ms(created_ms).unwrap()
            )
        );
        let new = route.bucket_at(created_ms + OLD_MESSAGE_AGE_MS);
        assert_eq!(
            new.template,
            "DELETE /channels/{channel_id}/messages/{message_id}"
        );
        let old = route.bucket_at(created_ms + OLD_MESSAGE_AGE_MS + 1);
        assert_eq!(old.template, OLD_MESSAGE_DELETE);
        // Same channel: the major parameter does not change.
        assert_eq!((old.major, old.limits), (new.major, new.limits));
        // A clock before the message (or before 1970) never makes it old.
        assert_eq!(route.bucket_at(0).template, new.template);
        // With the real clock, a message from 2015 is old.
        let ancient = Route::DeleteMessage {
            channel_id: ChannelId::new(1),
            message_id: MessageId::new(1 << 22),
        };
        assert_eq!(ancient.bucket().template, OLD_MESSAGE_DELETE);
    }

    #[test]
    fn interaction_routes_are_not_global() {
        let callback = Route::CreateInteractionResponse {
            interaction_id: InteractionId::new(2),
            interaction_token: TOKEN,
        }
        .bucket();
        assert_eq!(callback.limits, Limits::INTERACTION_CALLBACK);
        assert!(!callback.limits.global && callback.limits.burst);
        let original = Route::GetOriginalInteractionResponse {
            application_id: ApplicationId::new(3),
            interaction_token: TOKEN,
        }
        .bucket();
        assert!(!original.limits.global && !original.limits.burst);
        assert!(Route::GetGatewayBot.bucket().limits.global);
    }
}
