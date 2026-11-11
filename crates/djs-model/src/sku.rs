//! [SKUs](https://docs.discord.com/developers/resources/sku): the premium offerings (one-time
//! purchases and subscriptions) an app sells.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{ApplicationId, SkuId},
    macros::{int_enum, int_flags},
};

/// A [SKU](https://docs.discord.com/developers/resources/sku#sku-object): a premium offering
/// (one-time purchase or subscription) of an app.
///
/// To tell user from guild subscriptions, check [`flags`](Self::flags).
///
/// discord.js counterpart: `SKU`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sku {
    /// The SKU's ID.
    pub id: SkuId,
    /// The kind of offering.
    #[serde(rename = "type")]
    pub kind: SkuType,
    /// The application that sells it.
    pub application_id: ApplicationId,
    /// The name shown to customers.
    pub name: CompactString,
    /// A URL slug generated from the name.
    pub slug: CompactString,
    /// The SKU's flags.
    pub flags: SkuFlags,
}

int_enum! {
    /// The [kind of a SKU](https://docs.discord.com/developers/resources/sku#sku-object-sku-types).
    ///
    /// discord.js counterpart: `SKUType`.
    pub enum SkuType: u8 {
        /// A one-time purchase that is kept.
        Durable = 2,
        /// A one-time purchase that is used up.
        Consumable = 3,
        /// A recurring subscription.
        Subscription = 5,
        /// The group Discord creates for each [`Subscription`](Self::Subscription) SKU.
        SubscriptionGroup = 6,
    }
}

int_flags! {
    /// [Flags of a SKU](https://docs.discord.com/developers/resources/sku#sku-object-sku-flags).
    ///
    /// discord.js counterpart: `SKUFlags` / `SKUFlagsBitField`.
    pub struct SkuFlags: u64 {
        /// The SKU can be purchased.
        const AVAILABLE = 1 << 2;
        /// A recurring subscription bought by a user for one guild; everyone in that guild gets
        /// the benefits.
        const GUILD_SUBSCRIPTION = 1 << 7;
        /// A recurring subscription bought by a user for themselves, valid in every guild.
        const USER_SUBSCRIPTION = 1 << 8;
    }
}
