//! [Entitlements](https://docs.discord.com/developers/resources/entitlement): a user's or
//! guild's access to an app's premium offering (an [SKU](crate::sku)).

use serde::{Deserialize, Serialize};

use crate::{
    id::{ApplicationId, EntitlementId, GuildId, SkuId, UserId},
    macros::int_enum,
    util::Timestamp,
};

/// An [entitlement](https://docs.discord.com/developers/resources/entitlement#entitlement-object).
///
/// discord.js counterpart: `Entitlement`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Entitlement {
    /// The entitlement's ID.
    pub id: EntitlementId,
    /// The SKU it grants access to.
    pub sku_id: SkuId,
    /// The application the SKU belongs to.
    pub application_id: ApplicationId,
    /// The user who has access, for user subscriptions and items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<UserId>,
    /// How the entitlement was obtained.
    #[serde(rename = "type")]
    pub kind: EntitlementType,
    /// Whether the entitlement was deleted.
    pub deleted: bool,
    /// When the entitlement becomes valid (`None` for test entitlements).
    pub starts_at: Option<Timestamp>,
    /// When the entitlement stops being valid (`None` for test entitlements).
    pub ends_at: Option<Timestamp>,
    /// The guild that has access, for guild subscriptions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// For consumable items: whether the entitlement was consumed.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub consumed: bool,
}

impl Entitlement {
    /// Whether the entitlement grants access at `now`: not deleted, not consumed, and within
    /// its validity period (open ends count as unbounded).
    #[must_use]
    pub fn is_active(&self, now: Timestamp) -> bool {
        !self.deleted
            && !self.consumed
            && self.starts_at.is_none_or(|starts_at| starts_at <= now)
            && self.ends_at.is_none_or(|ends_at| now < ends_at)
    }
}

int_enum! {
    /// [How an entitlement was obtained](https://docs.discord.com/developers/resources/entitlement#entitlement-object-entitlement-types).
    ///
    /// discord.js counterpart: `EntitlementType`.
    pub enum EntitlementType: u8 {
        /// Purchased by the user.
        Purchase = 1,
        /// From a Discord Nitro subscription.
        PremiumSubscription = 2,
        /// Gifted by the developer.
        DeveloperGift = 3,
        /// Purchased by a developer in the application's test mode.
        TestModePurchase = 4,
        /// Granted when the SKU was free.
        FreePurchase = 5,
        /// Gifted by another user.
        UserGift = 6,
        /// Claimed for free by a Nitro subscriber.
        PremiumPurchase = 7,
        /// Purchased as an app subscription.
        ApplicationSubscription = 8,
    }
}
