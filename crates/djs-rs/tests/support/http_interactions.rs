//! Signed requests as Discord sends them to an interactions endpoint, shared by the HTTP
//! interactions benchmark and stress test (D-041).

#![allow(dead_code, reason = "each test crate uses some of the helpers")]

use std::fmt::Write as _;

use ed25519_dalek::{Signer, SigningKey};

/// A `PING`, the request Discord sends when the endpoint URL is saved.
pub(crate) const PING: &[u8] =
    br#"{"id":"1","application_id":"1","type":1,"token":"t","version":1}"#;

/// The application's key pair; `public_key` is what `Client::interactions_endpoint` takes.
pub(crate) fn key() -> SigningKey {
    SigningKey::from_bytes(&[42; 32])
}

/// The public key in hexadecimal, as the Developer Portal shows it.
pub(crate) fn public_key() -> String {
    hex(&key().verifying_key().to_bytes())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// The current Unix time in seconds, as the timestamp header carries it.
pub(crate) fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

/// The `X-Signature-Ed25519` header for `body` sent at `timestamp`.
pub(crate) fn sign(timestamp: &str, body: &[u8]) -> String {
    let mut message = timestamp.as_bytes().to_vec();
    message.extend_from_slice(body);
    hex(&key().sign(&message).to_bytes())
}

/// A realistic `/mod config set` interaction (≈ 2.4 KB): the guild and channel objects, the
/// invoking member, five options and their resolved user, member and role.
pub(crate) fn command(id: u64) -> Vec<u8> {
    format!(
        r#"{{"id":"{}","application_id":"1","type":2,"token":"not-a-real-token-only-for-benchmarking-0123456789-aWQ6MTMwMDAwMDAwMDAwMDAwMDAwMQ","version":1,"app_permissions":"2248473465835073","locale":"pl","guild_locale":"en-US","guild_id":"1400000000000000002","guild":{{"id":"1400000000000000002","locale":"en-US","features":["COMMUNITY","NEWS"]}},"channel_id":"1200000000000000020","channel":{{"id":"1200000000000000020","type":0,"name":"general","guild_id":"1400000000000000002","nsfw":false,"flags":0,"permissions":"2248473465835073","position":3,"topic":"Talk here","last_message_id":"1500000000000000099","rate_limit_per_user":0,"parent_id":"1200000000000000001"}},"member":{{"user":{{"id":"1300000000000000003","username":"nelly","discriminator":"0","global_name":"Nelly","avatar":"a_1269e74af4df7417b13759eae50c83dc","public_flags":0,"clan":null,"avatar_decoration_data":null}},"roles":["1100000000000000001","1100000000000000002"],"joined_at":"2024-05-01T12:00:00.000000+00:00","premium_since":null,"deaf":false,"mute":false,"flags":0,"pending":false,"nick":null,"avatar":null,"banner":null,"communication_disabled_until":null,"permissions":"2248473465835073"}},"entitlements":[],"authorizing_integration_owners":{{"0":"1400000000000000002"}},"context":0,"attachment_size_limit":10485760,"data":{{"id":"1600000000000000001","name":"mod","type":1,"guild_id":"1400000000000000002","options":[{{"name":"config","type":2,"options":[{{"name":"set","type":1,"options":[{{"name":"reason","type":3,"value":"spam in general, number {id}"}},{{"name":"days","type":4,"value":7}},{{"name":"silent","type":5,"value":true}},{{"name":"target","type":6,"value":"1300000000000000011"}},{{"name":"role","type":8,"value":"1100000000000000001"}}]}}]}}],"resolved":{{"users":{{"1300000000000000011":{{"id":"1300000000000000011","username":"spammer","discriminator":"0","global_name":null,"avatar":null,"public_flags":0}}}},"members":{{"1300000000000000011":{{"roles":[],"joined_at":"2026-11-01T12:00:00.000000+00:00","premium_since":null,"flags":0,"pending":false,"nick":null,"avatar":null,"banner":null,"communication_disabled_until":null,"permissions":"104324673"}}}},"roles":{{"1100000000000000001":{{"id":"1100000000000000001","name":"Muted","color":0,"colors":{{"primary_color":0,"secondary_color":null,"tertiary_color":null}},"hoist":false,"icon":null,"unicode_emoji":null,"position":1,"permissions":"0","managed":false,"mentionable":false,"flags":0}}}}}}}}}}"#,
        1_700_000_000_000_000_000_u64 + id
    )
    .into_bytes()
}
