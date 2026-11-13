//! [Subscription endpoints](https://docs.discord.com/developers/resources/subscription): the
//! recurring payments of users for an app's subscription SKUs.

use djs_model::{
    id::{SkuId, SubscriptionId},
    rest::ListSkuSubscriptions,
    subscription::Subscription,
};

use crate::client::boxed;
use crate::{Error, Request, Rest, Route};

impl Rest {
    /// [List SKU Subscriptions](https://docs.discord.com/developers/resources/subscription#list-sku-subscriptions):
    /// returns a page of the subscriptions that include the SKU (50 by default). With a bot
    /// token, [`user_id`](ListSkuSubscriptions::user_id) is required.
    ///
    /// discord.js counterpart: `SubscriptionManager#fetch`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example(rest: djs_rest::Rest, sku_id: djs_model::id::SkuId, user_id: djs_model::id::UserId) -> Result<(), djs_rest::Error> {
    /// use djs_model::rest::ListSkuSubscriptions;
    ///
    /// let mut query = ListSkuSubscriptions::default();
    /// query.user_id = Some(user_id);
    /// for subscription in rest.list_sku_subscriptions(sku_id, &query).await? {
    ///     println!("{:?} until {}", subscription.status, subscription.current_period_end);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10027` Unknown SKU.
    pub fn list_sku_subscriptions<'a>(
        &'a self,
        sku_id: SkuId,
        query: &'a ListSkuSubscriptions,
    ) -> impl Future<Output = Result<Vec<Subscription>, Error>> + Send + 'a {
        boxed(async move {
            self.call(Request::new(Route::ListSkuSubscriptions { sku_id }).query(query))
                .await
        })
    }

    /// [Get SKU Subscription](https://docs.discord.com/developers/resources/subscription#get-sku-subscription):
    /// returns one subscription.
    ///
    /// discord.js counterpart: `SubscriptionManager#fetch({ subscriptionId })`.
    ///
    /// # Errors
    ///
    /// See [`Rest::request`]; `10027` Unknown SKU.
    pub fn get_sku_subscription(
        &self,
        sku_id: SkuId,
        subscription_id: SubscriptionId,
    ) -> impl Future<Output = Result<Subscription, Error>> + Send + '_ {
        boxed(async move {
            self.call(Request::new(Route::GetSkuSubscription {
                sku_id,
                subscription_id,
            }))
            .await
        })
    }
}
