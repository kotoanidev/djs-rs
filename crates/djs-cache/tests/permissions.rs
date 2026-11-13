//! Members' permissions computed from the cache (D-292, D-294, D-295): every piece the
//! computation reads, and `None` when one is missing.

#[expect(
    dead_code,
    reason = "shared with the djs-model tests, which use other helpers"
)]
#[path = "../../djs-model/tests/common/payloads.rs"]
mod payloads;

use djs_cache::{Cache, CacheConfig, Limit};
use djs_model::{
    channel::Channel,
    guild::{Guild, Member},
    id::{ChannelId, GuildId, RoleId, UserId},
    permissions::Permissions,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const GUILD: GuildId = GuildId::new(payloads::GUILD);
const OWNER: UserId = UserId::new(1);
const MODERATOR: UserId = UserId::new(2);
const ADMIN: UserId = UserId::new(3);
const PLAIN: UserId = UserId::new(4);
const MUTED_MODERATOR: UserId = UserId::new(5);
const STRANGER: UserId = UserId::new(6);
const MODERATORS: RoleId = RoleId::new(100);
const ADMINS: RoleId = RoleId::new(101);
const UNCACHED_ROLE: RoleId = RoleId::new(102);
const GENERAL: ChannelId = ChannelId::new(30);
const STAFF: ChannelId = ChannelId::new(31);
const THREAD: ChannelId = ChannelId::new(32);

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn bits(permissions: Permissions) -> String {
    permissions.bits().to_string()
}

fn everyone() -> Permissions {
    Permissions::VIEW_CHANNEL
        | Permissions::SEND_MESSAGES
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::EMBED_LINKS
}

fn role(id: u64, permissions: Permissions) -> Value {
    json!({ "id": id.to_string(), "name": format!("role {id}"), "color": 0, "hoist": false,
            "position": 1, "permissions": bits(permissions), "managed": false,
            "mentionable": false, "flags": 0,
            "colors": { "primary_color": 0, "secondary_color": null, "tertiary_color": null } })
}

fn member(id: UserId, roles: &[RoleId], timeout_until: Option<&str>) -> Member {
    let roles: Vec<String> = roles.iter().map(ToString::to_string).collect();
    decode(json!({
        "user": { "id": id.to_string(), "username": format!("user{id}"), "discriminator": "0",
                  "global_name": null, "avatar": null },
        "roles": roles, "joined_at": "2026-01-01T00:00:00.000000+00:00", "deaf": false,
        "mute": false, "flags": 0, "pending": false, "communication_disabled_until": timeout_until
    }))
}

fn overwrite(id: u64, kind: u8, allow: Permissions, deny: Permissions) -> Value {
    json!({ "id": id.to_string(), "type": kind, "allow": bits(allow), "deny": bits(deny) })
}

/// A guild owned by [`OWNER`] with `@everyone`, the moderators and the admins; members of
/// each kind; `#general` (everyone may talk), `#staff` (moderators only, `PLAIN` banned by
/// name) and a thread in `#staff`.
fn cache() -> Cache {
    let cache = Cache::with_config(
        CacheConfig::builder()
            .members(Limit::Unlimited)
            .threads(Limit::Unlimited)
            .build(),
    );
    let create: Value = serde_json::from_str(&payloads::guild_create(1, 0, 0, 0, 0)).unwrap();
    let mut guild = create["d"].clone();
    guild["owner_id"] = json!(OWNER.to_string());
    guild["roles"] = json!([
        role(GUILD.get(), everyone()),
        role(
            MODERATORS.get(),
            Permissions::KICK_MEMBERS | Permissions::MANAGE_MESSAGES
        ),
        role(ADMINS.get(), Permissions::ADMINISTRATOR),
    ]);
    cache.store_guild(&decode::<Guild>(guild));

    for (user, roles, timeout) in [
        (OWNER, &[][..], None),
        (MODERATOR, &[MODERATORS][..], None),
        (
            ADMIN,
            &[ADMINS][..],
            Some("2999-01-01T00:00:00.000000+00:00"),
        ),
        (PLAIN, &[][..], Some("2000-01-01T00:00:00.000000+00:00")),
        (
            MUTED_MODERATOR,
            &[MODERATORS][..],
            Some("2999-01-01T00:00:00.000000+00:00"),
        ),
        (STRANGER, &[UNCACHED_ROLE][..], None),
    ] {
        cache.store_member(GUILD, &member(user, roles, timeout));
    }

    let channel = |id: ChannelId, overwrites: Value| -> Channel {
        decode(
            json!({ "id": id.to_string(), "type": 0, "guild_id": GUILD.to_string(),
                       "name": format!("channel {id}"), "position": 0,
                       "permission_overwrites": overwrites, "nsfw": false }),
        )
    };
    cache.store_channel(&channel(GENERAL, json!([])));
    cache.store_channel(&channel(
        STAFF,
        json!([
            overwrite(
                GUILD.get(),
                0,
                Permissions::empty(),
                Permissions::VIEW_CHANNEL
            ),
            overwrite(
                MODERATORS.get(),
                0,
                Permissions::VIEW_CHANNEL,
                Permissions::empty()
            ),
            overwrite(
                PLAIN.get(),
                1,
                Permissions::VIEW_CHANNEL,
                Permissions::SEND_MESSAGES
            ),
        ]),
    ));
    cache.store_channel(&thread(THREAD, STAFF));
    cache
}

fn thread(id: ChannelId, parent: ChannelId) -> Channel {
    decode(json!({
        "id": id.to_string(), "type": 11, "guild_id": GUILD.to_string(),
        "parent_id": parent.to_string(), "name": "a thread", "owner_id": "2",
        "message_count": 0, "member_count": 1,
        "thread_metadata": { "archived": false, "auto_archive_duration": 1440,
                             "archive_timestamp": "2026-11-12T10:00:00.000000+00:00",
                             "locked": false }
    }))
}

#[test]
fn guild_permissions() {
    let cache = cache();
    let of = |user| cache.member_permissions(GUILD, user);
    assert_eq!(of(OWNER), Some(Permissions::all()));
    assert_eq!(
        of(MODERATOR),
        Some(everyone() | Permissions::KICK_MEMBERS | Permissions::MANAGE_MESSAGES)
    );
    // Administrators are exempt from their timeout (D-294).
    assert_eq!(of(ADMIN), Some(Permissions::all()));
    // A timeout that has ended changes nothing.
    assert_eq!(of(PLAIN), Some(everyone()));
    // A timed-out moderator may only view and read (D-294).
    assert_eq!(of(MUTED_MODERATOR), Some(Permissions::DURING_TIMEOUT));
}

#[test]
fn channel_permissions() {
    let cache = cache();
    let general = cache.channels().get(GENERAL).unwrap();
    let staff = cache.channels().get(STAFF).unwrap();
    let thread = cache.threads(GUILD).get(THREAD).unwrap();
    let in_channel = |channel: &Channel, user| cache.member_permissions_in(channel, user);

    assert_eq!(in_channel(&general, PLAIN), Some(everyone()));
    // Hidden from `@everyone`: nothing at all is left (D-295), guild-wide permissions included.
    let hidden = cache.store_member(GUILD, &member(UserId::new(7), &[], None));
    assert_eq!(
        in_channel(&staff, hidden.user.id),
        Some(Permissions::empty())
    );
    // Shown to the moderators.
    let moderator = in_channel(&staff, MODERATOR).unwrap();
    assert!(moderator.contains(Permissions::SEND_MESSAGES | Permissions::KICK_MEMBERS));
    // `PLAIN` may look but not talk, so embeds go too (D-295).
    assert_eq!(
        in_channel(&staff, PLAIN),
        Some(Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY)
    );
    // A thread takes its parent's overwrites.
    assert_eq!(in_channel(&thread, PLAIN), in_channel(&staff, PLAIN));
    assert_eq!(in_channel(&thread, MODERATOR), Some(moderator));
    // Timeouts apply after the overwrites: an overwrite cannot give a timed-out member more.
    assert_eq!(
        in_channel(&staff, MUTED_MODERATOR),
        Some(Permissions::DURING_TIMEOUT)
    );
    assert_eq!(in_channel(&staff, OWNER), Some(Permissions::all()));
}

#[test]
fn missing_pieces_give_none() {
    let cache = cache();
    let general = cache.channels().get(GENERAL).unwrap();
    // One of the member's roles is not cached.
    assert_eq!(cache.member_permissions(GUILD, STRANGER), None);
    assert_eq!(cache.member_permissions_in(&general, STRANGER), None);
    // The member is not cached.
    assert_eq!(cache.member_permissions(GUILD, UserId::new(77)), None);
    // The guild is not cached.
    assert_eq!(cache.member_permissions(GuildId::new(77), MODERATOR), None);
    // A thread whose parent is not cached.
    let orphan = thread(ChannelId::new(40), ChannelId::new(41));
    assert_eq!(cache.member_permissions_in(&orphan, MODERATOR), None);
    // A DM has no guild.
    let dm: Channel = decode(json!({ "id": "50", "type": 1, "recipients": [] }));
    assert_eq!(cache.member_permissions_in(&dm, MODERATOR), None);
    // The default cache keeps no members.
    let default = Cache::new();
    default.store_member(GUILD, &member(MODERATOR, &[MODERATORS], None));
    assert_eq!(default.member_permissions(GUILD, MODERATOR), None);
}
