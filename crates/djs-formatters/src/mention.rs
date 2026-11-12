//! Mentions of users, channels, roles and commands, custom emoji, and guild navigation.
//!
//! See Discord's [message formats](https://docs.discord.com/developers/reference#message-formatting-formats).

use core::fmt;

use djs_model::id::{ChannelId, CommandId, EmojiId, RoleId, UserId};

/// Mentions a user: `<@USER_ID>`. Returned by [`user_mention`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct UserMention(UserId);

/// Mentions the user `id`: `<@80351110224678912>`.
///
/// Whether the user is notified depends on the message's `allowed_mentions`.
///
/// discord.js counterpart: `userMention`.
///
/// # Examples
///
/// ```
/// use djs_formatters::user_mention;
/// use djs_model::id::UserId;
///
/// let winner = UserId::new(80_351_110_224_678_912);
/// assert_eq!(format!("{} won!", user_mention(winner)), "<@80351110224678912> won!");
/// ```
pub const fn user_mention(id: UserId) -> UserMention {
    UserMention(id)
}

impl fmt::Display for UserMention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<@{}>", self.0)
    }
}

/// Mentions a channel: `<#CHANNEL_ID>`. Returned by [`channel_mention`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct ChannelMention(ChannelId);

/// Mentions the channel `id` as a clickable link: `<#103735883630395392>`.
///
/// discord.js counterpart: `channelMention`.
///
/// # Examples
///
/// ```
/// use djs_formatters::channel_mention;
/// use djs_model::id::ChannelId;
///
/// let rules = ChannelId::new(103_735_883_630_395_392);
/// assert_eq!(channel_mention(rules).to_string(), "<#103735883630395392>");
/// ```
pub const fn channel_mention(id: ChannelId) -> ChannelMention {
    ChannelMention(id)
}

impl fmt::Display for ChannelMention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<#{}>", self.0)
    }
}

/// Mentions a role: `<@&ROLE_ID>`. Returned by [`role_mention`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct RoleMention(RoleId);

/// Mentions the role `id`: `<@&165511591545143296>`.
///
/// Whether the role's members are notified depends on the message's `allowed_mentions` and
/// the role's settings.
///
/// discord.js counterpart: `roleMention`.
///
/// # Examples
///
/// ```
/// use djs_formatters::role_mention;
/// use djs_model::id::RoleId;
///
/// assert_eq!(role_mention(RoleId::new(165_511_591_545_143_296)).to_string(), "<@&165511591545143296>");
/// ```
pub const fn role_mention(id: RoleId) -> RoleMention {
    RoleMention(id)
}

impl fmt::Display for RoleMention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<@&{}>", self.0)
    }
}

/// Links to a linked role: `<id:linked-roles:ROLE_ID>`. Returned by [`linked_role_mention`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct LinkedRoleMention(RoleId);

/// Links to the [linked role](https://support.discord.com/hc/en-us/articles/10388356626711)
/// `id`; clicking it opens the connection dialog: `<id:linked-roles:165511591545143296>`.
///
/// discord.js counterpart: `linkedRoleMention`.
///
/// # Examples
///
/// ```
/// use djs_formatters::linked_role_mention;
/// use djs_model::id::RoleId;
///
/// assert_eq!(
///     linked_role_mention(RoleId::new(165_511_591_545_143_296)).to_string(),
///     "<id:linked-roles:165511591545143296>"
/// );
/// ```
pub const fn linked_role_mention(id: RoleId) -> LinkedRoleMention {
    LinkedRoleMention(id)
}

impl fmt::Display for LinkedRoleMention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<id:linked-roles:{}>", self.0)
    }
}

/// Mentions a slash command, a clickable link that fills in the command:
/// `</NAME:ID>`, `</NAME SUBCOMMAND:ID>` or `</NAME GROUP SUBCOMMAND:ID>`. Returned by
/// [`slash_command_mention`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct SlashCommandMention<'a> {
    name: &'a str,
    group: Option<&'a str>,
    subcommand: Option<&'a str>,
    id: CommandId,
}

/// Mentions the slash command `name` with the ID `id`: `</airhorn:816437322781949972>`.
///
/// A subcommand is added with [`subcommand`](SlashCommandMention::subcommand), a subcommand
/// in a group with [`group`](SlashCommandMention::group) (D-213).
///
/// discord.js counterpart: `chatInputApplicationCommandMention`.
///
/// # Examples
///
/// ```
/// use djs_formatters::slash_command_mention;
/// use djs_model::id::CommandId;
///
/// let id = CommandId::new(123_456_789_012_345_678);
/// assert_eq!(slash_command_mention("ping", id).to_string(), "</ping:123456789012345678>");
/// assert_eq!(
///     slash_command_mention("tag", id).subcommand("show").to_string(),
///     "</tag show:123456789012345678>"
/// );
/// assert_eq!(
///     slash_command_mention("admin", id).group("users", "ban").to_string(),
///     "</admin users ban:123456789012345678>"
/// );
/// ```
pub const fn slash_command_mention(name: &str, id: CommandId) -> SlashCommandMention<'_> {
    SlashCommandMention {
        name,
        group: None,
        subcommand: None,
        id,
    }
}

impl<'a> SlashCommandMention<'a> {
    /// Mentions the command's subcommand `subcommand`, outside a group.
    pub const fn subcommand(mut self, subcommand: &'a str) -> Self {
        self.group = None;
        self.subcommand = Some(subcommand);
        self
    }

    /// Mentions the subcommand `subcommand` of the command's group `group`.
    pub const fn group(mut self, group: &'a str, subcommand: &'a str) -> Self {
        self.group = Some(group);
        self.subcommand = Some(subcommand);
        self
    }
}

impl fmt::Display for SlashCommandMention<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("</")?;
        f.write_str(self.name)?;
        for part in [self.group, self.subcommand].into_iter().flatten() {
            f.write_str(" ")?;
            f.write_str(part)?;
        }
        write!(f, ":{}>", self.id)
    }
}

/// Shows a custom emoji: `<:NAME:ID>` or, animated, `<a:NAME:ID>`. Returned by [`emoji`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use = "a formatter does nothing until it is written"]
pub struct Emoji<'a> {
    id: EmojiId,
    name: Option<&'a str>,
    animated: bool,
}

/// Shows the custom emoji `id`: `<:emoji:216154654256398347>`.
///
/// Discord finds the emoji by its ID; the name only matters where the emoji cannot be shown,
/// so it defaults to `emoji`. Set it with [`name`](Emoji::name); an animated emoji needs
/// [`animated`](Emoji::animated) (D-213). A Unicode emoji is written as itself (`🪴`).
///
/// discord.js counterpart: `formatEmoji`.
///
/// # Examples
///
/// ```
/// use djs_formatters::emoji;
/// use djs_model::id::EmojiId;
///
/// let id = EmojiId::new(392_938_283_556_143_104);
/// assert_eq!(emoji(id).to_string(), "<:emoji:392938283556143104>");
/// assert_eq!(emoji(id).name("b1nzy").animated(true).to_string(), "<a:b1nzy:392938283556143104>");
/// ```
pub const fn emoji<'a>(id: EmojiId) -> Emoji<'a> {
    Emoji {
        id,
        name: None,
        animated: false,
    }
}

impl<'a> Emoji<'a> {
    /// Sets the emoji's name (default: `emoji`).
    pub const fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets whether the emoji is animated (default: no).
    pub const fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
}

impl fmt::Display for Emoji<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.animated { "<a:" } else { "<:" };
        write!(f, "{prefix}{}:{}>", self.name.unwrap_or("emoji"), self.id)
    }
}

/// A [guild navigation](https://docs.discord.com/developers/reference#message-formatting-guild-navigation-types)
/// link, opening a tab of the current server.
///
/// discord.js counterpart: `GuildNavigationMentions`.
///
/// # Examples
///
/// ```
/// use djs_formatters::GuildNavigation;
///
/// assert_eq!(format!("Pick your roles in {}", GuildNavigation::Customize), "Pick your roles in <id:customize>");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GuildNavigation {
    /// The Browse Channels tab: `<id:browse>`.
    Browse,
    /// The Channels & Roles tab with the onboarding prompts: `<id:customize>`.
    Customize,
    /// The Server Guide tab: `<id:guide>`.
    Guide,
    /// The Linked Roles tab: `<id:linked-roles>`. One linked role is linked with
    /// [`linked_role_mention`].
    LinkedRoles,
}

impl GuildNavigation {
    /// The link as written in a message, such as `<id:browse>`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Browse => "<id:browse>",
            Self::Customize => "<id:customize>",
            Self::Guide => "<id:guide>",
            Self::LinkedRoles => "<id:linked-roles>",
        }
    }
}

impl fmt::Display for GuildNavigation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
