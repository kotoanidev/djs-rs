//! The rules `build()` checks for components (D-198–D-201), shared by every component builder
//! and by `validate_components`.

use std::borrow::Cow;

use djs_model::component::{
    ActionRow, AutoPopulatedSelect, Button, ButtonStyle, ChannelSelect, Component,
    SelectDefaultValue, SelectDefaultValueType, StringSelect,
};

use crate::{
    ValidationError,
    check::{self, Path, WEB_OR_DISCORD},
};

/// Discord's [component limits](https://docs.discord.com/developers/components/reference), as
/// checked on the live API (D-200, D-201).
pub(super) mod limits {
    pub(crate) const CUSTOM_ID: usize = 100;
    pub(crate) const BUTTON_LABEL: usize = 80;
    pub(crate) const BUTTON_URL: usize = 512;
    pub(crate) const ROW: usize = 5;
    pub(crate) const SELECT_OPTIONS: usize = 25;
    pub(crate) const OPTION_TEXT: usize = 100;
    pub(crate) const PLACEHOLDER: usize = 150;
    pub(crate) const SELECT_VALUES: u8 = 25;
}

/// Checks a `custom_id` (or a select option value): 1-100 code points, counted without
/// trimming — Discord accepts a whitespace-only one (D-200).
pub(super) fn identifier(
    field: impl Fn() -> Cow<'static, str>,
    identifier: &str,
) -> Result<(), ValidationError> {
    let actual = identifier.chars().count();
    if actual == 0 {
        return Err(ValidationError::Missing { field: field() });
    }
    if actual > limits::CUSTOM_ID {
        return Err(ValidationError::TooLong {
            field: field(),
            max: limits::CUSTOM_ID,
            actual,
        });
    }
    Ok(())
}

/// Checks any component this crate knows the rules of, at `path`.
pub(super) fn component(path: &Path<'_>, component: &Component) -> Result<(), ValidationError> {
    use SelectDefaultValueType::{Channel, Role, User};
    match component {
        Component::ActionRow(row) => action_row(path, row),
        Component::Button(button) => self::button(path, button),
        Component::StringSelect(select) => string_select(path, select),
        Component::UserSelect(select) => auto_select(path, select, &[User]),
        Component::RoleSelect(select) => auto_select(path, select, &[Role]),
        Component::MentionableSelect(select) => auto_select(path, select, &[User, Role]),
        Component::ChannelSelect(select) => channel_select(path, select, &[Channel]),
        _ => Ok(()),
    }
}

/// Whether a component takes a whole action row on its own.
const fn fills_a_row(component: &Component) -> bool {
    matches!(
        component,
        Component::StringSelect(_)
            | Component::UserSelect(_)
            | Component::RoleSelect(_)
            | Component::MentionableSelect(_)
            | Component::ChannelSelect(_)
            | Component::TextInput(_)
    )
}

/// Checks an action row: 1-5 buttons, or one select menu or text input, with unique
/// `custom_id`s (D-198, D-200).
pub(super) fn action_row(path: &Path<'_>, row: &ActionRow) -> Result<(), ValidationError> {
    let children = &row.components;
    check::at_least(|| path.field("components"), children.len(), 1)?;
    check::count(|| path.field("components"), children.len(), limits::ROW)?;
    for (index, child) in children.iter().enumerate() {
        let here = path.item("components", index);
        let misplaced = match child {
            Component::Button(_) => None,
            child if fills_a_row(child) => (children.len() > 1)
                .then_some("a select menu or text input fills a whole action row"),
            _ => Some("an action row can only hold buttons, one select menu or one text input"),
        };
        if let Some(reason) = misplaced {
            return Err(ValidationError::Invalid {
                field: here.this(),
                reason,
            });
        }
        if let Some(custom_id) = child.custom_id()
            && children[..index]
                .iter()
                .any(|other| other.custom_id() == Some(custom_id))
        {
            return Err(ValidationError::Invalid {
                field: here.field("custom_id"),
                reason: "another component has the same custom_id",
            });
        }
        component(&here, child)?;
    }
    Ok(())
}

/// Checks a button: its `custom_id` or URL, and a label unless it has an emoji (D-200).
pub(super) fn button(path: &Path<'_>, button: &Button) -> Result<(), ValidationError> {
    match button {
        Button::Interactive(button) => {
            if matches!(button.style, ButtonStyle::Link | ButtonStyle::Premium) {
                return Err(ValidationError::Invalid {
                    field: path.field("style"),
                    reason: "a button with a custom_id cannot have the link or premium style",
                });
            }
            identifier(|| path.field("custom_id"), &button.custom_id)?;
            label(path, button.label.as_deref(), button.emoji.is_some())
        }
        Button::Link(button) => {
            check::text(|| path.field("url"), &button.url, limits::BUTTON_URL)?;
            check::url(|| path.field("url"), &button.url, WEB_OR_DISCORD)?;
            label(path, button.label.as_deref(), button.emoji.is_some())
        }
        _ => Ok(()),
    }
}

/// A button needs a label that is not blank, or an emoji (D-200); a label has at most 80
/// characters.
fn label(path: &Path<'_>, label: Option<&str>, has_emoji: bool) -> Result<(), ValidationError> {
    match label {
        Some(label) if !check::is_blank(label) => {
            check::text(|| path.field("label"), label, limits::BUTTON_LABEL).map(drop)
        }
        _ if has_emoji => Ok(()),
        _ => Err(ValidationError::Missing {
            field: path.field("label"),
        }),
    }
}

/// Checks a placeholder (at most 150 characters).
fn placeholder(path: &Path<'_>, placeholder: Option<&str>) -> Result<(), ValidationError> {
    if let Some(placeholder) = placeholder {
        check::text(
            || path.field("placeholder"),
            placeholder,
            limits::PLACEHOLDER,
        )?;
    }
    Ok(())
}

/// Checks `min_values` (0-25) and `max_values` (1-25), both 1 when not set, with
/// `min_values` ≤ `max_values` (D-200); returns the pair in effect.
fn values(
    path: &Path<'_>,
    min_values: Option<u8>,
    max_values: Option<u8>,
) -> Result<(u8, u8), ValidationError> {
    let upper = limits::SELECT_VALUES;
    if let Some(min) = min_values
        && min > upper
    {
        return Err(ValidationError::OutOfRange {
            field: path.field("min_values"),
            min: 0,
            max: upper.into(),
            actual: min.into(),
        });
    }
    if let Some(max) = max_values
        && !(1..=upper).contains(&max)
    {
        return Err(ValidationError::OutOfRange {
            field: path.field("max_values"),
            min: 1,
            max: upper.into(),
            actual: max.into(),
        });
    }
    let (min, max) = (min_values.unwrap_or(1), max_values.unwrap_or(1));
    if min > max {
        // Discord reports it on `min_values`, bounded by `max_values`.
        return Err(ValidationError::OutOfRange {
            field: path.field("min_values"),
            min: 0,
            max: max.into(),
            actual: min.into(),
        });
    }
    Ok((min, max))
}

/// Checks a string select menu: 1-25 options, at least `max_values` of them, each with a
/// label, a unique value and an optional description (D-200).
fn string_select(path: &Path<'_>, select: &StringSelect) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &select.custom_id)?;
    placeholder(path, select.placeholder.as_deref())?;
    let (_, max) = values(path, select.min_values, select.max_values)?;
    let options = &select.options;
    check::count(
        || path.field("options"),
        options.len(),
        limits::SELECT_OPTIONS,
    )?;
    check::at_least(|| path.field("options"), options.len(), usize::from(max))?;
    for (index, option) in options.iter().enumerate() {
        let here = path.item("options", index);
        check::required_text(|| here.field("label"), &option.label, limits::OPTION_TEXT)?;
        identifier(|| here.field("value"), &option.value)?;
        if let Some(description) = &option.description {
            check::text(
                || here.field("description"),
                description,
                limits::OPTION_TEXT,
            )?;
        }
        if options[..index]
            .iter()
            .any(|other| other.value == option.value)
        {
            return Err(ValidationError::Invalid {
                field: here.field("value"),
                reason: "another option of the menu has the same value",
            });
        }
    }
    Ok(())
}

fn auto_select(
    path: &Path<'_>,
    select: &AutoPopulatedSelect,
    allowed: &[SelectDefaultValueType],
) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &select.custom_id)?;
    placeholder(path, select.placeholder.as_deref())?;
    let (min, max) = values(path, select.min_values, select.max_values)?;
    default_values(path, &select.default_values, (min, max), allowed)
}

fn channel_select(
    path: &Path<'_>,
    select: &ChannelSelect,
    allowed: &[SelectDefaultValueType],
) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &select.custom_id)?;
    placeholder(path, select.placeholder.as_deref())?;
    let (min, max) = values(path, select.min_values, select.max_values)?;
    default_values(path, &select.default_values, (min, max), allowed)
}

/// The number of default values must be within `min_values..=max_values`, and each must be of
/// a type the menu offers.
fn default_values(
    path: &Path<'_>,
    defaults: &[SelectDefaultValue],
    (min, max): (u8, u8),
    allowed: &[SelectDefaultValueType],
) -> Result<(), ValidationError> {
    if defaults.is_empty() {
        return Ok(());
    }
    check::count(|| path.field("default_values"), defaults.len(), max.into())?;
    check::at_least(|| path.field("default_values"), defaults.len(), min.into())?;
    for (index, default) in defaults.iter().enumerate() {
        if !allowed.contains(&default.kind) {
            return Err(ValidationError::Invalid {
                field: path.item("default_values", index).field("type"),
                reason: "this select menu cannot have a default value of this type",
            });
        }
    }
    Ok(())
}
