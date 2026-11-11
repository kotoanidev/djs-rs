//! Queries of the [subscription endpoints](https://docs.discord.com/developers/resources/subscription).

use crate::id::{SubscriptionId, UserId};

/// The query of [List SKU Subscriptions](https://docs.discord.com/developers/resources/subscription#list-sku-subscriptions).
///
/// [`user_id`](Self::user_id) is required unless the request is made with a user's access
/// token.
///
/// discord.js counterpart: the options of `SubscriptionManager#fetch`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListSkuSubscriptions {
    /// Subscriptions before this one.
    pub before: Option<SubscriptionId>,
    /// Subscriptions after this one.
    pub after: Option<SubscriptionId>,
    /// How many subscriptions (1-100; Discord's default is 50).
    pub limit: Option<u8>,
    /// Only the subscriptions of this user.
    pub user_id: Option<UserId>,
}
