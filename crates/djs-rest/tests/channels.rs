//! The channel and thread endpoints against the mock API: method, path, query, body and
//! audit log reason of every request.

use djs_model::{
    channel::{Channel, ChannelType, OverwriteType},
    id::{ChannelId, GenericId, MessageId, RoleId, UserId},
    permissions::Permissions,
    rest::{
        CreateChannelInvite, EditChannelPermissions, FileUpload, FollowAnnouncementChannel,
        ForumThreadMessage, GetChannelPermissions, GetThreadMember, GroupDmAddRecipient,
        ListArchivedThreads, ListJoinedPrivateArchivedThreads, ListThreadMembers, ModifyGroupDm,
        ModifyGuildChannel, ModifyThread, SetVoiceChannelStatus, StartThreadFromMessage,
        StartThreadInForumOrMediaChannel, StartThreadWithoutMessage,
    },
    util::{Patch, Timestamp},
};

mod common;

use common::{Canned, mock, seen};

const TEXT_CHANNEL: &str = r#"{
    "id": "1200000000000000000", "guild_id": "1100000000000000000", "name": "general",
    "type": 0, "position": 6, "permission_overwrites": [], "rate_limit_per_user": 2,
    "nsfw": false, "topic": null, "last_message_id": null, "parent_id": null
}"#;

const THREAD: &str = r#"{
    "id": "1300000000000000001", "guild_id": "1100000000000000000",
    "parent_id": "1200000000000000000", "owner_id": "1", "name": "help", "type": 11,
    "last_message_id": null, "rate_limit_per_user": 0,
    "thread_metadata": {"archived": false, "auto_archive_duration": 1440,
                        "archive_timestamp": "2026-11-11T12:00:00+00:00", "locked": false}
}"#;

const THREAD_MEMBER: &str = r#"{"id": "1300000000000000001", "user_id": "7", "join_timestamp": "2026-11-11T12:00:00+00:00", "flags": 1}"#;

const MESSAGE: &str = r#"{
    "id": "1300000000000000001", "channel_id": "1300000000000000001",
    "author": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
               "avatar": null, "bot": true},
    "content": "It crashes", "timestamp": "2026-11-11T12:00:00.000000+00:00",
    "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
    "type": 0
}"#;

const INVITE: &str = r#"{
    "type": 0, "code": "abc", "channel": {"id": "1200000000000000000", "name": "general", "type": 0},
    "expires_at": null, "uses": 0, "max_uses": 1, "max_age": 0, "temporary": false,
    "created_at": "2026-11-11T12:00:00+00:00"
}"#;

const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);
const THREAD_ID: ChannelId = ChannelId::new(1_300_000_000_000_000_001);
const USER: UserId = UserId::new(7);
const ROLE: RoleId = RoleId::new(8);

/// `(method, path after /api/v10, query, body, audit log reason)`.
type Expected = (
    &'static str,
    String,
    &'static str,
    String,
    Option<&'static str>,
);

fn path(channel: ChannelId, rest: &str) -> String {
    format!("/api/v10/channels/{channel}{rest}")
}

fn no_content() -> Canned {
    Canned::json(204, "")
}

#[tokio::test]
async fn channel_endpoints() {
    let channel = || Canned::json(200, TEXT_CHANNEL);
    let permissions = || Canned::json(200, r#"{"permissions":"3072"}"#);
    let (rest, state) = mock(vec![
        channel(),
        channel(),
        Canned::json(200, THREAD),
        channel(),
        channel(),
        permissions(),
        permissions(),
        no_content(),
        no_content(),
        no_content(),
    ])
    .await;

    let fetched = rest.get_channel(CHANNEL).await.unwrap();
    assert!(matches!(fetched, Channel::Text(_)));

    let mut modify = ModifyGuildChannel::default();
    modify.topic = Patch::Set("Rust".into());
    modify.parent_id = Patch::Clear;
    rest.modify_channel(CHANNEL, &modify, Some("tidy"))
        .await
        .unwrap();
    let mut thread = ModifyThread::default();
    thread.archived = Some(true);
    let archived = rest.modify_thread(THREAD_ID, &thread, None).await.unwrap();
    assert!(matches!(archived, Channel::PublicThread(_)));
    let mut group = ModifyGroupDm::default();
    group.name = Some("friends".into());
    rest.modify_group_dm(CHANNEL, &group).await.unwrap();
    rest.delete_channel(CHANNEL, Some("unused")).await.unwrap();

    let mut implicit = GetChannelPermissions::default();
    implicit.implicit = Some(false);
    let member = rest
        .get_channel_member_permissions(CHANNEL, USER, &implicit)
        .await
        .unwrap();
    assert_eq!(
        member,
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES
    );
    rest.get_channel_role_permissions(CHANNEL, ROLE, &GetChannelPermissions::default())
        .await
        .unwrap();
    rest.set_voice_channel_status(CHANNEL, &SetVoiceChannelStatus::new(None::<&str>), None)
        .await
        .unwrap();

    let mut overwrite = EditChannelPermissions::new(OverwriteType::Role);
    overwrite.deny = Some(Permissions::SEND_MESSAGES);
    rest.edit_channel_permissions(CHANNEL, ROLE.cast(), &overwrite, Some("mute"))
        .await
        .unwrap();
    rest.delete_channel_permission(CHANNEL, GenericId::new(7), None)
        .await
        .unwrap();

    check(&seen(&state), &channel_expectations());
}

/// The requests of `channel_endpoints`, in order.
fn channel_expectations() -> Vec<Expected> {
    let none = String::new;
    vec![
        ("GET", path(CHANNEL, ""), "", none(), None),
        (
            "PATCH",
            path(CHANNEL, ""),
            "",
            r#"{"topic":"Rust","parent_id":null}"#.into(),
            Some("tidy"),
        ),
        (
            "PATCH",
            path(THREAD_ID, ""),
            "",
            r#"{"archived":true}"#.into(),
            None,
        ),
        (
            "PATCH",
            path(CHANNEL, ""),
            "",
            r#"{"name":"friends"}"#.into(),
            None,
        ),
        ("DELETE", path(CHANNEL, ""), "", none(), Some("unused")),
        (
            "GET",
            path(CHANNEL, "/permissions/member/7"),
            "implicit=false",
            none(),
            None,
        ),
        (
            "GET",
            path(CHANNEL, "/permissions/role/8"),
            "",
            none(),
            None,
        ),
        (
            "PUT",
            path(CHANNEL, "/voice-status"),
            "",
            r#"{"status":null}"#.into(),
            None,
        ),
        (
            "PUT",
            path(CHANNEL, "/permissions/8"),
            "",
            r#"{"deny":"2048","type":0}"#.into(),
            Some("mute"),
        ),
        ("DELETE", path(CHANNEL, "/permissions/7"), "", none(), None),
    ]
}

#[tokio::test]
async fn invite_follow_typing_and_group_dm_endpoints() {
    let (rest, state) = mock(vec![
        Canned::json(200, &format!("[{INVITE}]")),
        Canned::json(200, INVITE),
        Canned::json(
            200,
            r#"{"channel_id":"1200000000000000000","webhook_id":"9"}"#,
        ),
        no_content(),
        // Undocumented responses are discarded, whatever they are.
        Canned::json(201, r#"{"id":"1200000000000000000"}"#),
        no_content(),
    ])
    .await;

    let invites = rest.get_channel_invites(CHANNEL).await.unwrap();
    assert_eq!(invites[0].code, "abc");
    let mut invite = CreateChannelInvite::default();
    invite.max_uses = Some(1);
    invite.role_ids.push(ROLE);
    let created = rest
        .create_channel_invite(CHANNEL, &invite, Some("event"))
        .await
        .unwrap();
    assert_eq!(created.metadata.unwrap().max_uses, 1);
    let followed = rest
        .follow_announcement_channel(CHANNEL, &FollowAnnouncementChannel::new(THREAD_ID), None)
        .await
        .unwrap();
    assert_eq!(followed.channel_id, CHANNEL);
    rest.trigger_typing_indicator(CHANNEL).await.unwrap();
    let mut recipient = GroupDmAddRecipient::new("user-access-token");
    recipient.nick = Some("Ferris".into());
    rest.group_dm_add_recipient(CHANNEL, USER, &recipient)
        .await
        .unwrap();
    rest.group_dm_remove_recipient(CHANNEL, USER).await.unwrap();

    let none = String::new;
    let expected: Vec<Expected> = vec![
        ("GET", path(CHANNEL, "/invites"), "", none(), None),
        (
            "POST",
            path(CHANNEL, "/invites"),
            "",
            r#"{"max_uses":1,"role_ids":["8"]}"#.into(),
            Some("event"),
        ),
        (
            "POST",
            path(CHANNEL, "/followers"),
            "",
            r#"{"webhook_channel_id":"1300000000000000001"}"#.into(),
            None,
        ),
        ("POST", path(CHANNEL, "/typing"), "", none(), None),
        (
            "PUT",
            path(CHANNEL, "/recipients/7"),
            "",
            r#"{"access_token":"user-access-token","nick":"Ferris"}"#.into(),
            None,
        ),
        ("DELETE", path(CHANNEL, "/recipients/7"), "", none(), None),
    ];
    check(&seen(&state), &expected);
}

#[tokio::test]
async fn invite_target_users_file_is_a_named_form_field() {
    let (rest, state) = mock(vec![Canned::json(200, INVITE)]).await;
    let mut invite = CreateChannelInvite::default();
    invite.target_users_file = Some(FileUpload::new("users.csv", "7\n8\n"));
    rest.create_channel_invite(CHANNEL, &invite, None)
        .await
        .unwrap();
    let request = &seen(&state)[0];
    assert!(
        request
            .content_type
            .as_deref()
            .unwrap()
            .starts_with("multipart/form-data; boundary=")
    );
    assert!(request.body.contains(
        "Content-Disposition: form-data; name=\"target_users_file\"; filename=\"users.csv\"\r\n\
         Content-Type: text/csv; charset=utf-8\r\n\r\n7\n8\n\r\n"
    ));
    assert!(request.body.contains(
        "Content-Disposition: form-data; name=\"payload_json\"\r\n\
         Content-Type: application/json\r\n\r\n{}\r\n"
    ));
}

#[tokio::test]
async fn thread_endpoints() {
    let thread = || Canned::json(200, THREAD);
    let archived = || {
        Canned::json(
            200,
            &format!(r#"{{"threads":[{THREAD}],"members":[{THREAD_MEMBER}],"has_more":false}}"#),
        )
    };
    let (rest, state) = mock(vec![
        thread(),
        thread(),
        no_content(),
        no_content(),
        no_content(),
        no_content(),
        Canned::json(200, THREAD_MEMBER),
        Canned::json(200, &format!("[{THREAD_MEMBER}]")),
        archived(),
        archived(),
        archived(),
    ])
    .await;

    let from_message = StartThreadFromMessage::new("about this");
    rest.start_thread_from_message(CHANNEL, MessageId::new(5), &from_message, Some("why"))
        .await
        .unwrap();
    let mut private = StartThreadWithoutMessage::new("ticket", ChannelType::PrivateThread);
    private.invitable = Some(false);
    rest.start_thread_without_message(CHANNEL, &private, None)
        .await
        .unwrap();

    rest.join_thread(THREAD_ID).await.unwrap();
    rest.add_thread_member(THREAD_ID, USER).await.unwrap();
    rest.leave_thread(THREAD_ID).await.unwrap();
    rest.remove_thread_member(THREAD_ID, USER).await.unwrap();
    let mut with_member = GetThreadMember::default();
    with_member.with_member = Some(true);
    let member = rest
        .get_thread_member(THREAD_ID, USER, &with_member)
        .await
        .unwrap();
    assert_eq!(member.user_id, Some(USER));
    let mut page = ListThreadMembers::default();
    page.with_member = Some(true);
    page.after = Some(UserId::new(3));
    page.limit = Some(50);
    rest.list_thread_members(THREAD_ID, &page).await.unwrap();

    let mut before = ListArchivedThreads::default();
    before.before = Some(Timestamp::from_unix_millis(1_767_225_600_000).unwrap());
    before.limit = Some(10);
    let public = rest
        .list_public_archived_threads(CHANNEL, &before)
        .await
        .unwrap();
    assert_eq!((public.threads.len(), public.members.len()), (1, 1));
    rest.list_private_archived_threads(CHANNEL, &ListArchivedThreads::default())
        .await
        .unwrap();
    let mut joined = ListJoinedPrivateArchivedThreads::default();
    joined.before = Some(THREAD_ID);
    rest.list_joined_private_archived_threads(CHANNEL, &joined)
        .await
        .unwrap();

    check(&seen(&state), &thread_expectations());
}

/// The requests of `thread_endpoints`, in order.
fn thread_expectations() -> Vec<Expected> {
    let none = String::new;
    let members = |rest: &str| path(THREAD_ID, &format!("/thread-members{rest}"));
    vec![
        (
            "POST",
            path(CHANNEL, "/messages/5/threads"),
            "",
            r#"{"name":"about this"}"#.into(),
            Some("why"),
        ),
        (
            "POST",
            path(CHANNEL, "/threads"),
            "",
            r#"{"name":"ticket","type":12,"invitable":false}"#.into(),
            None,
        ),
        ("PUT", members("/@me"), "", none(), None),
        ("PUT", members("/7"), "", none(), None),
        ("DELETE", members("/@me"), "", none(), None),
        ("DELETE", members("/7"), "", none(), None),
        ("GET", members("/7"), "with_member=true", none(), None),
        (
            "GET",
            members(""),
            "with_member=true&after=3&limit=50",
            none(),
            None,
        ),
        (
            "GET",
            path(CHANNEL, "/threads/archived/public"),
            "before=2026-01-01T00%3A00%3A00.000000%2B00%3A00&limit=10",
            none(),
            None,
        ),
        (
            "GET",
            path(CHANNEL, "/threads/archived/private"),
            "",
            none(),
            None,
        ),
        (
            "GET",
            path(CHANNEL, "/users/@me/threads/archived/private"),
            "before=1300000000000000001",
            none(),
            None,
        ),
    ]
}

#[tokio::test]
async fn forum_thread_returns_the_thread_and_its_message() {
    let response = format!("{}, \"message\": {MESSAGE}}}", THREAD.trim_end_matches('}'));
    let (rest, state) = mock(vec![Canned::json(201, &response)]).await;
    let mut message = ForumThreadMessage::from("It crashes");
    message.files.push(FileUpload::new("trace.txt", "panicked"));
    let mut body = StartThreadInForumOrMediaChannel::new("Bug", message);
    body.applied_tags.push(djs_model::id::TagId::new(4));

    let post = rest
        .start_thread_in_forum_or_media_channel(CHANNEL, &body, Some("report"))
        .await
        .unwrap();
    assert!(matches!(post.channel, Channel::PublicThread(_)));
    assert_eq!(post.message.unwrap().content, "It crashes");

    let request = &seen(&state)[0];
    assert_eq!(request.path, path(CHANNEL, "/threads"));
    assert_eq!(request.audit_log_reason.as_deref(), Some("report"));
    assert!(
        request
            .body
            .contains("name=\"files[0]\"; filename=\"trace.txt\"")
    );
    assert!(request.body.contains(concat!(
        r#"{"name":"Bug","message":{"content":"It crashes","attachments":"#,
        r#"[{"id":0,"filename":"trace.txt"}]},"applied_tags":["4"]}"#
    )));
}

fn check(seen: &[common::Seen], expected: &[Expected]) {
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body, reason)) in seen.iter().zip(expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, *path, "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}
