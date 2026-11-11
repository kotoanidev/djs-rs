//! Query strings of endpoints (D-092): the query structs of `djs_model::rest` are written
//! straight into the request URI by the small encoder below, without a dependency.

use core::fmt::{self, Write as _};

use djs_model::{
    id::Id,
    message::ReactionType,
    rest::{
        GetApplicationCommands, GetChannelMessages, GetChannelPermissions, GetChannelPins,
        GetReactions, GetThreadMember, ListArchivedThreads, ListJoinedPrivateArchivedThreads,
        ListThreadMembers, SearchAuthorType, SearchEmbedType, SearchFilter, SearchGuildMessages,
        SearchHasType, SearchSortMode, SearchSortOrder,
    },
    util::Timestamp,
};

use crate::route::write_text;

/// The query parameters of an endpoint.
pub(crate) trait Query: fmt::Debug + Sync {
    /// Writes the parameters that are set.
    fn write(&self, query: &mut QueryWriter<'_>);
}

/// Appends `name=value` pairs to a URI, starting with `?`.
pub(crate) struct QueryWriter<'a> {
    out: &'a mut String,
    empty: bool,
}

impl<'a> QueryWriter<'a> {
    /// A writer appending to `out`, which holds the URI up to its path.
    pub(crate) const fn new(out: &'a mut String) -> Self {
        Self { out, empty: true }
    }

    /// Writes a parameter.
    pub(crate) fn pair(&mut self, name: &str, value: &(impl QueryValue + ?Sized)) {
        self.out.push(if self.empty { '?' } else { '&' });
        self.empty = false;
        self.out.push_str(name);
        self.out.push('=');
        value.write(self.out);
    }

    /// Writes a list parameter: the name repeated for every value (`?id=1&id=2`), Discord's
    /// [array query string](https://docs.discord.com/developers/reference#array-query-strings).
    pub(crate) fn each<V: QueryValue>(&mut self, name: &str, values: &[V]) {
        for value in values {
            self.pair(name, value);
        }
    }

    /// Writes a parameter if it is set.
    pub(crate) fn optional<V: QueryValue + ?Sized>(&mut self, name: &str, value: Option<&V>) {
        if let Some(value) = value {
            self.pair(name, value);
        }
    }
}

/// A value that can appear in a query string.
pub(crate) trait QueryValue {
    /// Writes the value, percent-encoded where needed.
    fn write(&self, out: &mut String);
}

macro_rules! display_values {
    ($($ty:ty),*) => {$(
        impl QueryValue for $ty {
            fn write(&self, out: &mut String) {
                // Writing to a `String` cannot fail.
                let _ = write!(out, "{self}");
            }
        }
    )*};
}

display_values!(u8, u16, u32, u64, i32, i64, bool);

impl<M> QueryValue for Id<M> {
    fn write(&self, out: &mut String) {
        self.get().write(out);
    }
}

impl QueryValue for str {
    fn write(&self, out: &mut String) {
        write_text(out, self);
    }
}

impl QueryValue for Timestamp {
    /// ISO 8601, percent-encoded while it is written (no intermediate string).
    fn write(&self, out: &mut String) {
        struct Encoder<'a>(&'a mut String);

        impl fmt::Write for Encoder<'_> {
            fn write_str(&mut self, text: &str) -> fmt::Result {
                write_text(self.0, text);
                Ok(())
            }
        }

        let _ = write!(Encoder(out), "{self}");
    }
}

macro_rules! str_enum_values {
    ($($ty:ty),*) => {$(
        impl QueryValue for $ty {
            fn write(&self, out: &mut String) {
                write_text(out, self.as_str());
            }
        }
    )*};
}

str_enum_values!(
    SearchAuthorType,
    SearchHasType,
    SearchEmbedType,
    SearchSortMode,
    SearchSortOrder
);

impl<T: QueryValue> QueryValue for SearchFilter<T> {
    /// An exclusion is written with a leading `-`.
    fn write(&self, out: &mut String) {
        match self {
            Self::Include(value) => value.write(out),
            Self::Exclude(value) => {
                out.push('-');
                value.write(out);
            }
        }
    }
}

impl QueryValue for ReactionType {
    fn write(&self, out: &mut String) {
        self.value().write(out);
    }
}

impl<V: QueryValue + ?Sized> QueryValue for &V {
    fn write(&self, out: &mut String) {
        (**self).write(out);
    }
}

impl Query for GetChannelMessages {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("around", self.around.as_ref());
        query.optional("before", self.before.as_ref());
        query.optional("after", self.after.as_ref());
        query.optional("limit", self.limit.as_ref());
    }
}

impl Query for GetReactions {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("type", self.kind.as_ref());
        query.optional("after", self.after.as_ref());
        query.optional("limit", self.limit.as_ref());
    }
}

impl Query for GetChannelPins {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("before", self.before.as_ref());
        query.optional("limit", self.limit.as_ref());
    }
}

impl Query for GetApplicationCommands {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("with_localizations", self.with_localizations.as_ref());
    }
}

impl Query for GetChannelPermissions {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("implicit", self.implicit.as_ref());
    }
}

impl Query for GetThreadMember {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("with_member", self.with_member.as_ref());
    }
}

impl Query for ListThreadMembers {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("with_member", self.with_member.as_ref());
        query.optional("after", self.after.as_ref());
        query.optional("limit", self.limit.as_ref());
    }
}

impl Query for ListArchivedThreads {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("before", self.before.as_ref());
        query.optional("limit", self.limit.as_ref());
    }
}

impl Query for ListJoinedPrivateArchivedThreads {
    fn write(&self, query: &mut QueryWriter<'_>) {
        query.optional("before", self.before.as_ref());
        query.optional("limit", self.limit.as_ref());
    }
}

impl Query for SearchGuildMessages {
    fn write(&self, query: &mut QueryWriter<'_>) {
        // `..` is required for a `#[non_exhaustive]` struct of another crate; the test
        // `search_query_writes_every_field` catches a forgotten field.
        let Self {
            limit,
            offset,
            max_id,
            min_id,
            slop,
            content,
            channel_id,
            author_type,
            author_id,
            mentions,
            mentions_role_id,
            mention_everyone,
            replied_to_user_id,
            replied_to_message_id,
            pinned,
            has,
            embed_type,
            embed_provider,
            link_hostname,
            attachment_filename,
            attachment_extension,
            sort_by,
            sort_order,
            include_nsfw,
            ..
        } = self;
        query.optional("limit", limit.as_ref());
        query.optional("offset", offset.as_ref());
        query.optional("max_id", max_id.as_ref());
        query.optional("min_id", min_id.as_ref());
        query.optional("slop", slop.as_ref());
        query.optional("content", content.as_deref());
        query.each("channel_id", channel_id);
        query.each("author_type", author_type);
        query.each("author_id", author_id);
        query.each("mentions", mentions);
        query.each("mentions_role_id", mentions_role_id);
        query.optional("mention_everyone", mention_everyone.as_ref());
        query.each("replied_to_user_id", replied_to_user_id);
        query.each("replied_to_message_id", replied_to_message_id);
        query.optional("pinned", pinned.as_ref());
        query.each("has", has);
        query.each("embed_type", embed_type);
        for (name, values) in [
            ("embed_provider", embed_provider),
            ("link_hostname", link_hostname),
            ("attachment_filename", attachment_filename),
            ("attachment_extension", attachment_extension),
        ] {
            for value in values {
                query.pair(name, value.as_str());
            }
        }
        query.optional("sort_by", sort_by.as_ref());
        query.optional("sort_order", sort_order.as_ref());
        query.optional("include_nsfw", include_nsfw.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use djs_model::id::MessageId;

    use super::*;

    #[derive(Debug)]
    struct Example {
        before: Option<MessageId>,
        limit: Option<u8>,
        query: Option<String>,
        with_counts: bool,
    }

    impl Query for Example {
        fn write(&self, query: &mut QueryWriter<'_>) {
            query.optional("before", self.before.as_ref());
            query.optional("limit", self.limit.as_ref());
            query.optional("query", self.query.as_deref());
            if self.with_counts {
                query.pair("with_counts", &true);
            }
        }
    }

    fn uri(example: &Example) -> String {
        let mut out = "/channels/1/messages".to_owned();
        example.write(&mut QueryWriter::new(&mut out));
        out
    }

    #[test]
    fn writes_the_parameters_that_are_set() {
        let mut example = Example {
            before: None,
            limit: None,
            query: None,
            with_counts: false,
        };
        assert_eq!(uri(&example), "/channels/1/messages");
        example.limit = Some(50);
        assert_eq!(uri(&example), "/channels/1/messages?limit=50");
        example.before = Some(MessageId::new(1_300_000_000_000_000_000));
        example.query = Some("a&b=c d/ü".into());
        example.with_counts = true;
        assert_eq!(
            uri(&example),
            "/channels/1/messages?before=1300000000000000000&limit=50\
             &query=a%26b%3Dc%20d%2F%C3%BC&with_counts=true"
        );
    }

    fn query(query: &dyn Query) -> String {
        let mut out = String::new();
        query.write(&mut QueryWriter::new(&mut out));
        out
    }

    #[test]
    fn search_query() {
        let mut search = SearchGuildMessages::default();
        assert_eq!(query(&search), "");
        search.content = Some("new release".into());
        search.channel_id = vec![
            djs_model::id::ChannelId::new(1),
            djs_model::id::ChannelId::new(2),
        ];
        search.author_type = vec![
            SearchFilter::Include(SearchAuthorType::User),
            SearchFilter::Exclude(SearchAuthorType::Bot),
        ];
        search.has = vec![SearchHasType::Link.into()];
        search.link_hostname = vec!["discord.com".into()];
        search.sort_by = Some(SearchSortMode::Relevance);
        search.include_nsfw = Some(false);
        assert_eq!(
            query(&search),
            "?content=new%20release&channel_id=1&channel_id=2&author_type=user\
             &author_type=-bot&has=link&link_hostname=discord.com&sort_by=relevance\
             &include_nsfw=false"
        );
    }

    #[test]
    fn search_query_writes_every_field() {
        use djs_model::id::{ChannelId, RoleId, UserId};

        let mut search = SearchGuildMessages::default();
        search.limit = Some(1);
        search.offset = Some(2);
        search.max_id = Some(MessageId::new(3));
        search.min_id = Some(MessageId::new(4));
        search.slop = Some(5);
        search.content = Some("c".into());
        search.channel_id = vec![ChannelId::new(6)];
        search.author_type = vec![SearchAuthorType::Webhook.into()];
        search.author_id = vec![UserId::new(7)];
        search.mentions = vec![UserId::new(8)];
        search.mentions_role_id = vec![RoleId::new(9)];
        search.mention_everyone = Some(true);
        search.replied_to_user_id = vec![UserId::new(10)];
        search.replied_to_message_id = vec![MessageId::new(11)];
        search.pinned = Some(true);
        search.has = vec![SearchFilter::Exclude(SearchHasType::Poll)];
        search.embed_type = vec![SearchEmbedType::Gif];
        search.embed_provider = vec!["Tenor".into()];
        search.link_hostname = vec!["h".into()];
        search.attachment_filename = vec!["f".into()];
        search.attachment_extension = vec!["txt".into()];
        search.sort_by = Some(SearchSortMode::Timestamp);
        search.sort_order = Some(SearchSortOrder::Ascending);
        search.include_nsfw = Some(true);
        let written = query(&search);
        let names: Vec<&str> = written
            .trim_start_matches('?')
            .split('&')
            .map(|pair| pair.split('=').next().unwrap())
            .collect();
        assert_eq!(names.len(), 24, "{written}");
        assert!(
            written.ends_with(
                "&has=-poll&embed_type=gif&embed_provider=Tenor&link_hostname=h\
             &attachment_filename=f&attachment_extension=txt&sort_by=timestamp&sort_order=asc\
             &include_nsfw=true"
            ),
            "{written}"
        );
    }

    #[test]
    fn message_queries() {
        let mut messages = GetChannelMessages::default();
        assert_eq!(query(&messages), "");
        messages.around = Some(MessageId::new(5));
        messages.limit = Some(100);
        assert_eq!(query(&messages), "?around=5&limit=100");

        let mut reactions = GetReactions::default();
        reactions.kind = Some(ReactionType::Burst);
        reactions.after = Some(djs_model::id::UserId::new(7));
        assert_eq!(query(&reactions), "?type=1&after=7");

        let mut pins = GetChannelPins::default();
        pins.before = Some("2026-11-11T12:00:00+00:00".parse().unwrap());
        pins.limit = Some(50);
        assert_eq!(
            query(&pins),
            "?before=2026-11-11T12%3A00%3A00.000000%2B00%3A00&limit=50"
        );
    }
}
