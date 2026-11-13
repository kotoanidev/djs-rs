//! [Entitlement endpoints](https://docs.discord.com/developers/resources/entitlement): which
//! users and guilds have access to an app's SKUs, consuming one-time purchases and test
//! entitlements.

use djs_model::{
    entitlement::Entitlement,
    id::{ApplicationId, EntitlementId},
    rest::{CreateTestEntitlement, ListEntitlements},
};

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [List Entitlements](https://docs.discord.com/developers/resources/entitlement#list-entitlements):
    /// returns a page of the application's entitlements, active and ended (100 by default).
    ///
    /// discord.js counterpart: `EntitlementManager#fetch`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId, user_id: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::{rest::ListEntitlements, util::Timestamp};
    ///
    /// let mut query = ListEntitlements::default();
    /// query.user_id = Some(user_id);
    /// query.exclude_ended = Some(true);
    /// let entitlements = rest.list_entitlements(application_id, &query).await?;
    /// let premium = entitlements
    ///     .iter()
    ///     .any(|entitlement| entitlement.is_active(Timestamp::now()));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`].
    pub fn list_entitlements<'a>(
        &'a self,
        application_id: ApplicationId,
        query: &'a ListEntitlements,
    ) -> impl Future<Output = Result<Vec<Entitlement>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListEntitlements { application_id }).query(query))
                .await
        })
    }

    /// [Get Entitlement](https://docs.discord.com/developers/resources/entitlement#get-entitlement):
    /// returns one entitlement.
    ///
    /// discord.js counterpart: none.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10029` Unknown Entitlement.
    pub fn get_entitlement(
        &self,
        application_id: ApplicationId,
        entitlement_id: EntitlementId,
    ) -> impl Future<Output = Result<Entitlement, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetEntitlement {
                application_id,
                entitlement_id,
            }))
            .await
        })
    }

    /// [Consume an Entitlement](https://docs.discord.com/developers/resources/entitlement#consume-an-entitlement):
    /// marks the entitlement to a consumable one-time purchase as used; it then has
    /// [`consumed`](Entitlement::consumed) set.
    ///
    /// discord.js counterpart: `Entitlement#consume`, `EntitlementManager#consume`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10029` Unknown Entitlement; `40018` for an entitlement to an
    /// SKU that is not consumable.
    pub fn consume_entitlement(
        &self,
        application_id: ApplicationId,
        entitlement_id: EntitlementId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::ConsumeEntitlement {
                application_id,
                entitlement_id,
            }))
            .await
        })
    }

    /// [Create Test Entitlement](https://docs.discord.com/developers/resources/entitlement#create-test-entitlement):
    /// grants a guild or user access to an SKU for testing, without an end. Discord returns
    /// it without [`starts_at`](Entitlement::starts_at) and
    /// [`ends_at`](Entitlement::ends_at).
    ///
    /// discord.js counterpart: `EntitlementManager#createTest`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, application_id: djs_model::id::ApplicationId, sku_id: djs_model::id::SkuId, guild_id: djs_model::id::GuildId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::CreateTestEntitlement;
    ///
    /// let body = CreateTestEntitlement::guild(sku_id, guild_id);
    /// let entitlement = rest.create_test_entitlement(application_id, &body).await?;
    /// println!("Test entitlement {}", entitlement.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `40074` when the owner already has the entitlement.
    pub fn create_test_entitlement<'a>(
        &'a self,
        application_id: ApplicationId,
        body: &'a CreateTestEntitlement,
    ) -> impl Future<Output = Result<Entitlement, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::CreateTestEntitlement { application_id }).json(body)?)
                .await
        })
    }

    /// [Delete Test Entitlement](https://docs.discord.com/developers/resources/entitlement#delete-test-entitlement):
    /// removes a test entitlement.
    ///
    /// discord.js counterpart: `EntitlementManager#deleteTest`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10029` Unknown Entitlement; `40019` for an entitlement that is
    /// not a test entitlement.
    pub fn delete_test_entitlement(
        &self,
        application_id: ApplicationId,
        entitlement_id: EntitlementId,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::DeleteTestEntitlement {
                application_id,
                entitlement_id,
            }))
            .await
        })
    }
}
