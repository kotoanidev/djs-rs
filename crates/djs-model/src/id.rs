//! Typed Discord [snowflake] IDs.
//!
//! Every Discord resource is identified by a 64-bit *snowflake*. djs-rs represents them with a
//! single generic type, [`Id<T>`], where `T` is a zero-sized **marker** describing what the ID
//! points at (see [`marker`]). Type aliases such as [`UserId`] and [`ChannelId`] make signatures
//! readable:
//!
//! ```
//! use djs_model::id::{ChannelId, UserId};
//!
//! fn send_to(channel: ChannelId) -> u64 {
//!     channel.get()
//! }
//!
//! let channel = ChannelId::new(1_234_567_890_123_456_789);
//! assert_eq!(send_to(channel), 1_234_567_890_123_456_789);
//!
//! let user = UserId::new(80_351_110_224_678_912);
//! // send_to(user); // ❌ does not compile: a user ID is not a channel ID
//! # let _ = user;
//! ```
//!
//! # Memory layout
//!
//! `Id<T>` wraps a [`NonZeroU64`] (Discord never issues the ID `0`), so both `Id<T>` and
//! `Option<Id<T>>` are exactly 8 bytes.
//!
//! # Serialization
//!
//! Discord transmits snowflakes as JSON **strings** (`"80351110224678912"`) to avoid precision
//! loss in JavaScript. `Id<T>` serializes to a string and deserializes from either a string or an
//! integer.
//!
//! discord.js counterpart: the `Snowflake` string type and `SnowflakeUtil`.
//!
//! [snowflake]: https://docs.discord.com/developers/reference#snowflakes

use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU64,
    str::FromStr,
};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, Unexpected, Visitor},
};

use crate::util::Timestamp;

/// The Discord epoch — the first second of 2015 — in milliseconds since the Unix epoch.
///
/// Snowflake timestamps are stored relative to this instant.
pub const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

/// A typed Discord snowflake ID.
///
/// `T` is a marker type from [`marker`] that prevents mixing different kinds of IDs at compile
/// time. Use the aliases in this module ([`UserId`], [`GuildId`], …) rather than spelling out
/// `Id<UserMarker>`.
///
/// IDs are ordered chronologically: an ID created later always compares greater.
///
/// # Examples
///
/// ```
/// use djs_model::id::GuildId;
///
/// let id: GuildId = "175928847299117063".parse().unwrap();
/// assert_eq!(id.get(), 175_928_847_299_117_063);
/// assert_eq!(id.to_string(), "175928847299117063");
///
/// // Snowflakes embed their creation time.
/// assert_eq!(id.timestamp_ms(), 1_462_015_105_796);
/// ```
pub struct Id<T> {
    value: NonZeroU64,
    // `fn() -> T` keeps `Id<T>` `Send + Sync + Unpin` regardless of `T` and avoids drop-check
    // implications: no `T` is ever stored.
    marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Creates an ID from a raw non-zero value.
    ///
    /// # Panics
    ///
    /// Panics if `value` is `0`. Use [`Id::new_checked`] for fallible construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::id::UserId;
    ///
    /// const BOT_OWNER: UserId = UserId::new(80_351_110_224_678_912);
    /// assert_eq!(BOT_OWNER.get(), 80_351_110_224_678_912);
    /// ```
    #[must_use]
    #[track_caller]
    pub const fn new(value: u64) -> Self {
        match Self::new_checked(value) {
            Some(id) => id,
            None => panic!("snowflake IDs must be non-zero"),
        }
    }

    /// Creates an ID from a raw value, returning `None` if it is `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::id::UserId;
    ///
    /// assert!(UserId::new_checked(0).is_none());
    /// assert!(UserId::new_checked(1).is_some());
    /// ```
    #[must_use]
    pub const fn new_checked(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self::from_nonzero(value)),
            None => None,
        }
    }

    /// Creates an ID from a [`NonZeroU64`]. This can never fail.
    #[must_use]
    pub const fn from_nonzero(value: NonZeroU64) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }

    /// Returns the raw value of the ID.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.value.get()
    }

    /// Returns the raw value of the ID as a [`NonZeroU64`].
    #[must_use]
    pub const fn into_nonzero(self) -> NonZeroU64 {
        self.value
    }

    /// Reinterprets this ID as an ID of another kind.
    ///
    /// Some Discord resources deliberately share IDs — for example the `@everyone` role of a
    /// guild has the same ID as the guild itself. `cast` makes such conversions explicit.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::id::{GuildId, RoleId};
    ///
    /// let guild = GuildId::new(175_928_847_299_117_063);
    /// let everyone: RoleId = guild.cast();
    /// assert_eq!(everyone.get(), guild.get());
    /// ```
    #[must_use]
    pub const fn cast<U>(self) -> Id<U> {
        Id::from_nonzero(self.value)
    }

    /// Returns the creation time encoded in the snowflake, in milliseconds since the Unix epoch.
    ///
    /// discord.js counterpart: `SnowflakeUtil.timestampFrom`.
    #[must_use]
    pub const fn timestamp_ms(self) -> u64 {
        (self.get() >> 22) + DISCORD_EPOCH_MS
    }

    /// Returns the creation time encoded in the snowflake.
    ///
    /// discord.js counterpart: the `createdAt` / `createdTimestamp` getters on structures.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::id::UserId;
    ///
    /// let id = UserId::new(80_351_110_224_678_912);
    /// assert_eq!(id.timestamp().to_string(), "2015-08-10T17:26:37.529000+00:00");
    /// ```
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "a snowflake timestamp is at most 2^42 ms after the Discord epoch, far below i64::MAX µs"
    )]
    pub const fn timestamp(self) -> Timestamp {
        Timestamp::from_unix_micros(self.timestamp_ms() as i64 * 1_000)
    }

    /// Returns the internal worker ID encoded in the snowflake (5 bits).
    #[must_use]
    pub const fn worker_id(self) -> u8 {
        ((self.get() & 0x003E_0000) >> 17) as u8
    }

    /// Returns the internal process ID encoded in the snowflake (5 bits).
    #[must_use]
    pub const fn process_id(self) -> u8 {
        ((self.get() & 0x0001_F000) >> 12) as u8
    }

    /// Returns the per-process increment encoded in the snowflake (12 bits).
    #[must_use]
    pub const fn increment(self) -> u16 {
        (self.get() & 0x0FFF) as u16
    }

    /// Creates the smallest ID that could have been generated at `unix_ms`
    /// (milliseconds since the Unix epoch).
    ///
    /// This is useful for paginating REST endpoints by time (`before` / `after` parameters).
    /// Returns `None` if `unix_ms` is at or before the Discord epoch (the ID would be `0`) or so
    /// far in the future that the timestamp does not fit in a snowflake.
    ///
    /// discord.js counterpart: `SnowflakeUtil.generate({ timestamp })`.
    ///
    /// # Examples
    ///
    /// ```
    /// use djs_model::id::MessageId;
    ///
    /// let id = MessageId::from_timestamp_ms(1_462_015_105_796).unwrap();
    /// assert_eq!(id.timestamp_ms(), 1_462_015_105_796);
    /// ```
    #[must_use]
    pub const fn from_timestamp_ms(unix_ms: u64) -> Option<Self> {
        if unix_ms <= DISCORD_EPOCH_MS {
            return None;
        }
        let since_epoch = unix_ms - DISCORD_EPOCH_MS;
        if since_epoch >= 1 << 42 {
            return None;
        }
        Self::new_checked(since_epoch << 22)
    }
}

// Manual impls: deriving would add unnecessary `T: Trait` bounds on the marker.

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash exactly like a `u64` so `Id<T>` can be looked up with hashers specialised for
        // integers.
        state.write_u64(self.get());
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marker = core::any::type_name::<T>();
        let marker = marker.rsplit("::").next().unwrap_or(marker);
        write!(f, "Id<{marker}>({})", self.value)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl<T> From<NonZeroU64> for Id<T> {
    fn from(value: NonZeroU64) -> Self {
        Self::from_nonzero(value)
    }
}

impl<T> From<Id<T>> for NonZeroU64 {
    fn from(id: Id<T>) -> Self {
        id.value
    }
}

impl<T> From<Id<T>> for u64 {
    fn from(id: Id<T>) -> Self {
        id.get()
    }
}

impl<T> TryFrom<u64> for Id<T> {
    type Error = ParseIdError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new_checked(value).ok_or(ParseIdError {
            kind: ParseIdErrorKind::Zero,
        })
    }
}

impl<T> FromStr for Id<T> {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u64 = s.parse().map_err(|_| ParseIdError {
            kind: ParseIdErrorKind::NotAnInteger,
        })?;
        Self::try_from(value)
    }
}

/// The error returned when parsing an [`Id`] from a string or integer fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseIdError {
    kind: ParseIdErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParseIdErrorKind {
    NotAnInteger,
    Zero,
}

impl ParseIdError {
    /// Returns `true` if parsing failed because the value was `0`.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.kind == ParseIdErrorKind::Zero
    }
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind {
            ParseIdErrorKind::NotAnInteger => "snowflake ID is not a valid unsigned 64-bit integer",
            ParseIdErrorKind::Zero => "snowflake ID must be non-zero",
        })
    }
}

impl core::error::Error for ParseIdError {}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `collect_str` lets serializers such as serde_json write the digits directly without an
        // intermediate `String` allocation.
        serializer.collect_str(&self.value)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(IdVisitor(PhantomData))
    }
}

struct IdVisitor<T>(PhantomData<fn() -> T>);

impl<T> Visitor<'_> for IdVisitor<T> {
    type Value = Id<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a non-zero snowflake ID as a string or integer")
    }

    fn visit_u64<E: DeError>(self, value: u64) -> Result<Self::Value, E> {
        Id::new_checked(value).ok_or_else(|| E::invalid_value(Unexpected::Unsigned(0), &self))
    }

    fn visit_i64<E: DeError>(self, value: i64) -> Result<Self::Value, E> {
        u64::try_from(value)
            .map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
            .and_then(|value| self.visit_u64(value))
    }

    fn visit_str<E: DeError>(self, value: &str) -> Result<Self::Value, E> {
        value
            .parse()
            .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
    }
}

/// Marker types for [`Id`].
///
/// Markers are uninhabited enums: they exist only at the type level and are never constructed.
pub mod marker {
    macro_rules! markers {
        ($($(#[$meta:meta])* $marker:ident => $alias:ident,)*) => {
            $(
                $(#[$meta])*
                #[doc = ""]
                #[doc = concat!("Usually used through the [`", stringify!($alias), "`](super::", stringify!($alias), ") alias.")]
                #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
                pub enum $marker {}
            )*

            /// Type aliases are generated next to the markers.
            pub(super) mod aliases {
                $(
                    #[doc = concat!("ID of a resource marked by [`", stringify!($marker), "`](super::", stringify!($marker), ").")]
                    pub type $alias = super::super::Id<super::$marker>;
                )*
            }
        };
    }

    markers! {
        /// Marker for [application](https://docs.discord.com/developers/resources/application) IDs.
        ApplicationMarker => ApplicationId,
        /// Marker for message [attachment](https://docs.discord.com/developers/resources/message#attachment-object) IDs.
        AttachmentMarker => AttachmentId,
        /// Marker for [audit log entry](https://docs.discord.com/developers/resources/audit-log) IDs.
        AuditLogEntryMarker => AuditLogEntryId,
        /// Marker for [auto moderation rule](https://docs.discord.com/developers/resources/auto-moderation) IDs.
        AutoModerationRuleMarker => AutoModerationRuleId,
        /// Marker for [channel](https://docs.discord.com/developers/resources/channel) IDs
        /// (including threads, which are channels).
        ChannelMarker => ChannelId,
        /// Marker for [application command](https://docs.discord.com/developers/interactions/application-commands) IDs.
        CommandMarker => CommandId,
        /// Marker for application command version IDs.
        CommandVersionMarker => CommandVersionId,
        /// Marker for [emoji](https://docs.discord.com/developers/resources/emoji) IDs.
        EmojiMarker => EmojiId,
        /// Marker for [entitlement](https://docs.discord.com/developers/resources/entitlement) IDs.
        EntitlementMarker => EntitlementId,
        /// Marker for IDs whose kind is not known or varies (for example audit log target IDs).
        GenericMarker => GenericId,
        /// Marker for [guild](https://docs.discord.com/developers/resources/guild) IDs.
        GuildMarker => GuildId,
        /// Marker for guild [integration](https://docs.discord.com/developers/resources/guild#integration-object) IDs.
        IntegrationMarker => IntegrationId,
        /// Marker for [interaction](https://docs.discord.com/developers/interactions/receiving-and-responding) IDs.
        InteractionMarker => InteractionId,
        /// Marker for [lobby](https://docs.discord.com/developers/resources/lobby) IDs.
        LobbyMarker => LobbyId,
        /// Marker for [message](https://docs.discord.com/developers/resources/message) IDs.
        MessageMarker => MessageId,
        /// Marker for [role](https://docs.discord.com/developers/topics/permissions#role-object) IDs.
        RoleMarker => RoleId,
        /// Marker for [guild scheduled event](https://docs.discord.com/developers/resources/guild-scheduled-event) IDs.
        ScheduledEventMarker => ScheduledEventId,
        /// Marker for [SKU](https://docs.discord.com/developers/resources/sku) IDs.
        SkuMarker => SkuId,
        /// Marker for [soundboard sound](https://docs.discord.com/developers/resources/soundboard) IDs.
        SoundboardSoundMarker => SoundboardSoundId,
        /// Marker for [stage instance](https://docs.discord.com/developers/resources/stage-instance) IDs.
        StageInstanceMarker => StageInstanceId,
        /// Marker for [sticker](https://docs.discord.com/developers/resources/sticker) IDs.
        StickerMarker => StickerId,
        /// Marker for sticker pack IDs.
        StickerPackMarker => StickerPackId,
        /// Marker for [subscription](https://docs.discord.com/developers/resources/subscription) IDs.
        SubscriptionMarker => SubscriptionId,
        /// Marker for forum/media channel [tag](https://docs.discord.com/developers/resources/channel#forum-tag-object) IDs.
        TagMarker => TagId,
        /// Marker for [team](https://docs.discord.com/developers/topics/teams) IDs.
        TeamMarker => TeamId,
        /// Marker for [user](https://docs.discord.com/developers/resources/user) IDs.
        UserMarker => UserId,
        /// Marker for [webhook](https://docs.discord.com/developers/resources/webhook) IDs.
        WebhookMarker => WebhookId,
    }
}

pub use marker::aliases::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout() {
        assert_eq!(size_of::<UserId>(), 8);
        assert_eq!(size_of::<Option<UserId>>(), 8);
    }

    #[test]
    fn snowflake_parts() {
        // Example snowflake from the Discord API documentation.
        let id = UserId::new(175_928_847_299_117_063);
        assert_eq!(id.timestamp_ms(), 1_462_015_105_796);
        assert_eq!(id.worker_id(), 1);
        assert_eq!(id.process_id(), 0);
        assert_eq!(id.increment(), 7);
    }

    #[test]
    fn from_timestamp_round_trip() {
        let id = MessageId::from_timestamp_ms(1_462_015_105_796).unwrap();
        assert_eq!(id.timestamp_ms(), 1_462_015_105_796);
        assert_eq!(id.increment(), 0);
        assert!(MessageId::from_timestamp_ms(DISCORD_EPOCH_MS).is_none());
        assert!(MessageId::from_timestamp_ms(0).is_none());
        assert!(MessageId::from_timestamp_ms(u64::MAX).is_none());
    }

    #[test]
    fn parse() {
        assert_eq!("1".parse::<GuildId>().unwrap().get(), 1);
        assert!("0".parse::<GuildId>().unwrap_err().is_zero());
        assert!(!"abc".parse::<GuildId>().unwrap_err().is_zero());
        assert!("-1".parse::<GuildId>().is_err());
        assert!("18446744073709551616".parse::<GuildId>().is_err());
    }

    #[test]
    fn debug_and_display() {
        let id = ChannelId::new(42);
        assert_eq!(format!("{id:?}"), "Id<ChannelMarker>(42)");
        assert_eq!(id.to_string(), "42");
    }

    #[test]
    fn ordering_is_chronological() {
        assert!(UserId::new(1) < UserId::new(2));
    }
}
