//! [SKUs](https://docs.discord.com/developers/resources/sku): the premium offerings (one-time
//! purchases and subscriptions) an app sells.

use crate::macros::int_flags;

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
