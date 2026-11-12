//! [`validate_components`]: the limits only a whole list of message components shows (D-199).

use djs_model::component::Component;

use super::validate;
use crate::{
    ValidationError,
    check::{self, Path},
};

/// Which component system a message uses — the `IS_COMPONENTS_V2` message flag.
///
/// discord.js counterpart: `MessageFlags.IsComponentsV2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ComponentsVersion {
    /// Without the flag: up to five action rows next to the content and embeds.
    Legacy,
    /// With [`MessageFlags::IS_COMPONENTS_V2`](djs_model::message::MessageFlags::IS_COMPONENTS_V2):
    /// layout and content components replace the content and embeds, up to 40 components
    /// and 4000 characters of text in all.
    V2,
}

/// Legacy messages have at most five action rows.
const LEGACY_ROWS: usize = 5;
/// Components V2 messages have at most 40 components, nested ones included (D-200).
const V2_COMPONENTS: usize = 40;
/// Components V2 messages show at most 4000 characters of text in all (D-200, D-201).
const V2_TEXT: usize = 4000;

/// Checks the limits that apply to all components of one message together (D-199), and each
/// component itself.
///
/// - [`Legacy`](ComponentsVersion::Legacy): at most 5 components, all action rows.
/// - [`V2`](ComponentsVersion::V2): only layout and content components at the top level; at
///   most 40 components counting nested ones, and at most 4000 characters (code points after
///   trimming) across the text displays — as measured on the live API (D-200, D-201).
/// - Both: every `custom_id` is unique in the message (D-200).
///
/// Discord also refuses a request that is too large in bytes (`413`) — for example a single
/// text display of 4000 emoji —, which this does not check.
///
/// # Errors
///
/// The first [`ValidationError`] found: the number or type of top-level components, a
/// component's own rules, a repeated `custom_id`, then the V2 totals
/// ([`TooMany`](ValidationError::TooMany) / [`TooLong`](ValidationError::TooLong) with the
/// field `components`).
///
/// # Examples
///
/// ```
/// use djs_builders::{ActionRowBuilder, ButtonBuilder, ComponentsVersion, validate_components};
///
/// let row = |id: &str| {
///     ActionRowBuilder::new()
///         .component(ButtonBuilder::primary(id).label("Go"))
///         .build()
///         .unwrap()
/// };
/// assert!(validate_components(&[row("a"), row("b")], ComponentsVersion::Legacy).is_ok());
///
/// let error = validate_components(&[row("a"), row("a")], ComponentsVersion::Legacy).unwrap_err();
/// assert_eq!(error.field(), "components[1].components[0].custom_id");
/// ```
pub fn validate_components(
    components: &[Component],
    version: ComponentsVersion,
) -> Result<(), ValidationError> {
    let root = Path::Root;
    if version == ComponentsVersion::Legacy {
        check::count(|| "components".into(), components.len(), LEGACY_ROWS)?;
    }
    let mut totals = Totals::default();
    for (index, component) in components.iter().enumerate() {
        let here = root.item("components", index);
        let allowed = match version {
            ComponentsVersion::Legacy => matches!(component, Component::ActionRow(_)),
            ComponentsVersion::V2 => matches!(
                component,
                Component::ActionRow(_)
                    | Component::TextDisplay(_)
                    | Component::Section(_)
                    | Component::MediaGallery(_)
                    | Component::File(_)
                    | Component::Separator(_)
                    | Component::Container(_)
            ),
        };
        if !allowed {
            return Err(ValidationError::Invalid {
                field: here.this(),
                reason: match version {
                    ComponentsVersion::Legacy => {
                        "a message without the Components V2 flag can only have action rows"
                    }
                    ComponentsVersion::V2 => {
                        "this component cannot be at the top level of a message"
                    }
                },
            });
        }
        validate::component(&here, component)?;
        totals.walk(&here, component)?;
    }
    if version == ComponentsVersion::V2 {
        check::count(|| "components".into(), totals.components, V2_COMPONENTS)?;
        if totals.text > V2_TEXT {
            return Err(ValidationError::TooLong {
                field: "components".into(),
                max: V2_TEXT,
                actual: totals.text,
            });
        }
    }
    Ok(())
}

/// What a walk over the component tree has seen.
#[derive(Default)]
struct Totals<'a> {
    components: usize,
    text: usize,
    custom_ids: Vec<&'a str>,
}

impl<'a> Totals<'a> {
    fn walk(&mut self, path: &Path<'_>, component: &'a Component) -> Result<(), ValidationError> {
        self.components += 1;
        if let Some(custom_id) = component.custom_id() {
            if self.custom_ids.contains(&custom_id) {
                return Err(ValidationError::Invalid {
                    field: path.field("custom_id"),
                    reason: "another component of the message has the same custom_id",
                });
            }
            self.custom_ids.push(custom_id);
        }
        if let Component::TextDisplay(text) = component {
            self.text += check::length(&text.content);
        }
        for (index, child) in component.children().iter().enumerate() {
            self.walk(&path.item("components", index), child)?;
        }
        if let Component::Section(section) = component {
            self.walk(&path.child("accessory"), &section.accessory)?;
        }
        Ok(())
    }
}
