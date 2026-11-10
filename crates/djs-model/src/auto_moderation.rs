//! [Auto Moderation](https://docs.discord.com/developers/resources/auto-moderation): rules that
//! block, flag or time out content automatically.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{AutoModerationRuleId, ChannelId, GuildId, RoleId, UserId},
    macros::int_enum,
};

/// An [Auto Moderation rule](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-rule-object).
///
/// discord.js counterpart: `AutoModerationRule`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "AutoModerationRule")]
pub struct Rule {
    /// The rule's ID.
    pub id: AutoModerationRuleId,
    /// The guild the rule belongs to.
    pub guild_id: GuildId,
    /// The rule's name.
    pub name: CompactString,
    /// Who created the rule.
    pub creator_id: UserId,
    /// What the rule checks: sent messages or member profiles.
    pub event_type: EventType,
    /// How the rule detects violations.
    pub trigger_type: TriggerType,
    /// The settings of the trigger.
    pub trigger_metadata: TriggerMetadata,
    /// What happens when the rule triggers.
    pub actions: Vec<Action>,
    /// Whether the rule is enabled.
    pub enabled: bool,
    /// Roles the rule does not apply to (up to 20).
    pub exempt_roles: Vec<RoleId>,
    /// Channels the rule does not apply to (up to 50).
    pub exempt_channels: Vec<ChannelId>,
}

int_enum! {
    /// [How a rule detects violations](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-rule-object-trigger-types).
    ///
    /// discord.js counterpart: `AutoModerationRuleTriggerType`.
    #[doc(alias = "AutoModerationRuleTriggerType")]
    pub enum TriggerType: u8 {
        /// Content contains words from a list of keywords (up to 6 rules per guild).
        Keyword = 1,
        /// Content is generic spam (1 rule per guild).
        Spam = 3,
        /// Content contains words from Discord's predefined word lists (1 rule per guild).
        KeywordPreset = 4,
        /// Content mentions more unique users and roles than allowed (1 rule per guild).
        MentionSpam = 5,
        /// A member profile contains words from a list of keywords (1 rule per guild).
        MemberProfile = 6,
    }
}

/// The [settings of a rule's trigger](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-rule-object-trigger-metadata).
///
/// Which fields apply depends on the [`TriggerType`]: keywords, regexes and the allow list for
/// `Keyword` and `MemberProfile`; presets and the allow list for `KeywordPreset`; the mention
/// settings for `MentionSpam`.
///
/// discord.js counterpart: `AutoModerationTriggerMetadata`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TriggerMetadata {
    /// Substrings to search for (up to 1000, 60 characters each). Supports `*` wildcards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyword_filter: Vec<CompactString>,
    /// Rust-flavored regular expressions to match (up to 10, 260 characters each).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regex_patterns: Vec<CompactString>,
    /// Discord's predefined word lists to search for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<KeywordPresetType>,
    /// Substrings that never trigger the rule (up to 100, or 1000 for `KeywordPreset`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_list: Vec<CompactString>,
    /// How many unique user and role mentions a message may have (up to 50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention_total_limit: Option<u8>,
    /// Whether mention raids are detected automatically.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub mention_raid_protection_enabled: bool,
}

int_enum! {
    /// A [predefined word list](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-rule-object-keyword-preset-types).
    ///
    /// discord.js counterpart: `AutoModerationRuleKeywordPresetType`.
    pub enum KeywordPresetType: u8 {
        /// Swearing and cursing.
        Profanity = 1,
        /// Sexually explicit words.
        SexualContent = 2,
        /// Personal insults and hate speech.
        Slurs = 3,
    }
}

int_enum! {
    /// [What a rule checks](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-rule-object-event-types).
    ///
    /// discord.js counterpart: `AutoModerationRuleEventType`.
    pub enum EventType: u8 {
        /// A member sends or edits a message.
        MessageSend = 1,
        /// A member edits their profile.
        MemberUpdate = 2,
    }
}

/// An [action](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-action-object)
/// taken when a rule triggers.
///
/// discord.js counterpart: `AutoModerationAction`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Action {
    /// What the action does.
    #[serde(rename = "type")]
    pub kind: ActionType,
    /// Settings of the action; required for `SendAlertMessage` and `Timeout`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ActionMetadata>,
}

int_enum! {
    /// [What an action does](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-action-object-action-types).
    ///
    /// discord.js counterpart: `AutoModerationActionType`.
    pub enum ActionType: u8 {
        /// Blocks the message, optionally explaining why to the member.
        BlockMessage = 1,
        /// Logs the content to a channel.
        SendAlertMessage = 2,
        /// Times the member out (`MentionSpam`, `Keyword` and `MemberProfile` rules; needs the
        /// `MODERATE_MEMBERS` permission).
        Timeout = 3,
        /// Stops the member from using text, voice and other interactions.
        BlockMemberInteraction = 4,
    }
}

/// The [settings of an action](https://docs.discord.com/developers/resources/auto-moderation#auto-moderation-action-object-action-metadata).
///
/// discord.js counterpart: `AutoModerationActionMetadata`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActionMetadata {
    /// Where `SendAlertMessage` logs the content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// How long `Timeout` times the member out, in seconds (up to 2 419 200, four weeks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,
    /// The explanation `BlockMessage` shows to the member (up to 150 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_message: Option<CompactString>,
}
