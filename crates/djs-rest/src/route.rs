//! Typed REST API routes.

use core::fmt::Write as _;

use djs_model::id::ChannelId;
use http::Method;

/// A Discord REST API endpoint together with its path parameters.
///
/// Each route knows its HTTP method, its path and how it is
/// [rate limited](https://docs.discord.com/developers/topics/rate-limits). Most code uses the
/// typed methods on [`Rest`](crate::Rest) instead of building routes directly; routes are useful
/// with the low-level [`Rest::request`](crate::Rest::request).
///
/// **Coverage:** Phase 1 contains the routes of the vertical slice; every other v10 endpoint is
/// added in Phase 3.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Route {
    /// [Get Gateway Bot](https://docs.discord.com/developers/events/gateway#get-gateway-bot):
    /// `GET /gateway/bot`.
    GetGatewayBot,
    /// [Create Message](https://docs.discord.com/developers/resources/message#create-message):
    /// `POST /channels/{channel_id}/messages`.
    CreateMessage {
        /// The channel to send the message in.
        channel_id: ChannelId,
    },
}

/// How a route is grouped for rate limiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BucketRoute {
    /// A unique, static description of the route *without* its parameters, e.g.
    /// `"POST /channels/:id/messages"`. Routes sharing a template share a bucket hash.
    pub(crate) template: &'static str,
    /// The [major parameter](https://docs.discord.com/developers/topics/rate-limits) (channel,
    /// guild or webhook ID), or `0` if the route has none. Rate limits are tracked separately
    /// per major parameter.
    pub(crate) major: u64,
}

impl Route {
    /// Returns the HTTP method of the route.
    #[must_use]
    pub fn method(self) -> Method {
        match self {
            Self::GetGatewayBot => Method::GET,
            Self::CreateMessage { .. } => Method::POST,
        }
    }

    /// Returns the path of the route, relative to the versioned API base
    /// (e.g. `/channels/42/messages`).
    #[must_use]
    pub fn path(self) -> String {
        let mut path = String::with_capacity(48);
        self.write_path(&mut path);
        path
    }

    /// Appends the path of the route to `out`.
    pub(crate) fn write_path(self, out: &mut String) {
        // Writing to a `String` cannot fail.
        let _ = match self {
            Self::GetGatewayBot => out.write_str("/gateway/bot"),
            Self::CreateMessage { channel_id } => write!(out, "/channels/{channel_id}/messages"),
        };
    }

    /// Returns the rate-limit grouping of the route.
    pub(crate) const fn bucket(self) -> BucketRoute {
        match self {
            Self::GetGatewayBot => BucketRoute {
                template: "GET /gateway/bot",
                major: 0,
            },
            Self::CreateMessage { channel_id } => BucketRoute {
                template: "POST /channels/:id/messages",
                major: channel_id.get(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
