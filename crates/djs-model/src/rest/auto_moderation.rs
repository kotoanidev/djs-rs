//! Bodies of the [Auto Moderation endpoints](https://docs.discord.com/developers/resources/auto-moderation).

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    auto_moderation::{Action, EventType, KeywordPresetType, TriggerMetadata, TriggerType},
    id::{ChannelId, RoleId},
};

/// The body of [Create Auto Moderation Rule](https://docs.discord.com/developers/resources/auto-moderation#create-auto-moderation-rule).
///
/// Built with one constructor per [`TriggerType`] (D-128), which sets the trigger, the
/// [`EventType`] it checks and the trigger metadata it requires. The allow list, regular
/// expressions, [`enabled`](Self::enabled) and the exemptions are set through the public
/// fields afterwards.
///
/// discord.js counterpart: the argument of `AutoModerationRuleManager#create`.
///
/// # Examples
///
/// ```
/// use djs_model::{auto_moderation::Action, rest::CreateAutoModerationRule};
///
/// let mut body = CreateAutoModerationRule::keyword(
///     "No spoilers",
///     ["*spoiler*"],
///     [Action::block_message()],
/// );
/// body.enabled = Some(true);
/// body.trigger_metadata.allow_list = vec!["spoilerfree".into()];
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"name":"No spoilers","event_type":1,"trigger_type":1,"trigger_metadata":{"keyword_filter":["*spoiler*"],"allow_list":["spoilerfree"]},"actions":[{"type":1}],"enabled":true}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct CreateAutoModerationRule {
    /// The rule's name.
    pub name: CompactString,
    /// What the rule checks: sent messages, or member profiles for `MemberProfile` rules.
    pub event_type: EventType,
    /// How the rule detects violations. Each guild may have up to 6 `Keyword` rules and 1 of
    /// every other type.
    pub trigger_type: TriggerType,
    /// The settings of the trigger; only the fields of [`trigger_type`](Self::trigger_type)
    /// apply. Left out of the request when empty (`Spam` rules have none).
    #[serde(skip_serializing_if = "is_empty")]
    pub trigger_metadata: TriggerMetadata,
    /// What happens when the rule triggers.
    pub actions: Vec<Action>,
    /// Whether the rule is enabled; Discord's default is disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Roles the rule does not apply to (up to 20).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exempt_roles: Vec<RoleId>,
    /// Channels the rule does not apply to (up to 50).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exempt_channels: Vec<ChannelId>,
}

/// Whether trigger metadata has nothing set, so it can be left out of the request.
fn is_empty(metadata: &TriggerMetadata) -> bool {
    *metadata == TriggerMetadata::default()
}

impl CreateAutoModerationRule {
    /// The shared part of the constructors.
    fn with(
        name: impl Into<CompactString>,
        event_type: EventType,
        trigger_type: TriggerType,
        trigger_metadata: TriggerMetadata,
        actions: impl IntoIterator<Item = Action>,
    ) -> Self {
        Self {
            name: name.into(),
            event_type,
            trigger_type,
            trigger_metadata,
            actions: actions.into_iter().collect(),
            enabled: None,
            exempt_roles: Vec::new(),
            exempt_channels: Vec::new(),
        }
    }

    /// A rule that triggers on messages containing one of `keywords` (up to 1000, 60
    /// characters each; `*` at the start or end matches a prefix or suffix). Add
    /// [`regex_patterns`](TriggerMetadata::regex_patterns) and an
    /// [`allow_list`](TriggerMetadata::allow_list) through
    /// [`trigger_metadata`](Self::trigger_metadata).
    #[must_use]
    pub fn keyword(
        name: impl Into<CompactString>,
        keywords: impl IntoIterator<Item = impl Into<CompactString>>,
        actions: impl IntoIterator<Item = Action>,
    ) -> Self {
        let metadata = TriggerMetadata {
            keyword_filter: keywords.into_iter().map(Into::into).collect(),
            ..TriggerMetadata::default()
        };
        Self::with(
            name,
            EventType::MessageSend,
            TriggerType::Keyword,
            metadata,
            actions,
        )
    }

    /// A rule that triggers on messages Discord considers generic spam.
    #[must_use]
    pub fn spam(name: impl Into<CompactString>, actions: impl IntoIterator<Item = Action>) -> Self {
        Self::with(
            name,
            EventType::MessageSend,
            TriggerType::Spam,
            TriggerMetadata::default(),
            actions,
        )
    }

    /// A rule that triggers on messages containing words from Discord's predefined word
    /// lists. Exceptions go into the [`allow_list`](TriggerMetadata::allow_list) (up to 1000).
    #[must_use]
    pub fn keyword_preset(
        name: impl Into<CompactString>,
        presets: impl IntoIterator<Item = KeywordPresetType>,
        actions: impl IntoIterator<Item = Action>,
    ) -> Self {
        let metadata = TriggerMetadata {
            presets: presets.into_iter().collect(),
            ..TriggerMetadata::default()
        };
        Self::with(
            name,
            EventType::MessageSend,
            TriggerType::KeywordPreset,
            metadata,
            actions,
        )
    }

    /// A rule that triggers on messages mentioning more than `limit` unique users and roles
    /// (up to 50). Enable
    /// [`mention_raid_protection_enabled`](TriggerMetadata::mention_raid_protection_enabled)
    /// to also detect mention raids.
    #[must_use]
    pub fn mention_spam(
        name: impl Into<CompactString>,
        limit: u8,
        actions: impl IntoIterator<Item = Action>,
    ) -> Self {
        let metadata = TriggerMetadata {
            mention_total_limit: Some(limit),
            ..TriggerMetadata::default()
        };
        Self::with(
            name,
            EventType::MessageSend,
            TriggerType::MentionSpam,
            metadata,
            actions,
        )
    }

    /// A rule that triggers when a member's profile contains one of `keywords`, checked when
    /// they edit it. The keywords work as for [`keyword`](Self::keyword).
    #[must_use]
    pub fn member_profile(
        name: impl Into<CompactString>,
        keywords: impl IntoIterator<Item = impl Into<CompactString>>,
        actions: impl IntoIterator<Item = Action>,
    ) -> Self {
        let metadata = TriggerMetadata {
            keyword_filter: keywords.into_iter().map(Into::into).collect(),
            ..TriggerMetadata::default()
        };
        Self::with(
            name,
            EventType::MemberUpdate,
            TriggerType::MemberProfile,
            metadata,
            actions,
        )
    }
}

/// The body of [Modify Auto Moderation Rule](https://docs.discord.com/developers/resources/auto-moderation#modify-auto-moderation-rule).
///
/// Every field is optional; the trigger type cannot be changed. Lists replace the rule's
/// lists, so pass complete ones (an empty list clears it).
///
/// discord.js counterpart: the argument of `AutoModerationRule#edit`,
/// `AutoModerationRuleManager#edit`.
///
/// # Examples
///
/// ```
/// use djs_model::{auto_moderation::Action, rest::ModifyAutoModerationRule};
///
/// let mut body = ModifyAutoModerationRule::default();
/// body.enabled = Some(false);
/// body.actions = Some(vec![Action::block_message(), Action::timeout(60)]);
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"actions":[{"type":1},{"type":3,"metadata":{"duration_seconds":60}}],"enabled":false}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ModifyAutoModerationRule {
    /// The new name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The new event type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<EventType>,
    /// The new trigger settings. Start from the rule's current
    /// [`trigger_metadata`](crate::auto_moderation::Rule::trigger_metadata) to keep the
    /// fields you do not change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_metadata: Option<TriggerMetadata>,
    /// The new actions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The new exempt roles (up to 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_roles: Option<Vec<RoleId>>,
    /// The new exempt channels (up to 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_channels: Option<Vec<ChannelId>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_moderation::{Action, KeywordPresetType};

    #[test]
    fn constructors_set_the_trigger() {
        let json = |body: &CreateAutoModerationRule| serde_json::to_string(body).unwrap();
        assert_eq!(
            json(&CreateAutoModerationRule::spam(
                "Spam",
                [Action::block_message()]
            )),
            r#"{"name":"Spam","event_type":1,"trigger_type":3,"actions":[{"type":1}]}"#
        );
        assert_eq!(
            json(&CreateAutoModerationRule::keyword_preset(
                "Slurs",
                [KeywordPresetType::Slurs],
                [Action::block_message()]
            )),
            r#"{"name":"Slurs","event_type":1,"trigger_type":4,"trigger_metadata":{"presets":[3]},"actions":[{"type":1}]}"#
        );
        let mut mentions = CreateAutoModerationRule::mention_spam(
            "Mentions",
            10,
            [Action::block_message(), Action::timeout(60)],
        );
        mentions.trigger_metadata.mention_raid_protection_enabled = true;
        assert_eq!(
            json(&mentions),
            r#"{"name":"Mentions","event_type":1,"trigger_type":5,"trigger_metadata":{"mention_total_limit":10,"mention_raid_protection_enabled":true},"actions":[{"type":1},{"type":3,"metadata":{"duration_seconds":60}}]}"#
        );
        assert_eq!(
            json(&CreateAutoModerationRule::member_profile(
                "Names",
                ["*admin*"],
                [Action::block_member_interaction()]
            )),
            r#"{"name":"Names","event_type":2,"trigger_type":6,"trigger_metadata":{"keyword_filter":["*admin*"]},"actions":[{"type":4}]}"#
        );
    }
}
