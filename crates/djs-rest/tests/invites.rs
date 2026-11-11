//! The invite endpoints against the mock API: method, path, query, body and audit log reason of
//! every request, the parsed target users CSV and the target users file upload.

use djs_model::{
    id::{ScheduledEventId, UserId},
    invite::InviteTargetUsersJobStatusCode,
    rest::{BulkTargetUsers, FileUpload, GetInvite},
};

mod common;

use common::{Canned, mock, seen};

const INVITE: &str = include_str!("../../djs-model/tests/fixtures/docs/invite_invite_object.json");
const CODE: &str = "0vCdhLbwjZZTWZLD";

const JOB: &str = r#"{"status": 3, "total_users": 100, "processed_users": 41,
    "created_at": "2025-01-08T12:00:00.000000+00:00", "completed_at": null,
    "error_message": "Failed to parse CSV file"}"#;

/// `(method, path after /api/v10, query, body, audit log reason)`.
type Expected = (
    &'static str,
    String,
    &'static str,
    &'static str,
    Option<&'static str>,
);

#[tokio::test]
async fn invite_endpoints() {
    let no_content = || Canned::json(204, "");
    let (rest, state) = mock(vec![
        Canned::json(200, INVITE),
        Canned::json(200, INVITE),
        Canned::json(200, "user_id\r\n7\r\n8\r\n").header("content-type", "text/csv"),
        no_content(),
        no_content(),
        no_content(),
        no_content(),
        Canned::json(200, JOB),
    ])
    .await;

    let mut query = GetInvite::default();
    query.with_counts = Some(true);
    query.guild_scheduled_event_id = Some(ScheduledEventId::new(9));
    let invite = rest.get_invite(CODE, &query).await.unwrap();
    assert_eq!(invite.code, CODE);
    rest.delete_invite(CODE, Some("leaked")).await.unwrap();
    assert_eq!(
        rest.get_target_users(CODE).await.unwrap(),
        [UserId::new(7), UserId::new(8)]
    );
    rest.add_target_user(CODE, UserId::new(9)).await.unwrap();
    rest.remove_target_user(CODE, UserId::new(9)).await.unwrap();
    let ids = BulkTargetUsers::new([UserId::new(10), UserId::new(11)]);
    rest.bulk_add_target_users(CODE, &ids).await.unwrap();
    rest.bulk_delete_target_users(CODE, &ids).await.unwrap();
    let job = rest.get_target_users_job_status(CODE).await.unwrap();
    assert_eq!(job.status, InviteTargetUsersJobStatusCode::Failed);
    assert_eq!(job.processed_users, 41);

    let path = format!("/invites/{CODE}");
    let targets = format!("{path}/target-users");
    let bulk = r#"{"user_ids":["10","11"]}"#;
    let expected: Vec<Expected> = vec![
        (
            "GET",
            path.clone(),
            "with_counts=true&guild_scheduled_event_id=9",
            "",
            None,
        ),
        ("DELETE", path, "", "", Some("leaked")),
        ("GET", targets.clone(), "", "", None),
        ("PUT", format!("{targets}/9"), "", "", None),
        ("DELETE", format!("{targets}/9"), "", "", None),
        ("POST", format!("{targets}/bulk-add"), "", bulk, None),
        ("POST", format!("{targets}/bulk-delete"), "", bulk, None),
        ("GET", format!("{targets}/job-status"), "", "", None),
    ];
    let seen = seen(&state);
    assert_eq!(seen.len(), expected.len());
    for (request, (method, path, query, body, reason)) in seen.iter().zip(&expected) {
        let what = format!("{method} {path}");
        assert_eq!(request.method, *method, "{what}");
        assert_eq!(request.path, format!("/api/v10{path}"), "{what}");
        assert_eq!(request.query, *query, "{what}");
        assert_eq!(request.body, *body, "{what}");
        assert_eq!(request.audit_log_reason.as_deref(), *reason, "{what}");
    }
}

#[tokio::test]
async fn update_target_users_uploads_a_csv_file() {
    let (rest, state) = mock(vec![Canned::json(204, "")]).await;
    let file = FileUpload::target_users([UserId::new(7), UserId::new(8)]);
    rest.update_target_users(CODE, &file).await.unwrap();

    let request = &seen(&state)[0];
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.path,
        format!("/api/v10/invites/{CODE}/target-users")
    );
    let boundary = request
        .content_type
        .as_deref()
        .unwrap()
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    assert_eq!(
        request.body,
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"target_users_file\"; \
             filename=\"users.csv\"\r\nContent-Type: text/csv\r\n\r\n7\n8\r\n--{boundary}--\r\n"
        )
    );
}

#[tokio::test]
async fn invalid_target_users_csv_is_a_decode_error() {
    let (rest, _) = mock(vec![Canned::json(200, "user_id\nnot-an-id\n")]).await;
    let error = rest.get_target_users(CODE).await.unwrap_err();
    assert!(matches!(error.kind(), djs_rest::ErrorKind::Deserialize));
}
