//! The SKU, entitlement and subscription endpoints against the mock API: method, path, query
//! and body of every request.

use djs_model::{
    id::{ApplicationId, EntitlementId, GuildId, SkuId, SubscriptionId, UserId},
    rest::{CreateTestEntitlement, ListEntitlements, ListSkuSubscriptions},
    sku::SkuType,
};

mod common;

use common::{Canned, mock, seen};

const SKU: &str = include_str!("../../djs-model/tests/fixtures/docs/sku_sku.json");
const ENTITLEMENT: &str =
    include_str!("../../djs-model/tests/fixtures/docs/entitlement_entitlement.json");
const SUBSCRIPTION: &str =
    include_str!("../../djs-model/tests/fixtures/docs/subscription_subscription.json");
/// What Create Test Entitlement returns: no `starts_at`, `ends_at` or `subscription_id`.
const TEST_ENTITLEMENT: &str = r#"{"id": "1019653849998299137", "sku_id": "10",
    "application_id": "1019370614521200640", "guild_id": "20", "type": 4, "deleted": false,
    "consumed": false}"#;

const APP: ApplicationId = ApplicationId::new(1_019_370_614_521_200_640);
const ENTITLEMENT_ID: EntitlementId = EntitlementId::new(1_019_653_849_998_299_136);

/// `(method, path after /api/v10, query, body)`.
type Expected = (&'static str, String, &'static str, &'static str);

#[tokio::test]
async fn monetization_endpoints() {
    let no_content = || Canned::json(204, "");
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{SKU}]")),
        Canned::json(200, &format!("[{ENTITLEMENT}]")),
        Canned::json(200, ENTITLEMENT),
        no_content(),
        Canned::json(200, TEST_ENTITLEMENT),
        no_content(),
        Canned::json(200, &format!("[{SUBSCRIPTION}]")),
        Canned::json(200, SUBSCRIPTION),
    ])
    .await;

    let skus = rest.list_skus(APP).await.unwrap();
    assert_eq!(skus[0].kind, SkuType::Subscription);

    let mut query = ListEntitlements::default();
    query.user_id = Some(UserId::new(7));
    query.sku_ids = vec![SkuId::new(1), SkuId::new(2)];
    query.limit = Some(10);
    query.exclude_ended = Some(true);
    let entitlements = rest.list_entitlements(APP, &query).await.unwrap();
    assert_eq!(entitlements[0].id, ENTITLEMENT_ID);
    rest.get_entitlement(APP, ENTITLEMENT_ID).await.unwrap();
    rest.consume_entitlement(APP, ENTITLEMENT_ID).await.unwrap();

    let body = CreateTestEntitlement::guild(SkuId::new(10), GuildId::new(20));
    let test = rest.create_test_entitlement(APP, &body).await.unwrap();
    assert_eq!(test.starts_at, None);
    assert_eq!(test.guild_id, Some(GuildId::new(20)));
    rest.delete_test_entitlement(APP, test.id).await.unwrap();

    let sku = SkuId::new(1_158_857_122_189_168_803);
    let mut subscriptions = ListSkuSubscriptions::default();
    subscriptions.user_id = Some(UserId::new(7));
    subscriptions.after = Some(SubscriptionId::new(5));
    let listed = rest
        .list_sku_subscriptions(sku, &subscriptions)
        .await
        .unwrap();
    let subscription = rest.get_sku_subscription(sku, listed[0].id).await.unwrap();
    assert_eq!(subscription.sku_ids, [sku]);

    let app = format!("/applications/{APP}");
    let one = format!("{app}/entitlements/{ENTITLEMENT_ID}");
    let expected: Vec<Expected> = vec![
        ("GET", format!("{app}/skus"), "", ""),
        (
            "GET",
            format!("{app}/entitlements"),
            "user_id=7&sku_ids=1,2&limit=10&exclude_ended=true",
            "",
        ),
        ("GET", one.clone(), "", ""),
        ("POST", format!("{one}/consume"), "", ""),
        (
            "POST",
            format!("{app}/entitlements"),
            "",
            r#"{"sku_id":"10","owner_id":"20","owner_type":1}"#,
        ),
        (
            "DELETE",
            format!("{app}/entitlements/1019653849998299137"),
            "",
            "",
        ),
        (
            "GET",
            format!("/skus/{sku}/subscriptions"),
            "after=5&user_id=7",
            "",
        ),
        (
            "GET",
            format!("/skus/{sku}/subscriptions/1278078770116427839"),
            "",
            "",
        ),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason, None, "{what}");
    }
}
