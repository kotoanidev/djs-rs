//! The rules `build()` checks for components (D-198–D-201), shared by every component builder
//! and by `validate_components`.

use std::borrow::Cow;

use djs_model::{
    component::{
        ActionRow, AutoPopulatedSelect, Button, ButtonStyle, ChannelSelect, CheckboxGroup,
        Component, Container, File, FileUpload, GroupOption, Label, MediaGallery, RadioGroup,
        Section, SelectDefaultValue, SelectDefaultValueType, StringSelect, TextDisplay, TextInput,
        Thumbnail,
    },
    interaction::ModalCallbackData,
};

use crate::{
    ValidationError,
    check::{self, Path, WEB_OR_ATTACHMENT, WEB_OR_DISCORD},
};

/// The only scheme a file component accepts (D-205).
const ATTACHMENT: &[&str] = &["attachment://"];

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
    /// A text display, and all text displays of a message together.
    pub(crate) const TEXT: usize = 4000;
    pub(crate) const MEDIA_DESCRIPTION: usize = 1024;
    pub(crate) const GALLERY_ITEMS: usize = 10;
    pub(crate) const SECTION_TEXTS: usize = 3;
    pub(crate) const CONTAINER: usize = 40;
    pub(crate) const COLOR: u32 = 0xFF_FFFF;
    pub(crate) const MODAL_TITLE: usize = 45;
    pub(crate) const MODAL_COMPONENTS: usize = 5;
    pub(crate) const LABEL: usize = 45;
    pub(crate) const LABEL_DESCRIPTION: usize = 100;
    /// A text input's length bounds and pre-filled value.
    pub(crate) const TEXT_INPUT: u16 = 4000;
    pub(crate) const TEXT_INPUT_PLACEHOLDER: usize = 100;
    /// Files of a file upload, options of a radio or checkbox group, file types.
    pub(crate) const MODAL_CHOICES: u8 = 10;
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
        Component::TextDisplay(text) => text_display(path, text),
        Component::Thumbnail(thumbnail) => self::thumbnail(path, thumbnail),
        Component::MediaGallery(gallery) => media_gallery(path, gallery),
        Component::File(file) => self::file(path, file),
        Component::Section(section) => self::section(path, section),
        Component::Container(container) => self::container(path, container),
        Component::Label(label) => self::label(path, label),
        Component::TextInput(input) => text_input(path, input),
        Component::FileUpload(upload) => file_upload(path, upload),
        Component::RadioGroup(group) => radio_group(path, group),
        Component::CheckboxGroup(group) => checkbox_group(path, group),
        Component::Checkbox(checkbox) => {
            identifier(|| path.field("custom_id"), &checkbox.custom_id)
        }
        _ => Ok(()),
    }
}

/// What a walk over a component tree has seen: the components (nested ones and section
/// accessories included), the characters of the text displays, and the `custom_id`s, which
/// must be unique (D-200).
#[derive(Default)]
pub(super) struct Totals<'a> {
    pub(super) components: usize,
    pub(super) text: usize,
    custom_ids: Vec<&'a str>,
}

impl<'a> Totals<'a> {
    /// Adds `component` and everything in it, failing on a repeated `custom_id`.
    pub(super) fn walk(
        &mut self,
        path: &Path<'_>,
        component: &'a Component,
    ) -> Result<(), ValidationError> {
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
        if let Component::Label(label) = component {
            self.walk(&path.child("component"), &label.component)?;
        }
        Ok(())
    }
}

/// A text display has 1-4000 characters after trimming (D-200).
fn text_display(path: &Path<'_>, text: &TextDisplay) -> Result<(), ValidationError> {
    check::required_text(|| path.field("content"), &text.content, limits::TEXT).map(drop)
}

/// A media description (alt text) has 1-1024 characters when set (D-205).
fn media_description(path: &Path<'_>, description: Option<&str>) -> Result<(), ValidationError> {
    if let Some(description) = description {
        check::required_text(
            || path.field("description"),
            description,
            limits::MEDIA_DESCRIPTION,
        )?;
    }
    Ok(())
}

/// A thumbnail shows a link or an uploaded file (the D-190 rule, although Discord accepts any
/// scheme, D-205).
fn thumbnail(path: &Path<'_>, thumbnail: &Thumbnail) -> Result<(), ValidationError> {
    check::url(
        || path.child("media").field("url"),
        &thumbnail.media.url,
        WEB_OR_ATTACHMENT,
    )?;
    media_description(path, thumbnail.description.as_deref())
}

/// A gallery has 1-10 items, each a link or an uploaded file (D-205).
fn media_gallery(path: &Path<'_>, gallery: &MediaGallery) -> Result<(), ValidationError> {
    check::at_least(|| path.field("items"), gallery.items.len(), 1)?;
    check::count(
        || path.field("items"),
        gallery.items.len(),
        limits::GALLERY_ITEMS,
    )?;
    for (index, item) in gallery.items.iter().enumerate() {
        let here = path.item("items", index);
        check::url(
            || here.child("media").field("url"),
            &item.media.url,
            WEB_OR_ATTACHMENT,
        )?;
        media_description(&here, item.description.as_deref())?;
    }
    Ok(())
}

/// A file component only shows a file uploaded with the message (D-205).
fn file(path: &Path<'_>, file: &File) -> Result<(), ValidationError> {
    check::url(
        || path.child("file").field("url"),
        &file.file.url,
        ATTACHMENT,
    )
}

/// A section has 1-3 text displays and a button or thumbnail as its accessory (D-205).
fn section(path: &Path<'_>, section: &Section) -> Result<(), ValidationError> {
    let children = &section.components;
    check::at_least(|| path.field("components"), children.len(), 1)?;
    check::count(
        || path.field("components"),
        children.len(),
        limits::SECTION_TEXTS,
    )?;
    for (index, child) in children.iter().enumerate() {
        let here = path.item("components", index);
        if !matches!(child, Component::TextDisplay(_)) {
            return Err(ValidationError::Invalid {
                field: here.this(),
                reason: "a section can only hold text displays",
            });
        }
        component(&here, child)?;
    }
    let accessory = path.child("accessory");
    if !matches!(
        *section.accessory,
        Component::Button(_) | Component::Thumbnail(_)
    ) {
        return Err(ValidationError::Invalid {
            field: accessory.this(),
            reason: "a section accessory must be a button or a thumbnail",
        });
    }
    component(&accessory, &section.accessory)
}

/// A container has 1-40 layout and content components, an accent color up to `0xFFFFFF` and
/// unique `custom_id`s inside (D-205).
fn container(path: &Path<'_>, container: &Container) -> Result<(), ValidationError> {
    let children = &container.components;
    check::at_least(|| path.field("components"), children.len(), 1)?;
    check::count(
        || path.field("components"),
        children.len(),
        limits::CONTAINER,
    )?;
    if let Some(color) = container.accent_color
        && color > limits::COLOR
    {
        return Err(ValidationError::OutOfRange {
            field: path.field("accent_color"),
            min: 0,
            max: limits::COLOR.into(),
            actual: color.into(),
        });
    }
    for (index, child) in children.iter().enumerate() {
        let here = path.item("components", index);
        if !matches!(
            child,
            Component::ActionRow(_)
                | Component::TextDisplay(_)
                | Component::Section(_)
                | Component::MediaGallery(_)
                | Component::Separator(_)
                | Component::File(_)
        ) {
            return Err(ValidationError::Invalid {
                field: here.this(),
                reason: "a container can only hold action rows, text displays, sections, media \
                         galleries, separators and files",
            });
        }
        component(&here, child)?;
    }
    let mut totals = Totals::default();
    for (index, child) in children.iter().enumerate() {
        totals.walk(&path.item("components", index), child)?;
    }
    Ok(())
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
    )
}

/// Checks an action row: 1-5 buttons or one select menu, with unique `custom_id`s (D-198,
/// D-200). Text inputs go in a modal's labels (D-207).
pub(super) fn action_row(path: &Path<'_>, row: &ActionRow) -> Result<(), ValidationError> {
    let children = &row.components;
    check::at_least(|| path.field("components"), children.len(), 1)?;
    check::count(|| path.field("components"), children.len(), limits::ROW)?;
    for (index, child) in children.iter().enumerate() {
        let here = path.item("components", index);
        let misplaced = match child {
            Component::Button(_) => None,
            child if fills_a_row(child) => {
                (children.len() > 1).then_some("a select menu fills a whole action row")
            }
            _ => Some("an action row can only hold buttons or one select menu"),
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
            button_label(path, button.label.as_deref(), button.emoji.is_some())
        }
        Button::Link(button) => {
            check::text(|| path.field("url"), &button.url, limits::BUTTON_URL)?;
            check::url(|| path.field("url"), &button.url, WEB_OR_DISCORD)?;
            button_label(path, button.label.as_deref(), button.emoji.is_some())
        }
        _ => Ok(()),
    }
}

/// A button needs a label that is not blank, or an emoji (D-200); a label has at most 80
/// characters.
fn button_label(
    path: &Path<'_>,
    label: Option<&str>,
    has_emoji: bool,
) -> Result<(), ValidationError> {
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

/// Checks `min_values` (0-`upper`) and `max_values` (1-`upper`), with `min_values` ≤
/// `max_values` (D-200); when not set they are 1 and `default_max`. Returns the pair in effect.
fn values(
    path: &Path<'_>,
    (min_values, max_values): (Option<u8>, Option<u8>),
    upper: u8,
    default_max: u8,
) -> Result<(u8, u8), ValidationError> {
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
    let (min, max) = (min_values.unwrap_or(1), max_values.unwrap_or(default_max));
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
    let (_, max) = values(
        path,
        (select.min_values, select.max_values),
        limits::SELECT_VALUES,
        1,
    )?;
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
    let (min, max) = values(
        path,
        (select.min_values, select.max_values),
        limits::SELECT_VALUES,
        1,
    )?;
    default_values(path, &select.default_values, (min, max), allowed)
}

fn channel_select(
    path: &Path<'_>,
    select: &ChannelSelect,
    allowed: &[SelectDefaultValueType],
) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &select.custom_id)?;
    placeholder(path, select.placeholder.as_deref())?;
    let (min, max) = values(
        path,
        (select.min_values, select.max_values),
        limits::SELECT_VALUES,
        1,
    )?;
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

/// Checks a modal: a `custom_id`, a title of 1-45 characters, and 1-5 labels or text displays
/// with unique `custom_id`s (D-206, D-207).
pub(super) fn modal(modal: &ModalCallbackData) -> Result<(), ValidationError> {
    let root = Path::Root;
    identifier(|| "custom_id".into(), &modal.custom_id)?;
    check::required_text(|| "title".into(), &modal.title, limits::MODAL_TITLE)?;
    let children = &modal.components;
    check::at_least(|| "components".into(), children.len(), 1)?;
    check::count(
        || "components".into(),
        children.len(),
        limits::MODAL_COMPONENTS,
    )?;
    for (index, child) in children.iter().enumerate() {
        let here = root.item("components", index);
        if !matches!(child, Component::Label(_) | Component::TextDisplay(_)) {
            return Err(ValidationError::Invalid {
                field: here.this(),
                reason: "a modal can only hold labels and text displays",
            });
        }
        component(&here, child)?;
    }
    let mut totals = Totals::default();
    for (index, child) in children.iter().enumerate() {
        totals.walk(&root.item("components", index), child)?;
    }
    Ok(())
}

/// Checks a label: a text of 1-45 characters, a description of at most 100, and one input
/// field with the rules fields have in a modal.
fn label(path: &Path<'_>, label: &Label) -> Result<(), ValidationError> {
    check::required_text(|| path.field("label"), &label.label, limits::LABEL)?;
    if let Some(description) = &label.description {
        check::text(
            || path.field("description"),
            description,
            limits::LABEL_DESCRIPTION,
        )?;
    }
    let field = path.child("component");
    if !matches!(
        *label.component,
        Component::TextInput(_)
            | Component::StringSelect(_)
            | Component::UserSelect(_)
            | Component::RoleSelect(_)
            | Component::MentionableSelect(_)
            | Component::ChannelSelect(_)
            | Component::FileUpload(_)
            | Component::RadioGroup(_)
            | Component::CheckboxGroup(_)
            | Component::Checkbox(_)
    ) {
        return Err(ValidationError::Invalid {
            field: field.this(),
            reason: "a label can only hold a text input, a select menu, a file upload, a radio \
                     group, a checkbox group or a checkbox",
        });
    }
    component(&field, &label.component)?;
    in_modal(&field, &label.component)
}

/// The rules a field has only in a modal: a select menu cannot be disabled, and a required
/// field cannot have `min_values` 0 (the docs).
fn in_modal(path: &Path<'_>, field: &Component) -> Result<(), ValidationError> {
    let (disabled, required, min_values) = match field {
        Component::StringSelect(select) => (select.disabled, select.required, select.min_values),
        Component::UserSelect(select)
        | Component::RoleSelect(select)
        | Component::MentionableSelect(select) => {
            (select.disabled, select.required, select.min_values)
        }
        Component::ChannelSelect(select) => (select.disabled, select.required, select.min_values),
        Component::FileUpload(upload) => (false, upload.required, upload.min_values),
        Component::CheckboxGroup(group) => (false, group.required, group.min_values),
        _ => return Ok(()),
    };
    if disabled {
        return Err(ValidationError::Invalid {
            field: path.field("disabled"),
            reason: "a select menu in a modal cannot be disabled",
        });
    }
    if required && min_values == Some(0) {
        return Err(ValidationError::Invalid {
            field: path.field("min_values"),
            reason: "min_values must be at least 1 while the field is required",
        });
    }
    Ok(())
}

/// Checks a text input: `min_length` 0-4000, `max_length` 1-4000 and at least `min_length`, a
/// value of at most 4000 characters and a placeholder of at most 100.
fn text_input(path: &Path<'_>, input: &TextInput) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &input.custom_id)?;
    let upper = limits::TEXT_INPUT;
    if let Some(min) = input.min_length
        && min > upper
    {
        return Err(ValidationError::OutOfRange {
            field: path.field("min_length"),
            min: 0,
            max: upper.into(),
            actual: min.into(),
        });
    }
    if let Some(max) = input.max_length {
        let lowest = input.min_length.unwrap_or(0).max(1);
        if !(lowest..=upper).contains(&max) {
            return Err(ValidationError::OutOfRange {
                field: path.field("max_length"),
                min: lowest.into(),
                max: upper.into(),
                actual: max.into(),
            });
        }
    }
    if let Some(value) = &input.value {
        check::text(|| path.field("value"), value, upper.into())?;
    }
    if let Some(placeholder) = &input.placeholder {
        check::text(
            || path.field("placeholder"),
            placeholder,
            limits::TEXT_INPUT_PLACEHOLDER,
        )?;
    }
    Ok(())
}

/// Checks a file upload: 0-10 / 1-10 files and at most 10 file types.
fn file_upload(path: &Path<'_>, upload: &FileUpload) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &upload.custom_id)?;
    values(
        path,
        (upload.min_values, upload.max_values),
        limits::MODAL_CHOICES,
        1,
    )?;
    check::count(
        || path.field("file_types"),
        upload.file_types.len(),
        limits::MODAL_CHOICES.into(),
    )
}

/// Checks the options of a radio or checkbox group: labels of 1-100 characters, unique values
/// of 1-100, descriptions of at most 100.
fn group_options(path: &Path<'_>, options: &[GroupOption]) -> Result<(), ValidationError> {
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
                reason: "another option of the group has the same value",
            });
        }
    }
    Ok(())
}

/// Checks a radio group: 2-10 options, at most one of them selected by default.
fn radio_group(path: &Path<'_>, group: &RadioGroup) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &group.custom_id)?;
    let options = &group.options;
    check::at_least(|| path.field("options"), options.len(), 2)?;
    check::count(
        || path.field("options"),
        options.len(),
        limits::MODAL_CHOICES.into(),
    )?;
    group_options(path, options)?;
    if let Some((index, _)) = options
        .iter()
        .enumerate()
        .filter(|(_, option)| option.default)
        .nth(1)
    {
        return Err(ValidationError::Invalid {
            field: path.item("options", index).field("default"),
            reason: "only one option of a radio group can be selected",
        });
    }
    Ok(())
}

/// Checks a checkbox group: 1-10 options and `min_values` / `max_values` within them
/// (`max_values` defaults to the number of options).
fn checkbox_group(path: &Path<'_>, group: &CheckboxGroup) -> Result<(), ValidationError> {
    identifier(|| path.field("custom_id"), &group.custom_id)?;
    let options = &group.options;
    check::at_least(|| path.field("options"), options.len(), 1)?;
    check::count(
        || path.field("options"),
        options.len(),
        limits::MODAL_CHOICES.into(),
    )?;
    let all = u8::try_from(options.len()).unwrap_or(u8::MAX);
    let (_, max) = values(
        path,
        (group.min_values, group.max_values),
        limits::MODAL_CHOICES,
        all,
    )?;
    check::at_least(|| path.field("options"), options.len(), max.into())?;
    group_options(path, options)
}
