//! Bodies of the [guild template endpoints](https://docs.discord.com/developers/resources/guild-template).

use compact_str::CompactString;
use serde::Serialize;

use crate::util::Patch;

/// The body of [Create Guild Template](https://docs.discord.com/developers/resources/guild-template#create-guild-template).
///
/// discord.js counterpart: the arguments of `Guild#createTemplate`.
///
/// # Examples
///
/// ```
/// use djs_model::rest::CreateGuildTemplate;
///
/// let mut body = CreateGuildTemplate::new("Study group");
/// body.description = Some("Channels and roles for a study group".into());
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"name":"Study group","description":"Channels and roles for a study group"}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateGuildTemplate {
    /// The name (1-100 characters).
    pub name: CompactString,
    /// The description (0-120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CompactString>,
}

impl CreateGuildTemplate {
    /// A template named `name`, without a description.
    #[must_use]
    pub fn new(name: impl Into<CompactString>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }
}

/// The body of [Modify Guild Template](https://docs.discord.com/developers/resources/guild-template#modify-guild-template).
///
/// Every field is optional. The template's guild snapshot is updated with
/// `sync_guild_template` instead.
///
/// discord.js counterpart: the argument of `GuildTemplate#edit`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct ModifyGuildTemplate {
    /// The new name (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<CompactString>,
    /// The new description (0-120 characters); [`Patch::Clear`] removes it.
    #[serde(skip_serializing_if = "Patch::is_keep")]
    pub description: Patch<CompactString>,
}
