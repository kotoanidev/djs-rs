//! Typed REST API routes.

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use djs_model::id::{ApplicationId, ChannelId, Id, InteractionId, MessageId};
use http::Method;

/// How the requests of a route are rate limited.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Limits {
    /// Whether the route counts towards the bot's global rate limit.
    pub(crate) global: bool,
    /// Whether requests are sent at once instead of one after another per bucket
    /// (interaction callbacks; like discord.js' `BurstHandler`). A 429 then only delays the
    /// request that received it.
    pub(crate) burst: bool,
}

impl Limits {
    /// A normal route: one request at a time per bucket, counted globally.
    pub(crate) const NORMAL: Self = Self {
        global: true,
        burst: false,
    };
    /// [Interaction endpoints](https://docs.discord.com/developers/topics/rate-limits#global-rate-limit)
    /// are not bound to the global rate limit.
    const INTERACTION: Self = Self {
        global: false,
        burst: false,
    };
    /// Interaction callbacks must be answered within 3 seconds, so they never wait for one
    /// another.
    const INTERACTION_CALLBACK: Self = Self {
        global: false,
        burst: true,
    };
}

/// How a route is grouped for rate limiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BucketRoute {
    /// A unique, static description of the route *without* its parameters, e.g.
    /// `"POST /channels/{channel_id}/messages"`. Routes sharing a template share a bucket hash.
    pub(crate) template: &'static str,
    /// The [major parameter](https://docs.discord.com/developers/topics/rate-limits) (channel,
    /// guild or webhook ID, or webhook ID + token), or `0` if the route has none. Rate limits
    /// are tracked separately per major parameter.
    pub(crate) major: u64,
    /// How the route is limited.
    pub(crate) limits: Limits,
}

/// Messages older than this are deleted under a separate, stricter rate limit.
const OLD_MESSAGE_AGE_MS: u64 = 14 * 24 * 60 * 60 * 1000;

/// The bucket template of [`Route::DeleteMessage`] for messages older than two weeks.
const OLD_MESSAGE_DELETE: &str =
    "DELETE /channels/{channel_id}/messages/{message_id} (older than two weeks)";

/// A path parameter of a route.
#[derive(Clone, Copy)]
enum Segment<'a> {
    Id(u64),
    Text(&'a str),
}

/// A value that can appear in a route's path.
trait Param: Copy {
    /// The value as a path segment.
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a;

    /// The value's contribution to the major parameter.
    fn major(self) -> u64;

    /// Writes the value for `Debug`, redacting tokens.
    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>);
}

impl<M> Param for Id<M> {
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a,
    {
        Segment::Id(self.get())
    }

    fn major(self) -> u64 {
        self.get()
    }

    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>) {
        out.field(name, &self.get());
    }
}

impl Param for &str {
    fn to_segment<'a>(self) -> Segment<'a>
    where
        Self: 'a,
    {
        Segment::Text(self)
    }

    fn major(self) -> u64 {
        // FNV-1a: a stable, cheap hash, so that every token gets its own bucket.
        self.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    fn debug(self, name: &str, out: &mut fmt::DebugStruct<'_, '_>) {
        if name.ends_with("token") {
            out.field(name, &"<redacted>");
        } else {
            out.field(name, &self);
        }
    }
}

/// Combines major parameter parts (webhook ID + token).
const fn combine(hash: u64, part: u64) -> u64 {
    (hash.rotate_left(29) ^ part).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// Writes `text` as one path segment, percent-encoding everything but unreserved characters
/// (RFC 3986), so a parameter can never change the structure of the path.
pub(crate) fn write_text(out: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0xf)]));
        }
    }
}

/// Writes `template` with every `{name}` replaced by the matching parameter.
fn write_template(out: &mut String, template: &str, params: &[(&str, Segment<'_>)]) {
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('}').unwrap_or(after.len());
        let name = &after[..close];
        match params.iter().find(|(param, _)| *param == name) {
            Some((_, Segment::Id(id))) => {
                // Writing to a `String` cannot fail.
                let _ = fmt::Write::write_fmt(out, format_args!("{id}"));
            }
            Some((_, Segment::Text(text))) => write_text(out, text),
            None => debug_assert!(false, "unknown route parameter {{{name}}} in {template}"),
        }
        rest = after.get(close + 1..).unwrap_or("");
    }
    out.push_str(rest);
}

macro_rules! routes {
    ($(
        $(#[doc = $doc:literal])*
        $variant:ident $({ $( $(#[doc = $field_doc:literal])* $field:ident: $ty:ty ),* $(,)? })?
            => $method:ident $template:literal
            $(, major: $($major:ident)+)?
            $(, limits: $limits:ident)?;
    )*) => {
        /// A Discord REST API endpoint together with its path parameters.
        ///
        /// Each route knows its HTTP method, its path and how it is
        /// [rate limited](https://docs.discord.com/developers/topics/rate-limits). Most code uses
        /// the typed methods on [`Rest`](crate::Rest) instead of building routes directly;
        /// routes are useful with the low-level [`Rest::request`](crate::Rest::request).
        ///
        /// Tokens (interaction and webhook tokens) are borrowed (D-090), percent-encoded in the
        /// path and redacted in the `Debug` output.
        ///
        /// discord.js counterpart: the `Routes` helper of discord-api-types.
        ///
        /// # Examples
        ///
        /// ```
        /// use djs_model::id::ChannelId;
        /// use djs_rest::Route;
        ///
        /// let route = Route::CreateMessage { channel_id: ChannelId::new(42) };
        /// assert_eq!(route.method(), http::Method::POST);
        /// assert_eq!(route.path(), "/channels/42/messages");
        /// ```
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Route<'a> {
            $(
                $(#[doc = $doc])*
                #[doc = ""]
                #[doc = concat!("`", stringify!($method), " ", $template, "`")]
                $variant $({ $( $(#[doc = $field_doc])* $field: $ty ),* })?,
            )*
        }

        impl Route<'_> {
            /// Returns the HTTP method of the route.
            #[must_use]
            pub fn method(self) -> Method {
                match self {
                    $(Self::$variant { .. } => Method::$method,)*
                }
            }

            /// Returns the path of the route, relative to the versioned API base
            /// (e.g. `/channels/42/messages`).
            #[must_use]
            pub fn path(self) -> String {
                let mut path = String::with_capacity(64);
                self.write_path(&mut path);
                path
            }

            /// Appends the path of the route to `out`.
            pub(crate) fn write_path(self, out: &mut String) {
                match self {
                    $(Self::$variant $({ $($field),* })? => write_template(
                        out,
                        $template,
                        &[$($( (stringify!($field), Param::to_segment($field)) ),*)?],
                    ),)*
                }
            }

            /// Returns the rate-limit grouping of the route, before route-specific exceptions.
            fn base_bucket(self) -> BucketRoute {
                match self {
                    $(
                        #[allow(unused_variables, reason = "not every route has a major parameter")]
                        Self::$variant $({ $($field),* })? => BucketRoute {
                            template: concat!(stringify!($method), " ", $template),
                            major: 0 $($( ^ combine(0, Param::major($major)) )+)?,
                            limits: routes!(@limits $($limits)?),
                        },
                    )*
                }
            }
        }

        impl fmt::Debug for Route<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match *self {
                    $(
                        #[allow(unused_mut, reason = "unit routes have no fields")]
                        Self::$variant $({ $($field),* })? => {
                            let mut out = f.debug_struct(stringify!($variant));
                            $($( Param::debug($field, stringify!($field), &mut out); )*)?
                            out.finish()
                        }
                    )*
                }
            }
        }
    };
    (@limits) => { Limits::NORMAL };
    (@limits $limits:ident) => { Limits::$limits };
}

impl Route<'_> {
    /// Returns the rate-limit grouping of the route.
    pub(crate) fn bucket(self) -> BucketRoute {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            });
        self.bucket_at(now_ms)
    }

    /// [`bucket`](Self::bucket) at the Unix time `now_ms` (milliseconds).
    fn bucket_at(self, now_ms: u64) -> BucketRoute {
        let mut bucket = self.base_bucket();
        // Deleting a message older than two weeks is limited separately from newer ones
        // (https://github.com/discord/discord-api-docs/issues/1295; discord.js does the same).
        if let Self::DeleteMessage { message_id, .. } = self
            && now_ms.saturating_sub(message_id.timestamp_ms()) > OLD_MESSAGE_AGE_MS
        {
            bucket.template = OLD_MESSAGE_DELETE;
        }
        bucket
    }
}

routes! {
    /// [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot).
    GetGatewayBot => GET "/gateway/bot";

    /// [Create Message](https://docs.discord.com/developers/resources/message#create-message).
    CreateMessage {
        /// The channel to send the message in.
        channel_id: ChannelId,
    } => POST "/channels/{channel_id}/messages", major: channel_id;

    /// [Delete Message](https://docs.discord.com/developers/resources/message#delete-message).
    /// Messages older than two weeks are deleted under a separate rate limit.
    DeleteMessage {
        /// The channel of the message.
        channel_id: ChannelId,
        /// The message to delete.
        message_id: MessageId,
    } => DELETE "/channels/{channel_id}/messages/{message_id}", major: channel_id;

    /// [Create Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#create-interaction-response):
    /// the initial response to an interaction (within 3 seconds). Not bound to the global rate
    /// limit, and never queued behind other callbacks.
    CreateInteractionResponse {
        /// The interaction.
        interaction_id: InteractionId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => POST "/interactions/{interaction_id}/{interaction_token}/callback",
        limits: INTERACTION_CALLBACK;

    /// [Get Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#get-original-interaction-response).
    GetOriginalInteractionResponse {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => GET "/webhooks/{application_id}/{interaction_token}/messages/@original",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Edit Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#edit-original-interaction-response).
    EditOriginalInteractionResponse {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => PATCH "/webhooks/{application_id}/{interaction_token}/messages/@original",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Delete Original Interaction Response](https://docs.discord.com/developers/interactions/receiving-and-responding#delete-original-interaction-response).
    DeleteOriginalInteractionResponse {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => DELETE "/webhooks/{application_id}/{interaction_token}/messages/@original",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Create Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#create-followup-message).
    CreateFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
    } => POST "/webhooks/{application_id}/{interaction_token}",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Get Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#get-followup-message).
    GetFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
        /// The follow-up message.
        message_id: MessageId,
    } => GET "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Edit Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#edit-followup-message).
    EditFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
        /// The follow-up message.
        message_id: MessageId,
    } => PATCH "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        major: application_id interaction_token, limits: INTERACTION;

    /// [Delete Followup Message](https://docs.discord.com/developers/interactions/receiving-and-responding#delete-followup-message).
    DeleteFollowupMessage {
        /// The application.
        application_id: ApplicationId,
        /// The interaction's token.
        interaction_token: &'a str,
        /// The follow-up message.
        message_id: MessageId,
    } => DELETE "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        major: application_id interaction_token, limits: INTERACTION;
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "aW50ZXJhY3Rpb24";

    #[test]
    fn paths() {
        assert_eq!(Route::GetGatewayBot.path(), "/gateway/bot");
        assert_eq!(
            Route::CreateMessage {
                channel_id: ChannelId::new(1)
            }
            .path(),
            "/channels/1/messages"
        );
        assert_eq!(
            Route::CreateInteractionResponse {
                interaction_id: InteractionId::new(2),
                interaction_token: TOKEN,
            }
            .path(),
            "/interactions/2/aW50ZXJhY3Rpb24/callback"
        );
        assert_eq!(
            Route::EditFollowupMessage {
                application_id: ApplicationId::new(3),
                interaction_token: TOKEN,
                message_id: MessageId::new(4),
            }
            .path(),
            "/webhooks/3/aW50ZXJhY3Rpb24/messages/4"
        );
    }

    #[test]
    fn text_parameters_cannot_change_the_path() {
        let route = Route::CreateFollowupMessage {
            application_id: ApplicationId::new(3),
            interaction_token: "a/../b?c=d#e f%",
        };
        assert_eq!(route.path(), "/webhooks/3/a%2F..%2Fb%3Fc%3Dd%23e%20f%25");
    }

    #[test]
    fn debug_redacts_tokens() {
        let route = Route::GetFollowupMessage {
            application_id: ApplicationId::new(3),
            interaction_token: TOKEN,
            message_id: MessageId::new(4),
        };
        let debug = format!("{route:?}");
        assert_eq!(
            debug,
            r#"GetFollowupMessage { application_id: 3, interaction_token: "<redacted>", message_id: 4 }"#
        );
        assert_eq!(format!("{:?}", Route::GetGatewayBot), "GetGatewayBot");
    }

    #[test]
    fn major_parameter() {
        let a = Route::CreateMessage {
            channel_id: ChannelId::new(1),
        }
        .bucket();
        let b = Route::CreateMessage {
            channel_id: ChannelId::new(2),
        }
        .bucket();
        assert_eq!(a.template, b.template);
        assert_ne!(a.major, b.major);
        assert_eq!(a.template, "POST /channels/{channel_id}/messages");
        assert_eq!(Route::GetGatewayBot.bucket().major, 0);

        // Webhook routes are limited per webhook ID + token.
        let followup = |token| {
            Route::CreateFollowupMessage {
                application_id: ApplicationId::new(3),
                interaction_token: token,
            }
            .bucket()
        };
        assert_ne!(followup("a").major, followup("b").major);
        assert_eq!(followup("a").major, followup("a").major);
    }

    #[test]
    fn old_message_deletes_have_their_own_bucket() {
        let created_ms = 1_794_398_400_000;
        let route = Route::DeleteMessage {
            channel_id: ChannelId::new(1),
            message_id: MessageId::from_timestamp_ms(created_ms).unwrap(),
        };
        assert_eq!(
            route.path(),
            format!(
                "/channels/1/messages/{}",
                MessageId::from_timestamp_ms(created_ms).unwrap()
            )
        );
        let new = route.bucket_at(created_ms + OLD_MESSAGE_AGE_MS);
        assert_eq!(
            new.template,
            "DELETE /channels/{channel_id}/messages/{message_id}"
        );
        let old = route.bucket_at(created_ms + OLD_MESSAGE_AGE_MS + 1);
        assert_eq!(old.template, OLD_MESSAGE_DELETE);
        // Same channel: the major parameter does not change.
        assert_eq!((old.major, old.limits), (new.major, new.limits));
        // A clock before the message (or before 1970) never makes it old.
        assert_eq!(route.bucket_at(0).template, new.template);
        // With the real clock, a message from 2015 is old.
        let ancient = Route::DeleteMessage {
            channel_id: ChannelId::new(1),
            message_id: MessageId::new(1 << 22),
        };
        assert_eq!(ancient.bucket().template, OLD_MESSAGE_DELETE);
    }

    #[test]
    fn interaction_routes_are_not_global() {
        let callback = Route::CreateInteractionResponse {
            interaction_id: InteractionId::new(2),
            interaction_token: TOKEN,
        }
        .bucket();
        assert_eq!(callback.limits, Limits::INTERACTION_CALLBACK);
        assert!(!callback.limits.global && callback.limits.burst);
        let original = Route::GetOriginalInteractionResponse {
            application_id: ApplicationId::new(3),
            interaction_token: TOKEN,
        }
        .bucket();
        assert!(!original.limits.global && !original.limits.burst);
        assert!(Route::GetGatewayBot.bucket().limits.global);
    }
}
