//! The permission computation (D-292, D-294, D-295): Discord's documented algorithm, its
//! implicit denials and timeouts.

use djs_model::{
    channel::Overwrite,
    id::{GuildId, RoleId, UserId},
    permissions::Permissions,
};

const GUILD: GuildId = GuildId::new(1);
const USER: UserId = UserId::new(2);
const HELPER: RoleId = RoleId::new(3);
const MUTED: RoleId = RoleId::new(4);

/// What `@everyone` may do in the guild of these tests.
fn everyone() -> Permissions {
    Permissions::VIEW_CHANNEL
        | Permissions::SEND_MESSAGES
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::EMBED_LINKS
        | Permissions::ATTACH_FILES
}

#[test]
fn the_owner_and_administrators_have_everything() {
    assert_eq!(
        Permissions::guild_level(true, Permissions::empty(), []),
        Permissions::all()
    );
    assert_eq!(
        Permissions::guild_level(false, everyone(), [Permissions::ADMINISTRATOR]),
        Permissions::all()
    );
    // `@everyone` itself can grant it.
    assert_eq!(
        Permissions::guild_level(false, Permissions::ADMINISTRATOR, []),
        Permissions::all()
    );
    // Overwrites cannot take anything from them, and timeouts do not apply.
    let deny_all = [Overwrite::role(
        GUILD.cast(),
        Permissions::empty(),
        Permissions::all(),
    )];
    let admin = Permissions::all().in_channel(GUILD, USER, &[], &deny_all);
    assert_eq!(admin, Permissions::all());
    assert_eq!(admin.during_timeout(), Permissions::all());
}

#[test]
fn roles_add_to_everyone() {
    let base = Permissions::guild_level(
        false,
        everyone(),
        [Permissions::KICK_MEMBERS, Permissions::MANAGE_MESSAGES],
    );
    assert_eq!(
        base,
        everyone() | Permissions::KICK_MEMBERS | Permissions::MANAGE_MESSAGES
    );
    assert_eq!(Permissions::guild_level(false, everyone(), []), everyone());
}

#[test]
fn overwrites_apply_in_discords_order() {
    let base = everyone();
    let overwrites = [
        // `@everyone` may not send here…
        Overwrite::role(
            GUILD.cast(),
            Permissions::empty(),
            Permissions::SEND_MESSAGES,
        ),
        // …the helpers may (an allow of one role beats a deny of another)…
        Overwrite::role(HELPER, Permissions::SEND_MESSAGES, Permissions::empty()),
        Overwrite::role(MUTED, Permissions::empty(), Permissions::SEND_MESSAGES),
        // …and this member may not attach files, whatever their roles say.
        Overwrite::member(USER, Permissions::empty(), Permissions::ATTACH_FILES),
        // Other roles and members do not count.
        Overwrite::role(
            RoleId::new(99),
            Permissions::ADMINISTRATOR,
            Permissions::empty(),
        ),
        Overwrite::member(UserId::new(98), Permissions::all(), Permissions::empty()),
    ];
    let member = base.in_channel(GUILD, USER, &[], &overwrites);
    assert!(!member.contains(Permissions::SEND_MESSAGES));
    let helper = base.in_channel(GUILD, USER, &[HELPER, MUTED], &overwrites);
    assert!(helper.contains(Permissions::SEND_MESSAGES));
    assert!(!helper.contains(Permissions::ATTACH_FILES));
    assert!(helper.contains(Permissions::EMBED_LINKS));

    // The member's own allow beats every role's deny.
    let allowed = [
        Overwrite::role(MUTED, Permissions::empty(), Permissions::SEND_MESSAGES),
        Overwrite::member(USER, Permissions::SEND_MESSAGES, Permissions::empty()),
    ];
    assert!(
        base.in_channel(GUILD, USER, &[MUTED], &allowed)
            .contains(Permissions::SEND_MESSAGES)
    );
}

#[test]
fn an_overwrite_cannot_grant_administrator() {
    let overwrites = [Overwrite::member(
        USER,
        Permissions::ADMINISTRATOR,
        Permissions::empty(),
    )];
    let permissions = everyone().in_channel(GUILD, USER, &[], &overwrites);
    assert_eq!(permissions, everyone());
    // So a timeout still applies.
    assert_eq!(permissions.during_timeout(), Permissions::DURING_TIMEOUT);
}

#[test]
fn implicit_denials() {
    // Without `VIEW_CHANNEL`, nothing is left in the channel (D-295).
    let hidden = [Overwrite::role(
        GUILD.cast(),
        Permissions::empty(),
        Permissions::VIEW_CHANNEL,
    )];
    let base = everyone() | Permissions::KICK_MEMBERS;
    assert_eq!(
        base.in_channel(GUILD, USER, &[], &hidden),
        Permissions::empty()
    );

    // Without `SEND_MESSAGES`, the permissions that need it go too.
    let read_only = [Overwrite::role(
        GUILD.cast(),
        Permissions::empty(),
        Permissions::SEND_MESSAGES,
    )];
    let base = everyone() | Permissions::MENTION_EVERYONE | Permissions::SEND_TTS_MESSAGES;
    assert_eq!(
        base.in_channel(GUILD, USER, &[], &read_only),
        Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY
    );

    // No overwrites: the guild-level permissions, with the same rules.
    assert_eq!(everyone().in_channel(GUILD, USER, &[], &[]), everyone());
    assert_eq!(
        Permissions::SEND_MESSAGES.in_channel(GUILD, USER, &[], &[]),
        Permissions::empty()
    );
}

#[test]
fn timeouts_keep_viewing_and_reading() {
    let base = everyone() | Permissions::KICK_MEMBERS;
    assert_eq!(base.during_timeout(), Permissions::DURING_TIMEOUT);
    assert_eq!(
        Permissions::VIEW_CHANNEL.during_timeout(),
        Permissions::VIEW_CHANNEL
    );
    assert_eq!(Permissions::empty().during_timeout(), Permissions::empty());
}
