//! The poll endpoints against the mock API: method, path and query of every request, and the
//! unwrapped list of voters.

use djs_model::{
    id::{ChannelId, MessageId, UserId},
    rest::GetAnswerVoters,
};

mod common;

use common::{Canned, mock, seen};

const MESSAGE: &str = r#"{
    "id": "1300000000000000001", "channel_id": "1200000000000000000",
    "author": {"id": "1", "username": "bot", "discriminator": "0", "global_name": null,
               "avatar": null, "bot": true},
    "content": "", "timestamp": "2026-11-11T12:00:00.000000+00:00",
    "edited_timestamp": null, "tts": false, "mention_everyone": false, "pinned": false,
    "type": 0
}"#;

const VOTERS: &str = r#"{"users": [
    {"id": "7", "username": "ferris", "discriminator": "0", "global_name": null, "avatar": null},
    {"id": "8", "username": "corro", "discriminator": "0", "global_name": null, "avatar": null}
]}"#;

const CHANNEL: ChannelId = ChannelId::new(1_200_000_000_000_000_000);
const POLL: MessageId = MessageId::new(1_300_000_000_000_000_001);

#[tokio::test]
async fn poll_endpoints() {
    let (rest, state) = mock(vec![Canned::json(200, VOTERS), Canned::json(200, MESSAGE)]).await;

    let mut query = GetAnswerVoters::default();
    query.after = Some(UserId::new(5));
    query.limit = Some(100);
    let voters = rest
        .get_answer_voters(CHANNEL, POLL, 2, &query)
        .await
        .unwrap();
    assert_eq!(
        voters.iter().map(|user| user.id).collect::<Vec<_>>(),
        [UserId::new(7), UserId::new(8)]
    );
    let message = rest.end_poll(CHANNEL, POLL).await.unwrap();
    assert_eq!(message.id, POLL);

    let seen = seen(&state);
    assert_eq!(seen.len(), 2);
    let base = format!("/api/v10/channels/{CHANNEL}/polls/{POLL}");
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path, format!("{base}/answers/2"));
    assert_eq!(seen[0].query, "after=5&limit=100");
    assert_eq!(seen[1].method, "POST");
    assert_eq!(seen[1].path, format!("{base}/expire"));
    assert_eq!(seen[1].body, "");
}
