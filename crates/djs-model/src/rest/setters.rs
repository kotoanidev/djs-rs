//! The chainable setters of the message bodies (D-290, D-291).
//!
//! The bodies hold their fields in three shapes: `Option<T>` (absent or set), [`Patch<T>`]
//! (kept, cleared or set, for edits) and lists as `Vec<T>`, `Option<Vec<T>>` or
//! `Patch<Vec<T>>`. [`Slot`] and [`List`] write into any of them, so one setter macro serves
//! every body and a setter does the same thing whatever the field's shape.

use crate::{message::MessageFlags, util::Patch};

/// A field holding at most one value: `Option<T>` or [`Patch<T>`].
pub(crate) trait Slot<T> {
    /// Sets the value.
    fn put(&mut self, value: T);

    /// The value, if one is set.
    fn value(&self) -> Option<&T>;
}

impl<T> Slot<T> for Option<T> {
    fn put(&mut self, value: T) {
        *self = Some(value);
    }

    fn value(&self) -> Option<&T> {
        self.as_ref()
    }
}

impl<T> Slot<T> for Patch<T> {
    fn put(&mut self, value: T) {
        *self = Self::Set(value);
    }

    fn value(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Keep | Self::Clear => None,
        }
    }
}

/// A list field: `Vec<T>`, `Option<Vec<T>>` or `Patch<Vec<T>>`.
pub(crate) trait List<T> {
    /// Appends `item`, keeping the items set before.
    fn push_item(&mut self, item: T);

    /// Replaces the list (an empty one included: on an edit it removes the items).
    fn replace(&mut self, items: Vec<T>);
}

impl<T> List<T> for Vec<T> {
    fn push_item(&mut self, item: T) {
        self.push(item);
    }

    fn replace(&mut self, items: Vec<T>) {
        *self = items;
    }
}

impl<T> List<T> for Option<Vec<T>> {
    fn push_item(&mut self, item: T) {
        self.get_or_insert_with(Vec::new).push(item);
    }

    fn replace(&mut self, items: Vec<T>) {
        *self = Some(items);
    }
}

impl<T> List<T> for Patch<Vec<T>> {
    fn push_item(&mut self, item: T) {
        match self {
            Self::Set(items) => items.push(item),
            Self::Keep | Self::Clear => *self = Self::Set(vec![item]),
        }
    }

    fn replace(&mut self, items: Vec<T>) {
        *self = Self::Set(items);
    }
}

/// Adds `flags` to the flags set before.
pub(crate) fn insert_flags(slot: &mut impl Slot<MessageFlags>, flags: MessageFlags) {
    let current = slot.value().copied().unwrap_or_default();
    slot.put(current | flags);
}

/// Writes the chainable setters of a message body: the shared ones named in the list, from
/// `content`, `embeds`, `components`, `allowed_mentions`, `flags`, `files`, `attachments`,
/// `poll`, `tts`, `ephemeral` and `suppress_notifications`, and the three `From` conversions
/// of its text.
macro_rules! message_body {
    ($body:ident { $($setter:ident),* $(,)? }) => {
        impl $body {
            $($crate::rest::setters::message_body!(@ $setter);)*
        }

        impl From<&str> for $body {
            fn from(content: &str) -> Self {
                Self::default().content(content)
            }
        }

        impl From<String> for $body {
            fn from(content: String) -> Self {
                Self::default().content(content)
            }
        }

        impl From<::compact_str::CompactString> for $body {
            fn from(content: ::compact_str::CompactString) -> Self {
                Self::default().content(content)
            }
        }
    };
    (@ content) => {
        /// Sets the text (up to 2000 characters).
        ///
        /// discord.js counterpart: the `content` option.
        #[must_use]
        pub fn content(mut self, content: impl Into<::compact_str::CompactString>) -> Self {
            $crate::rest::setters::Slot::put(&mut self.content, content.into());
            self
        }
    };
    (@ embeds) => {
        /// Adds an embed after the ones set before (up to 10, 6000 characters in all; check
        /// them with `djs_builders::validate_embeds`).
        ///
        /// discord.js counterpart: an item of the `embeds` option.
        #[must_use]
        pub fn embed(mut self, embed: $crate::message::Embed) -> Self {
            $crate::rest::setters::List::push_item(&mut self.embeds, embed);
            self
        }

        /// Replaces the embeds. When editing a message (or updating it from a component),
        /// an empty list removes its embeds, while not calling this keeps them.
        ///
        /// discord.js counterpart: the `embeds` option.
        #[must_use]
        pub fn embeds(
            mut self,
            embeds: impl IntoIterator<Item = $crate::message::Embed>,
        ) -> Self {
            $crate::rest::setters::List::replace(&mut self.embeds, embeds.into_iter().collect());
            self
        }
    };
    (@ components) => {
        /// Adds a top-level component (an action row, or a Components V2 item with the
        /// [`IS_COMPONENTS_V2`]($crate::message::MessageFlags::IS_COMPONENTS_V2) flag) after
        /// the ones set before.
        ///
        /// discord.js counterpart: an item of the `components` option.
        #[must_use]
        pub fn component(mut self, component: impl Into<$crate::component::Component>) -> Self {
            $crate::rest::setters::List::push_item(&mut self.components, component.into());
            self
        }

        /// Replaces the top-level components. When editing a message (or updating it from a
        /// component), an empty list removes its components, while not calling this keeps
        /// them.
        ///
        /// discord.js counterpart: the `components` option.
        #[must_use]
        pub fn components(
            mut self,
            components: impl IntoIterator<Item = $crate::component::Component>,
        ) -> Self {
            $crate::rest::setters::List::replace(
                &mut self.components,
                components.into_iter().collect(),
            );
            self
        }
    };
    (@ allowed_mentions) => {
        /// Sets which mentions of the text notify their targets.
        ///
        /// discord.js counterpart: the `allowedMentions` option.
        #[must_use]
        pub fn allowed_mentions(mut self, mentions: $crate::message::AllowedMentions) -> Self {
            $crate::rest::setters::Slot::put(&mut self.allowed_mentions, mentions);
            self
        }

        /// Lets no mention notify anyone: no `@everyone`, role or user ping, whatever the text
        /// contains. The safe choice for text that comes from users.
        ///
        /// discord.js counterpart: `allowedMentions: { parse: [] }`.
        #[must_use]
        pub fn no_mentions(self) -> Self {
            self.allowed_mentions($crate::message::AllowedMentions::default())
        }
    };
    (@ flags) => {
        /// Replaces the message flags (only some can be sent; see
        /// [`MessageFlags`]($crate::message::MessageFlags)).
        ///
        /// discord.js counterpart: the `flags` option.
        #[must_use]
        pub fn flags(mut self, flags: $crate::message::MessageFlags) -> Self {
            $crate::rest::setters::Slot::put(&mut self.flags, flags);
            self
        }

        /// Hides the embeds Discord would generate for the links in the text (adds
        /// [`SUPPRESS_EMBEDS`]($crate::message::MessageFlags::SUPPRESS_EMBEDS) to the flags).
        ///
        /// discord.js counterpart: `flags: MessageFlags.SuppressEmbeds`.
        #[must_use]
        pub fn suppress_embeds(mut self) -> Self {
            $crate::rest::setters::insert_flags(
                &mut self.flags,
                $crate::message::MessageFlags::SUPPRESS_EMBEDS,
            );
            self
        }
    };
    (@ ephemeral) => {
        /// Makes the message visible only to the user of the interaction (adds
        /// [`EPHEMERAL`]($crate::message::MessageFlags::EPHEMERAL) to the flags).
        ///
        /// discord.js counterpart: `flags: MessageFlags.Ephemeral`.
        #[must_use]
        pub fn ephemeral(mut self) -> Self {
            $crate::rest::setters::insert_flags(
                &mut self.flags,
                $crate::message::MessageFlags::EPHEMERAL,
            );
            self
        }
    };
    (@ suppress_notifications) => {
        /// Sends the message without push and desktop notifications (adds
        /// [`SUPPRESS_NOTIFICATIONS`]($crate::message::MessageFlags::SUPPRESS_NOTIFICATIONS) to
        /// the flags; "@silent" in the client).
        ///
        /// discord.js counterpart: `flags: MessageFlags.SuppressNotifications`.
        #[must_use]
        pub fn suppress_notifications(mut self) -> Self {
            $crate::rest::setters::insert_flags(
                &mut self.flags,
                $crate::message::MessageFlags::SUPPRESS_NOTIFICATIONS,
            );
            self
        }
    };
    (@ files) => {
        /// Attaches a file, after the ones set before.
        ///
        /// discord.js counterpart: an item of the `files` option.
        #[must_use]
        pub fn file(mut self, file: $crate::rest::FileUpload) -> Self {
            self.files.push(file);
            self
        }

        /// Replaces the attached files.
        ///
        /// discord.js counterpart: the `files` option.
        #[must_use]
        pub fn files(mut self, files: impl IntoIterator<Item = $crate::rest::FileUpload>) -> Self {
            self.files = files.into_iter().collect();
            self
        }
    };
    (@ attachments) => {
        /// Sets the attachment metadata: descriptions of the uploaded files, or, when editing,
        /// the existing attachments to keep (an empty list removes them all).
        ///
        /// discord.js counterpart: the `attachments` option.
        #[must_use]
        pub fn attachments(
            mut self,
            attachments: impl IntoIterator<Item = $crate::message::AttachmentRequest>,
        ) -> Self {
            $crate::rest::setters::List::replace(
                &mut self.attachments,
                attachments.into_iter().collect(),
            );
            self
        }
    };
    (@ poll) => {
        /// Attaches a poll.
        ///
        /// discord.js counterpart: the `poll` option.
        #[must_use]
        pub fn poll(mut self, poll: $crate::poll::CreatePoll) -> Self {
            $crate::rest::setters::Slot::put(&mut self.poll, poll);
            self
        }
    };
    (@ tts) => {
        /// Sets whether the message is read aloud (needs `SEND_TTS_MESSAGES`).
        ///
        /// discord.js counterpart: the `tts` option.
        #[must_use]
        pub const fn tts(mut self, tts: bool) -> Self {
            self.tts = tts;
            self
        }
    };
}

pub(crate) use message_body;
