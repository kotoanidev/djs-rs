//! The [SKU endpoint](https://docs.discord.com/developers/resources/sku): the premium
//! offerings an app sells.

use djs_model::{id::ApplicationId, sku::Sku};

use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [List SKUs](https://docs.discord.com/developers/resources/sku#list-skus): returns the
    /// application's SKUs. A subscription appears as two SKUs; entitlements and tests use
    /// the one of type [`Subscription`](djs_model::sku::SkuType::Subscription).
    ///
    /// discord.js counterpart: `ClientApplication#fetchSKUs`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub async fn list_skus(&self, application_id: ApplicationId) -> Result<Vec<Sku>, Error> {
        self.request(Request::new(Route::ListSkus { application_id }))
            .await
    }
}
