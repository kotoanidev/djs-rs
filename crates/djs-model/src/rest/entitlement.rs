//! Bodies and queries of the [entitlement endpoints](https://docs.discord.com/developers/resources/entitlement).

use serde::Serialize;

use crate::{
    entitlement::EntitlementOwnerType,
    id::{EntitlementId, GenericId, GuildId, SkuId, UserId},
};

/// The query of [List Entitlements](https://docs.discord.com/developers/resources/entitlement#list-entitlements).
///
/// discord.js counterpart: the options of `EntitlementManager#fetch`.
///
/// # Examples
///
/// ```
/// use djs_model::{id::UserId, rest::ListEntitlements};
///
/// // The active entitlements of one user.
/// let mut query = ListEntitlements::default();
/// query.user_id = Some(UserId::new(7));
/// query.exclude_ended = Some(true);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ListEntitlements {
    /// Only the entitlements of this user.
    pub user_id: Option<UserId>,
    /// Only the entitlements to these SKUs (sent comma-separated; none means all).
    pub sku_ids: Vec<SkuId>,
    /// Entitlements before this one.
    pub before: Option<EntitlementId>,
    /// Entitlements after this one.
    pub after: Option<EntitlementId>,
    /// How many entitlements (1-100; Discord's default is 100).
    pub limit: Option<u8>,
    /// Only the entitlements of this guild.
    pub guild_id: Option<GuildId>,
    /// Whether to leave out entitlements that have ended (Discord's default: `false`).
    pub exclude_ended: Option<bool>,
    /// Whether to leave out deleted entitlements (Discord's default: `true`).
    pub exclude_deleted: Option<bool>,
}

/// The body of [Create Test Entitlement](https://docs.discord.com/developers/resources/entitlement#create-test-entitlement).
///
/// Built with [`guild`](Self::guild) or [`user`](Self::user), which set the owner and its
/// type (D-130).
///
/// discord.js counterpart: the argument of `EntitlementManager#createTest`.
///
/// # Examples
///
/// ```
/// use djs_model::{
///     id::{GuildId, SkuId},
///     rest::CreateTestEntitlement,
/// };
///
/// let body = CreateTestEntitlement::guild(SkuId::new(10), GuildId::new(20));
/// assert_eq!(
///     serde_json::to_string(&body)?,
///     r#"{"sku_id":"10","owner_id":"20","owner_type":1}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub struct CreateTestEntitlement {
    /// The SKU to grant access to.
    pub sku_id: SkuId,
    /// The guild or user that gets access.
    pub owner_id: GenericId,
    /// Whether [`owner_id`](Self::owner_id) is a guild or a user.
    pub owner_type: EntitlementOwnerType,
}

impl CreateTestEntitlement {
    /// A test entitlement granting `guild_id` access to a guild subscription SKU.
    #[must_use]
    pub const fn guild(sku_id: SkuId, guild_id: GuildId) -> Self {
        Self {
            sku_id,
            owner_id: guild_id.cast(),
            owner_type: EntitlementOwnerType::Guild,
        }
    }

    /// A test entitlement granting `user_id` access to a user subscription SKU.
    #[must_use]
    pub const fn user(sku_id: SkuId, user_id: UserId) -> Self {
        Self {
            sku_id,
            owner_id: user_id.cast(),
            owner_type: EntitlementOwnerType::User,
        }
    }
}
