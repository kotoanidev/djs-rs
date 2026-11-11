//! Bodies of the [stage instance endpoints](https://docs.discord.com/developers/resources/stage-instance).

use compact_str::CompactString;
use serde::Serialize;

use crate::{
    id::{ChannelId, ScheduledEventId},
    stage_instance::PrivacyLevel,
};

/// The body of [Create Stage Instance](https://docs.discord.com/developers/resources/stage-instance#create-stage-instance).
///
/// discord.js counterpart: `StageInstanceCreateOptions`.
///
/// # Examples
///
/// ```
/// use djs_model::{id::ChannelId, rest::CreateStageInstance};
///
/// let mut body = CreateStageInstance::new(ChannelId::new(1_200_000_000_000_000_005), "Q&A");
/// body.send_start_notification = Some(true);
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"channel_id":"1200000000000000005","topic":"Q&A","send_start_notification":true}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateStageInstance {
    /// The stage channel.
    pub channel_id: ChannelId,
    /// The topic (1-120 characters).
    pub topic: CompactString,
    /// Who can see the stage (Discord's default: [`PrivacyLevel::GuildOnly`]; `Public` is
    /// deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<PrivacyLevel>,
    /// Whether to notify `@everyone` that the stage started (the bot needs
    /// `MENTION_EVERYONE`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_start_notification: Option<bool>,
    /// The scheduled event the stage belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_scheduled_event_id: Option<ScheduledEventId>,
}

impl CreateStageInstance {
    /// Opens the stage channel `channel_id` with `topic`.
    #[must_use]
    pub fn new(channel_id: ChannelId, topic: impl Into<CompactString>) -> Self {
        Self {
            channel_id,
            topic: topic.into(),
            privacy_level: None,
            send_start_notification: None,
            guild_scheduled_event_id: None,
        }
    }
}

/// The body of [Modify Stage Instance](https://docs.discord.com/developers/resources/stage-instance#modify-stage-instance).
///
/// Every field is optional.
///
/// discord.js counterpart: `StageInstanceEditOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyStageInstance {
    /// The new topic (1-120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<CompactString>,
    /// Who can see the stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<PrivacyLevel>,
}
