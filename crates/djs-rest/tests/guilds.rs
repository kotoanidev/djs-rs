//! The guild endpoints against the mock API: method, path, query, body and audit log reason
//! of every request.

use djs_model::{
    channel::{ChannelType, Overwrite},
    guild::{OnboardingMode, WelcomeScreenChannel},
    id::{ChannelId, GuildId, IntegrationId, RoleId, UserId},
    permissions::Permissions,
    rest::{
        AddGuildMember, BeginGuildPrune, BulkGuildBan, CreateGuildBan, CreateGuildChannel,
        CreateGuildRole, GetGuild, GetGuildBans, GetGuildPruneCount, ListGuildMembers,
        ModifyCurrentMember, ModifyGuild, ModifyGuildChannelPosition, ModifyGuildIncidentActions,
        ModifyGuildMember, ModifyGuildOnboarding, ModifyGuildRole, ModifyGuildRolePosition,
        ModifyGuildWelcomeScreen, ModifyGuildWidget, SearchGuildMembers,
    },
    util::Patch,
};

mod common;

use common::{Canned, mock, seen};

/// The docs' JSON examples of `djs-model`.
macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            "../../djs-model/tests/fixtures/docs/",
            $name,
            ".json"
        ))
    };
}

const CHANNEL: &str = r#"{
    "id": "1200000000000000000", "guild_id": "1100000000000000000", "name": "general",
    "type": 0, "position": 0, "permission_overwrites": [], "rate_limit_per_user": 0,
    "nsfw": false, "topic": null, "last_message_id": null, "parent_id": null
}"#;

const MEMBER: &str = r#"{
    "user": {"id": "7", "username": "ferris", "discriminator": "0", "global_name": null, "avatar": null},
    "nick": null, "roles": ["8"], "joined_at": "2026-11-11T12:00:00+00:00", "deaf": false,
    "mute": false, "flags": 0
}"#;

const ROLE: &str = r#"{
    "id": "8", "name": "mods", "color": 0, "colors": {"primary_color": 0, "secondary_color": null,
    "tertiary_color": null}, "hoist": false, "position": 1, "permissions": "8",
    "managed": false, "mentionable": false, "flags": 0
}"#;

const GUILD: GuildId = GuildId::new(1_100_000_000_000_000_000);
const USER: UserId = UserId::new(7);
const ROLE_ID: RoleId = RoleId::new(8);

/// `(method, path after /api/v10/guilds/{GUILD}, query, body, audit log reason)`.
type Expected = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

fn no_content() -> Canned {
    Canned::json(204, "")
}

fn json(body: &str) -> Canned {
    Canned::json(200, body)
}

#[tokio::test]
async fn guild_and_channel_endpoints() {
    let (rest, state) = mock(vec![
        json(fixture!("guild")),
        json(fixture!("guild_preview")),
        json(fixture!("guild")),
        json(&format!("[{CHANNEL}]")),
        Canned::json(201, CHANNEL),
        no_content(),
        json(r#"{"threads":[],"members":[]}"#),
    ])
    .await;

    let mut counts = GetGuild::default();
    counts.with_counts = Some(true);
    rest.get_guild(GUILD, &counts).await.unwrap();
    rest.get_guild_preview(GUILD).await.unwrap();
    let mut modify = ModifyGuild::default();
    modify.name = Some("Rustaceans".into());
    modify.system_channel_id = Patch::Clear;
    rest.modify_guild(GUILD, &modify, Some("rename"))
        .await
        .unwrap();
    assert_eq!(rest.get_guild_channels(GUILD).await.unwrap().len(), 1);

    let mut channel = CreateGuildChannel::new("news");
    channel.kind = Some(ChannelType::GuildAnnouncement);
    channel.permission_overwrites.push(Overwrite::role(
        GUILD.cast(),
        Permissions::empty(),
        Permissions::SEND_MESSAGES,
    ));
    rest.create_guild_channel(GUILD, &channel, None)
        .await
        .unwrap();
    let mut moved = ModifyGuildChannelPosition::new(ChannelId::new(12), 3);
    moved.parent_id = Patch::Clear;
    rest.modify_guild_channel_positions(GUILD, &[moved], Some("tidy"))
        .await
        .unwrap();
    let active = rest.list_active_guild_threads(GUILD).await.unwrap();
    assert_eq!(active.threads.len(), 0);

    check(
        &seen(&state),
        &[
            ("GET", "", "with_counts=true", "", None),
            ("GET", "/preview", "", "", None),
            (
                "PATCH",
                "",
                "",
                r#"{"name":"Rustaceans","system_channel_id":null}"#,
                Some("rename"),
            ),
            ("GET", "/channels", "", "", None),
            (
                "POST",
                "/channels",
                "",
                concat!(
                    r#"{"name":"news","type":5,"permission_overwrites":[{"id":"1100000000000000000","#,
                    r#""type":0,"allow":"0","deny":"2048"}]}"#
                ),
                None,
            ),
            (
                "PATCH",
                "/channels",
                "",
                r#"[{"id":"12","position":3,"parent_id":null}]"#,
                Some("tidy"),
            ),
            ("GET", "/threads/active", "", "", None),
        ],
    );
}

#[tokio::test]
async fn member_endpoints() {
    let member = || json(MEMBER);
    let (rest, state) = mock(vec![
        member(),
        json(&format!("[{MEMBER}]")),
        json(&format!("[{MEMBER}]")),
        json(r#"{"permissions":"8"}"#),
        Canned::json(201, MEMBER),
        no_content(),
        member(),
        member(),
        no_content(),
        no_content(),
        no_content(),
    ])
    .await;

    rest.get_guild_member(GUILD, USER).await.unwrap();
    let mut page = ListGuildMembers::default();
    page.limit = Some(1000);
    page.after = Some(UserId::new(5));
    rest.list_guild_members(GUILD, &page).await.unwrap();
    let mut search = SearchGuildMembers::new("fer ris");
    search.limit = Some(10);
    rest.search_guild_members(GUILD, &search).await.unwrap();
    let permissions = rest
        .get_guild_member_permissions(GUILD, USER)
        .await
        .unwrap();
    assert_eq!(permissions, Permissions::ADMINISTRATOR);

    let mut add = AddGuildMember::new("user-access-token");
    add.roles.push(ROLE_ID);
    assert!(
        rest.add_guild_member(GUILD, USER, &add)
            .await
            .unwrap()
            .is_some()
    );
    // 204: already a member.
    assert!(
        rest.add_guild_member(GUILD, USER, &add)
            .await
            .unwrap()
            .is_none()
    );

    let mut modify = ModifyGuildMember::default();
    modify.nick = Patch::Set("Ferris".into());
    modify.communication_disabled_until = Patch::Clear;
    rest.modify_guild_member(GUILD, USER, &modify, Some("pardon"))
        .await
        .unwrap();
    let mut me = ModifyCurrentMember::default();
    me.bio = Patch::Set("I am a bot".into());
    rest.modify_current_member(GUILD, &me, None).await.unwrap();
    rest.add_guild_member_role(GUILD, USER, ROLE_ID, Some("promote"))
        .await
        .unwrap();
    rest.remove_guild_member_role(GUILD, USER, ROLE_ID, None)
        .await
        .unwrap();
    rest.remove_guild_member(GUILD, USER, Some("kick"))
        .await
        .unwrap();

    let add_body = r#"{"access_token":"user-access-token","roles":["8"]}"#;
    check(
        &seen(&state),
        &[
            ("GET", "/members/7", "", "", None),
            ("GET", "/members", "limit=1000&after=5", "", None),
            (
                "GET",
                "/members/search",
                "query=fer%20ris&limit=10",
                "",
                None,
            ),
            ("GET", "/permissions/member/7", "", "", None),
            ("PUT", "/members/7", "", add_body, None),
            ("PUT", "/members/7", "", add_body, None),
            (
                "PATCH",
                "/members/7",
                "",
                r#"{"nick":"Ferris","communication_disabled_until":null}"#,
                Some("pardon"),
            ),
            ("PATCH", "/members/@me", "", r#"{"bio":"I am a bot"}"#, None),
            ("PUT", "/members/7/roles/8", "", "", Some("promote")),
            ("DELETE", "/members/7/roles/8", "", "", None),
            ("DELETE", "/members/7", "", "", Some("kick")),
        ],
    );
}

#[tokio::test]
async fn ban_and_role_endpoints() {
    let ban = fixture!("ban");
    let (rest, state) = mock(vec![
        json(&format!("[{ban}]")),
        json(ban),
        no_content(),
        no_content(),
        json(r#"{"banned_users":["7"],"failed_users":["9"]}"#),
        json(&format!("[{ROLE}]")),
        json(ROLE),
        json(r#"{"permissions":"1024"}"#),
        json(r#"{"8": 12, "9": 0}"#),
        json(ROLE),
        json(&format!("[{ROLE}]")),
        json(ROLE),
        no_content(),
    ])
    .await;

    let mut page = GetGuildBans::default();
    page.before = Some(UserId::new(100));
    rest.get_guild_bans(GUILD, &page).await.unwrap();
    rest.get_guild_ban(GUILD, USER).await.unwrap();
    let mut create = CreateGuildBan::default();
    create.delete_message_seconds = Some(3600);
    rest.create_guild_ban(GUILD, USER, &create, Some("spam"))
        .await
        .unwrap();
    rest.remove_guild_ban(GUILD, USER, None).await.unwrap();
    let bulk = rest
        .bulk_guild_ban(
            GUILD,
            &BulkGuildBan::new(vec![USER, UserId::new(9)]),
            Some("raid"),
        )
        .await
        .unwrap();
    assert_eq!(
        (bulk.banned_users, bulk.failed_users),
        (vec![USER], vec![UserId::new(9)])
    );

    rest.get_guild_roles(GUILD).await.unwrap();
    rest.get_guild_role(GUILD, ROLE_ID).await.unwrap();
    let permissions = rest
        .get_guild_role_permissions(GUILD, ROLE_ID)
        .await
        .unwrap();
    assert_eq!(permissions, Permissions::VIEW_CHANNEL);
    let counts = rest.get_guild_role_member_counts(GUILD).await.unwrap();
    assert_eq!(counts[&ROLE_ID], 12);
    let mut role = CreateGuildRole::default();
    role.name = Some("mods".into());
    role.permissions = Some(Permissions::KICK_MEMBERS);
    rest.create_guild_role(GUILD, &role, Some("staff"))
        .await
        .unwrap();
    rest.modify_guild_role_positions(GUILD, &[ModifyGuildRolePosition::new(ROLE_ID, 2)], None)
        .await
        .unwrap();
    let mut modify = ModifyGuildRole::default();
    modify.mentionable = Some(true);
    modify.icon = Patch::Clear;
    rest.modify_guild_role(GUILD, ROLE_ID, &modify, None)
        .await
        .unwrap();
    rest.delete_guild_role(GUILD, ROLE_ID, Some("unused"))
        .await
        .unwrap();

    check(&seen(&state), &ban_and_role_expectations());
}

/// The requests of `ban_and_role_endpoints`, in order.
fn ban_and_role_expectations() -> Vec<Expected> {
    vec![
        ("GET", "/bans", "before=100", "", None),
        ("GET", "/bans/7", "", "", None),
        (
            "PUT",
            "/bans/7",
            "",
            r#"{"delete_message_seconds":3600}"#,
            Some("spam"),
        ),
        ("DELETE", "/bans/7", "", "", None),
        (
            "POST",
            "/bulk-ban",
            "",
            r#"{"user_ids":["7","9"]}"#,
            Some("raid"),
        ),
        ("GET", "/roles", "", "", None),
        ("GET", "/roles/8", "", "", None),
        ("GET", "/permissions/role/8", "", "", None),
        ("GET", "/roles/member-counts", "", "", None),
        (
            "POST",
            "/roles",
            "",
            r#"{"name":"mods","permissions":"2"}"#,
            Some("staff"),
        ),
        ("PATCH", "/roles", "", r#"[{"id":"8","position":2}]"#, None),
        (
            "PATCH",
            "/roles/8",
            "",
            r#"{"icon":null,"mentionable":true}"#,
            None,
        ),
        ("DELETE", "/roles/8", "", "", Some("unused")),
    ]
}

#[tokio::test]
async fn prune_widget_and_settings_endpoints() {
    let integration = r#"[{"id": "33", "name": "Twitch", "type": "twitch", "enabled": true,
        "account": {"id": "a", "name": "streamer"}}]"#;
    let (rest, state) = mock(vec![
        json(r#"{"pruned": 4}"#),
        json(r#"{"pruned": null}"#),
        json(r#"[{"id": "eu", "name": "Europe", "optimal": true, "deprecated": false, "custom": false}]"#),
        json("[]"),
        json(integration),
        no_content(),
        json(fixture!("guild_widget_settings")),
        json(fixture!("guild_widget_settings")),
        json(fixture!("guild_widget")),
        json(r#"{"code": "rust", "uses": 12}"#),
        json(fixture!("welcome_screen")),
        json(fixture!("welcome_screen")),
        json(fixture!("guild_onboarding")),
        json(fixture!("guild_onboarding")),
        json(fixture!("incidents_data")),
    ])
    .await;

    let mut count = GetGuildPruneCount::default();
    count.days = Some(30);
    count.include_roles = vec![ROLE_ID, RoleId::new(9)];
    assert_eq!(rest.get_guild_prune_count(GUILD, &count).await.unwrap(), 4);
    let mut prune = BeginGuildPrune::default();
    prune.compute_prune_count = Some(false);
    let pruned = rest
        .begin_guild_prune(GUILD, &prune, Some("cleanup"))
        .await
        .unwrap();
    assert_eq!(pruned, None);
    rest.get_guild_voice_regions(GUILD).await.unwrap();
    rest.get_guild_invites(GUILD).await.unwrap();
    rest.get_guild_integrations(GUILD).await.unwrap();
    rest.delete_guild_integration(GUILD, IntegrationId::new(33), Some("unused"))
        .await
        .unwrap();

    rest.get_guild_widget_settings(GUILD).await.unwrap();
    let mut widget = ModifyGuildWidget::default();
    widget.enabled = Some(true);
    rest.modify_guild_widget(GUILD, &widget, None)
        .await
        .unwrap();
    rest.get_guild_widget(GUILD).await.unwrap();
    let vanity = rest.get_guild_vanity_url(GUILD).await.unwrap();
    assert_eq!((vanity.code.as_deref(), vanity.uses), (Some("rust"), 12));
    rest.get_guild_welcome_screen(GUILD).await.unwrap();
    let mut welcome = ModifyGuildWelcomeScreen::default();
    welcome.welcome_channels = Patch::Set(vec![WelcomeScreenChannel::new(
        ChannelId::new(12),
        "Say hi",
    )]);
    rest.modify_guild_welcome_screen(GUILD, &welcome, None)
        .await
        .unwrap();
    rest.get_guild_onboarding(GUILD).await.unwrap();
    let mut onboarding = ModifyGuildOnboarding::default();
    onboarding.mode = Some(OnboardingMode::Advanced);
    rest.modify_guild_onboarding(GUILD, &onboarding, None)
        .await
        .unwrap();
    let mut incidents = ModifyGuildIncidentActions::default();
    incidents.dms_disabled_until = Patch::Clear;
    rest.modify_guild_incident_actions(GUILD, &incidents)
        .await
        .unwrap();

    check(&seen(&state), &prune_widget_and_settings_expectations());
}

/// The requests of `prune_widget_and_settings_endpoints`, in order.
fn prune_widget_and_settings_expectations() -> Vec<Expected> {
    vec![
        ("GET", "/prune", "days=30&include_roles=8,9", "", None),
        (
            "POST",
            "/prune",
            "",
            r#"{"compute_prune_count":false}"#,
            Some("cleanup"),
        ),
        ("GET", "/regions", "", "", None),
        ("GET", "/invites", "", "", None),
        ("GET", "/integrations", "", "", None),
        ("DELETE", "/integrations/33", "", "", Some("unused")),
        ("GET", "/widget", "", "", None),
        ("PATCH", "/widget", "", r#"{"enabled":true}"#, None),
        ("GET", "/widget.json", "", "", None),
        ("GET", "/vanity-url", "", "", None),
        ("GET", "/welcome-screen", "", "", None),
        (
            "PATCH",
            "/welcome-screen",
            "",
            r#"{"welcome_channels":[{"channel_id":"12","description":"Say hi","emoji_id":null,"emoji_name":null}]}"#,
            None,
        ),
        ("GET", "/onboarding", "", "", None),
        ("PUT", "/onboarding", "", r#"{"mode":1}"#, None),
        (
            "PUT",
            "/incident-actions",
            "",
            r#"{"dms_disabled_until":null}"#,
            None,
        ),
    ]
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body, reason)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(
            request.path,
            format!("/api/v10/guilds/{GUILD}{path}"),
            "{what}"
        );
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
