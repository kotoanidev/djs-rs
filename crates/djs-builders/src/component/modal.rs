//! Modal builders: [`ModalBuilder`], [`LabelBuilder`] and the fields a label holds.

use compact_str::CompactString;
use djs_model::{
    component::{
        Checkbox, CheckboxGroup, Component, FileUpload, GroupOption, Label, RadioGroup,
        TextDisplay, TextInput, TextInputStyle,
    },
    interaction::ModalCallbackData,
};

use super::validate;
use crate::ValidationError;

/// Builds a [modal](https://docs.discord.com/developers/interactions/receiving-and-responding#interaction-response-object-modal):
/// a pop-up form your app shows in answer to an interaction.
///
/// A modal has a `custom_id`, a title (1-45 characters) and 1-5 labels and text displays
/// (D-206, D-207). Each [label](LabelBuilder) holds one field: a
/// [text input](TextInputBuilder), a select menu, a [file upload](FileUploadBuilder), a
/// [radio group](RadioGroupBuilder), a [checkbox group](CheckboxGroupBuilder) or a
/// [checkbox](CheckboxBuilder). [`label`](Self::label) adds a label without a description;
/// [`component`](Self::component) takes a [`LabelBuilder`] with one.
///
/// The rules come from Discord's documentation only: a modal cannot be sent without a user's
/// interaction, so they were not tried on the live API (D-200).
///
/// discord.js counterpart: `ModalBuilder`. Unlike discord.js v14, action rows with text inputs
/// (the deprecated format) are rejected (D-207).
///
/// # Examples
///
/// ```
/// use djs_builders::{
///     FileUploadBuilder, LabelBuilder, ModalBuilder, RadioGroupBuilder, TextInputBuilder,
/// };
///
/// # fn main() -> Result<(), djs_builders::ValidationError> {
/// let modal = ModalBuilder::new("feedback", "Your feedback")
///     .text("Thanks for helping!")
///     .label("Topic", TextInputBuilder::short("topic").max_length(80))
///     .component(
///         LabelBuilder::new("Details", TextInputBuilder::paragraph("body").required(false))
///             .description("Optional"),
///     )
///     .label(
///         "Rating",
///         RadioGroupBuilder::new("rating").option("Good", "good").option("Bad", "bad"),
///     )
///     .label("Screenshots", FileUploadBuilder::new("shots").max_values(3))
///     .build()?;
/// assert_eq!(modal.components.len(), 5);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until `build` is called"]
pub struct ModalBuilder {
    modal: ModalCallbackData,
}

impl ModalBuilder {
    /// Creates a modal sending `custom_id` (1-100 characters) back when submitted, titled
    /// `title` (1-45 characters). It needs 1-5 labels and texts.
    ///
    /// discord.js counterpart: `new ModalBuilder().setCustomId(…).setTitle(…)`.
    pub fn new(custom_id: impl Into<CompactString>, title: impl Into<CompactString>) -> Self {
        Self {
            modal: ModalCallbackData::new(custom_id, title, Vec::new()),
        }
    }

    /// Adds a label `label` (1-45 characters) around `field`, without a description.
    ///
    /// discord.js counterpart: `addLabelComponents`.
    pub fn label(self, label: impl Into<CompactString>, field: impl Into<Component>) -> Self {
        self.component(LabelBuilder::new(label, field))
    }

    /// Adds a text display showing `content` (markdown, 1-4000 characters).
    ///
    /// discord.js counterpart: `addTextDisplayComponents`.
    pub fn text(self, content: impl Into<CompactString>) -> Self {
        self.component(TextDisplay::new(content))
    }

    /// Adds a component after those already added: a [`LabelBuilder`], a text display, or a
    /// `djs-model` [`Component`] of those types.
    ///
    /// discord.js counterpart: `addLabelComponents` / `addTextDisplayComponents`.
    pub fn component(mut self, component: impl Into<Component>) -> Self {
        self.modal.components.push(component.into());
        self
    }

    /// Checks the modal and everything in it, and returns it ready for
    /// `InteractionResponse::Modal`.
    ///
    /// discord.js counterpart: `ModalBuilder#toJSON`.
    ///
    /// # Errors
    ///
    /// The first [`ValidationError`] found: the `custom_id`, the title, the number and types
    /// of components, each label and field (paths such as `components[1].component.max_length`),
    /// then repeated `custom_id`s.
    pub fn build(self) -> Result<ModalCallbackData, ValidationError> {
        validate::modal(&self.modal)?;
        Ok(self.modal)
    }

    /// Returns the modal without any check (D-180) — also the way to send the deprecated
    /// action-row format (D-207).
    #[must_use]
    pub fn build_unchecked(self) -> ModalCallbackData {
        self.modal
    }
}

/// Builds a [label](https://docs.discord.com/developers/components/reference#label): a title
/// (1-45 characters) and an optional description (at most 100) around one field of a modal.
///
/// The field is given to [`new`](Self::new), like a section's accessory (D-204, D-206).
///
/// discord.js counterpart: `LabelBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a modal or built"]
pub struct LabelBuilder {
    component: Label,
}

impl LabelBuilder {
    /// Creates a label `label` around `field`: a text input, a select menu, a file upload, a
    /// radio group, a checkbox group or a checkbox.
    ///
    /// discord.js counterpart: `new LabelBuilder().setLabel(…).set…Component(…)`.
    pub fn new(label: impl Into<CompactString>, field: impl Into<Component>) -> Self {
        Self {
            component: Label::new(label, field.into()),
        }
    }

    /// Sets the smaller text under the label (at most 100 characters).
    ///
    /// discord.js counterpart: `setDescription`.
    pub fn description(mut self, description: impl Into<CompactString>) -> Self {
        self.component.description = Some(description.into());
        self
    }
}

component_builder!(LabelBuilder => Label);

/// Builds a [text input](https://docs.discord.com/developers/components/reference#text-input)
/// for a modal's [label](LabelBuilder): one line ([`short`](Self::short)) or several
/// ([`paragraph`](Self::paragraph)).
///
/// `min_length` is 0-4000, `max_length` 1-4000 and at least `min_length`; the pre-filled value
/// has at most 4000 characters and the placeholder at most 100.
///
/// discord.js counterpart: `TextInputBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a label or built"]
pub struct TextInputBuilder {
    component: TextInput,
}

impl TextInputBuilder {
    /// Creates a one-line, required text input sending `custom_id` (1-100 characters).
    ///
    /// discord.js counterpart: `setStyle(TextInputStyle.Short).setCustomId(…)`.
    pub fn short(custom_id: impl Into<CompactString>) -> Self {
        Self {
            component: TextInput::new(custom_id, TextInputStyle::Short),
        }
    }

    /// Creates a multi-line, required text input sending `custom_id` (1-100 characters).
    ///
    /// discord.js counterpart: `setStyle(TextInputStyle.Paragraph).setCustomId(…)`.
    pub fn paragraph(custom_id: impl Into<CompactString>) -> Self {
        Self {
            component: TextInput::new(custom_id, TextInputStyle::Paragraph),
        }
    }

    /// Sets the shortest text users may enter (0-4000).
    ///
    /// discord.js counterpart: `setMinLength`.
    pub fn min_length(mut self, min_length: u16) -> Self {
        self.component.min_length = Some(min_length);
        self
    }

    /// Sets the longest text users may enter (1-4000, at least the
    /// [`min_length`](Self::min_length)).
    ///
    /// discord.js counterpart: `setMaxLength`.
    pub fn max_length(mut self, max_length: u16) -> Self {
        self.component.max_length = Some(max_length);
        self
    }

    /// Sets whether the input must be filled to submit the modal (default: yes).
    ///
    /// discord.js counterpart: `setRequired`.
    pub fn required(mut self, required: bool) -> Self {
        self.component.required = required;
        self
    }

    /// Sets the text already filled in (at most 4000 characters).
    ///
    /// discord.js counterpart: `setValue`.
    pub fn value(mut self, value: impl Into<CompactString>) -> Self {
        self.component.value = Some(value.into());
        self
    }

    /// Sets the text shown while the input is empty (at most 100 characters).
    ///
    /// discord.js counterpart: `setPlaceholder`.
    pub fn placeholder(mut self, placeholder: impl Into<CompactString>) -> Self {
        self.component.placeholder = Some(placeholder.into());
        self
    }
}

component_builder!(TextInputBuilder => TextInput);

/// Builds a [file upload](https://docs.discord.com/developers/components/reference#file-upload)
/// for a modal's [label](LabelBuilder): users attach 0-10 files.
///
/// `min_values` is 0-10 (at least 1 while the upload is required), `max_values` 1-10 and at
/// least `min_values`; at most 10 [file types](Self::file_types).
///
/// discord.js counterpart: `FileUploadBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a label or built"]
pub struct FileUploadBuilder {
    component: FileUpload,
}

impl FileUploadBuilder {
    /// Creates a required upload of one file, sending `custom_id` (1-100 characters).
    ///
    /// discord.js counterpart: `new FileUploadBuilder().setCustomId(…)`.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            component: FileUpload::new(custom_id),
        }
    }

    /// Sets how many files must be uploaded at least (0-10; Discord's default: 1).
    ///
    /// discord.js counterpart: `setMinValues`.
    pub fn min_values(mut self, min_values: u8) -> Self {
        self.component.min_values = Some(min_values);
        self
    }

    /// Sets how many files can be uploaded at most (1-10; Discord's default: 1).
    ///
    /// discord.js counterpart: `setMaxValues`.
    pub fn max_values(mut self, max_values: u8) -> Self {
        self.component.max_values = Some(max_values);
        self
    }

    /// Sets whether files must be uploaded to submit the modal (default: yes).
    ///
    /// discord.js counterpart: `setRequired`.
    pub fn required(mut self, required: bool) -> Self {
        self.component.required = required;
        self
    }

    /// Sets the [file types](https://docs.discord.com/developers/reference#file-type-filtering)
    /// users may pick — `image`, `video`, `audio` or extensions such as `.pdf`, at most 10 —,
    /// replacing those set before.
    ///
    /// discord.js counterpart: `setFileTypes`.
    pub fn file_types<T: Into<CompactString>>(
        mut self,
        file_types: impl IntoIterator<Item = T>,
    ) -> Self {
        self.component.file_types = file_types.into_iter().map(Into::into).collect();
        self
    }
}

component_builder!(FileUploadBuilder => FileUpload);

/// Defines an option builder of a radio or checkbox group (`GroupOption`).
macro_rules! group_option {
    ($(#[$meta:meta])* $builder:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        #[must_use = "an option does nothing until it is added to a group"]
        pub struct $builder {
            option: GroupOption,
        }

        impl $builder {
            /// Creates an option showing `label` (1-100 characters) and sending `value`
            /// (1-100 characters, unique in the group).
            ///
            /// discord.js counterpart: `setLabel(…).setValue(…)`.
            pub fn new(label: impl Into<CompactString>, value: impl Into<CompactString>) -> Self {
                Self {
                    option: GroupOption::new(label, value),
                }
            }

            /// Sets the smaller text under the label (at most 100 characters).
            ///
            /// discord.js counterpart: `setDescription`.
            pub fn description(mut self, description: impl Into<CompactString>) -> Self {
                self.option.description = Some(description.into());
                self
            }

            /// Sets whether the option is selected when the modal opens.
            ///
            /// discord.js counterpart: `setDefault`.
            pub fn default(mut self, default: bool) -> Self {
                self.option.default = default;
                self
            }
        }

        impl From<$builder> for GroupOption {
            fn from(option: $builder) -> Self {
                option.option
            }
        }
    };
}

group_option! {
    /// Builds an option of a [radio group](RadioGroupBuilder) with a description or selected
    /// by default, for [`RadioGroupBuilder::option_with`].
    ///
    /// discord.js counterpart: `RadioGroupOptionBuilder`.
    RadioGroupOptionBuilder
}

group_option! {
    /// Builds an option of a [checkbox group](CheckboxGroupBuilder) with a description or
    /// checked by default, for [`CheckboxGroupBuilder::option_with`].
    ///
    /// discord.js counterpart: `CheckboxGroupOptionBuilder`.
    CheckboxGroupOptionBuilder
}

/// Builds a [radio group](https://docs.discord.com/developers/components/reference#radio-group)
/// for a modal's [label](LabelBuilder): users pick exactly one of 2-10 options.
///
/// At most one option can be selected by default.
///
/// discord.js counterpart: `RadioGroupBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a label or built"]
pub struct RadioGroupBuilder {
    component: RadioGroup,
}

impl RadioGroupBuilder {
    /// Creates a required group without options, sending `custom_id` (1-100 characters).
    ///
    /// discord.js counterpart: `new RadioGroupBuilder().setCustomId(…)`.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            component: RadioGroup::new(custom_id, Vec::new()),
        }
    }

    /// Adds an option showing `label` and sending `value`.
    ///
    /// discord.js counterpart: `addOptions({ label, value })`.
    pub fn option(
        mut self,
        label: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.component.options.push(GroupOption::new(label, value));
        self
    }

    /// Adds an option with a description or selected by default.
    ///
    /// discord.js counterpart: `addOptions(new RadioGroupOptionBuilder()…)`.
    pub fn option_with(mut self, option: RadioGroupOptionBuilder) -> Self {
        self.component.options.push(option.into());
        self
    }

    /// Adds several options: option builders or `djs-model` [`GroupOption`]s.
    ///
    /// discord.js counterpart: `addOptions`.
    pub fn options<O: Into<GroupOption>>(mut self, options: impl IntoIterator<Item = O>) -> Self {
        self.component
            .options
            .extend(options.into_iter().map(Into::into));
        self
    }

    /// Sets whether a choice is needed to submit the modal (default: yes).
    ///
    /// discord.js counterpart: `setRequired`.
    pub fn required(mut self, required: bool) -> Self {
        self.component.required = required;
        self
    }
}

component_builder!(RadioGroupBuilder => RadioGroup);

/// Builds a [checkbox group](https://docs.discord.com/developers/components/reference#checkbox-group)
/// for a modal's [label](LabelBuilder): users check some of 1-10 options.
///
/// `min_values` is 0-10 (at least 1 while the group is required) and `max_values` 1-10 (by
/// default all options), with at least `max_values` options.
///
/// discord.js counterpart: `CheckboxGroupBuilder`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a label or built"]
pub struct CheckboxGroupBuilder {
    component: CheckboxGroup,
}

impl CheckboxGroupBuilder {
    /// Creates a required group without options, sending `custom_id` (1-100 characters).
    ///
    /// discord.js counterpart: `new CheckboxGroupBuilder().setCustomId(…)`.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            component: CheckboxGroup::new(custom_id, Vec::new()),
        }
    }

    /// Adds an option showing `label` and sending `value`.
    ///
    /// discord.js counterpart: `addOptions({ label, value })`.
    pub fn option(
        mut self,
        label: impl Into<CompactString>,
        value: impl Into<CompactString>,
    ) -> Self {
        self.component.options.push(GroupOption::new(label, value));
        self
    }

    /// Adds an option with a description or checked by default.
    ///
    /// discord.js counterpart: `addOptions(new CheckboxGroupOptionBuilder()…)`.
    pub fn option_with(mut self, option: CheckboxGroupOptionBuilder) -> Self {
        self.component.options.push(option.into());
        self
    }

    /// Adds several options: option builders or `djs-model` [`GroupOption`]s.
    ///
    /// discord.js counterpart: `addOptions`.
    pub fn options<O: Into<GroupOption>>(mut self, options: impl IntoIterator<Item = O>) -> Self {
        self.component
            .options
            .extend(options.into_iter().map(Into::into));
        self
    }

    /// Sets how many options must be checked at least (0-10; Discord's default: 1).
    ///
    /// discord.js counterpart: `setMinValues`.
    pub fn min_values(mut self, min_values: u8) -> Self {
        self.component.min_values = Some(min_values);
        self
    }

    /// Sets how many options can be checked at most (1-10; Discord's default: all).
    ///
    /// discord.js counterpart: `setMaxValues`.
    pub fn max_values(mut self, max_values: u8) -> Self {
        self.component.max_values = Some(max_values);
        self
    }

    /// Sets whether checking is needed to submit the modal (default: yes).
    ///
    /// discord.js counterpart: `setRequired`.
    pub fn required(mut self, required: bool) -> Self {
        self.component.required = required;
        self
    }
}

component_builder!(CheckboxGroupBuilder => CheckboxGroup);

/// Builds a [checkbox](https://docs.discord.com/developers/components/reference#checkbox) for
/// a modal's [label](LabelBuilder): a single yes/no choice, never required (use a one-option
/// [checkbox group](CheckboxGroupBuilder) for that).
///
/// discord.js counterpart: `CheckboxBuilder`.
///
/// # Examples
///
/// ```
/// use djs_builders::{CheckboxBuilder, ModalBuilder};
///
/// let modal = ModalBuilder::new("signup", "Sign up")
///     .label("Send me the newsletter", CheckboxBuilder::new("newsletter").default(true))
///     .build()
///     .unwrap();
/// assert_eq!(modal.components.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a builder does nothing until it is added to a label or built"]
pub struct CheckboxBuilder {
    component: Checkbox,
}

impl CheckboxBuilder {
    /// Creates an unchecked checkbox sending `custom_id` (1-100 characters).
    ///
    /// discord.js counterpart: `new CheckboxBuilder().setCustomId(…)`.
    pub fn new(custom_id: impl Into<CompactString>) -> Self {
        Self {
            component: Checkbox::new(custom_id),
        }
    }

    /// Sets whether the checkbox is checked when the modal opens.
    ///
    /// discord.js counterpart: `setDefault`.
    pub fn default(mut self, default: bool) -> Self {
        self.component.default = default;
        self
    }
}

component_builder!(CheckboxBuilder => Checkbox);
