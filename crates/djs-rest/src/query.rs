//! Query strings of endpoints (D-092): the query structs of `djs_model::rest` are written
//! straight into the request URI by the small encoder below, without a dependency.

use core::fmt::{self, Write as _};

use djs_model::{
    id::Id,
    message::ReactionType,
    rest::{GetChannelMessages, GetChannelPins, GetReactions},
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
