//! [Subscriptions](https://docs.discord.com/developers/resources/subscription): a user's
//! recurring payment for one of an app's subscription [SKUs](crate::sku).

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{EntitlementId, SkuId, SubscriptionId, UserId},
    macros::int_enum,
    util::Timestamp,
};

/// A [subscription](https://docs.discord.com/developers/resources/subscription#subscription-object).
///
/// Use the [`Entitlement`](crate::entitlement::Entitlement)s it grants to decide access;
/// the subscription only describes the billing period.
///
/// discord.js counterpart: `Subscription`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Subscription {
    /// The subscription's ID.
    pub id: SubscriptionId,
    /// The subscribed user.
    pub user_id: UserId,
    /// The SKUs subscribed to.
    pub sku_ids: Vec<SkuId>,
    /// The entitlements granted by the subscription.
    pub entitlement_ids: Vec<EntitlementId>,
    /// The SKUs the user will be subscribed to at renewal (`None` if unchanged).
    #[serde(default)]
    pub renewal_sku_ids: Option<Vec<SkuId>>,
    /// The start of the current period.
    pub current_period_start: Timestamp,
    /// The end of the current period.
    pub current_period_end: Timestamp,
    /// The subscription's status.
    pub status: SubscriptionStatus,
    /// When the subscription was canceled.
    #[serde(default)]
    pub canceled_at: Option<Timestamp>,
    /// The ISO 3166-1 alpha-2 country code of the payment source (only with a private
    /// `OAuth2` scope).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<CompactString>,
}

int_enum! {
    /// The [status of a subscription](https://docs.discord.com/developers/resources/subscription#subscription-statuses).
    ///
    /// discord.js counterpart: `SubscriptionStatus`.
    pub enum SubscriptionStatus: u8 {
        /// Active and scheduled to renew.
        Active = 0,
        /// Inactive and not being charged.
        Inactive = 1,
        /// Active but will not renew.
        Ending = 2,
    }
}
