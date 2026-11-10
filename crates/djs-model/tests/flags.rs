//! Checks every bitflags type against values computed independently from the Discord docs and
//! discord-api-types (D-046), and their integer (de)serialization (D-047).

use djs_model::{
    application::ApplicationFlags,
    channel::ChannelFlags,
    gateway::{ActivityFlags, Intents},
    guild::{MemberFlags, RoleFlags, SystemChannelFlags},
    invite::InviteFlags,
    message::{AttachmentFlags, EmbedFlags, EmbedMediaFlags, MessageFlags},
    permissions::Permissions,
    sku::SkuFlags,
    user::UserFlags,
};
use serde::{Serialize, de::DeserializeOwned};

/// Asserts the union of all known flags and that unknown bits survive a JSON round trip.
fn check<T>(all: &T, expected_bits: u64, bits: impl Fn(&T) -> u64)
where
    T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
{
    assert_eq!(bits(all), expected_bits, "{all:?}");
    let json = serde_json::to_string(all).unwrap();
    assert_eq!(json, expected_bits.to_string());

    // Bit 63 is unassigned in every type.
    let unknown = expected_bits | (1 << 63);
    let parsed: T = serde_json::from_str(&unknown.to_string()).unwrap();
    assert_eq!(bits(&parsed), unknown);
    assert_eq!(serde_json::to_string(&parsed).unwrap(), unknown.to_string());
}

#[test]
fn all_known_flags() {
    check(&MessageFlags::all(), 62_975, MessageFlags::bits);
    check(&AttachmentFlags::all(), 255, AttachmentFlags::bits);
    check(&EmbedFlags::all(), 240, EmbedFlags::bits);
    check(&EmbedMediaFlags::all(), 32, EmbedMediaFlags::bits);
    check(&UserFlags::all(), 3_395_291_914_921_983, UserFlags::bits);
    check(&ApplicationFlags::all(), 10_483_806, ApplicationFlags::bits);
    check(&ChannelFlags::all(), 2_261_943, ChannelFlags::bits);
    check(&SystemChannelFlags::all(), 63, SystemChannelFlags::bits);
    check(&MemberFlags::all(), 2_047, MemberFlags::bits);
    check(&RoleFlags::all(), 1, RoleFlags::bits);
    check(&InviteFlags::all(), 1, InviteFlags::bits);
    check(&SkuFlags::all(), 388, SkuFlags::bits);
    check(&ActivityFlags::all(), 511, ActivityFlags::bits);
}

#[test]
fn integer_flags_reject_strings_and_negatives() {
    for json in [r#""4""#, "-1", "1.5", "null"] {
        assert!(
            serde_json::from_str::<MessageFlags>(json).is_err(),
            "{json}"
        );
    }
}

#[test]
fn permissions_and_intents_keep_their_formats() {
    // Permissions are strings (D-040), intents integers.
    assert_eq!(
        serde_json::to_string(&Permissions::ADMINISTRATOR).unwrap(),
        r#""8""#
    );
    assert_eq!(serde_json::to_string(&Intents::GUILDS).unwrap(), "1");
}

#[test]
fn default_is_empty() {
    assert!(MessageFlags::default().is_empty());
    assert!(Permissions::default().is_empty());
}
