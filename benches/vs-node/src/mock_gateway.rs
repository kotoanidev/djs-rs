//! The mock Discord gateway. Each connection gets one session: `HELLO`, then after `IDENTIFY`
//! a `READY` listing `--guilds` unavailable guilds, their `GUILD_CREATE`s and finally
//! `--messages` `MESSAGE_CREATE`s sent as fast as the client reads them.

use std::{sync::Arc, time::Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Message, ServerBuilder};

use crate::{
    Options,
    metrics::cpu_ms,
    mock_rest,
    payloads::{GuildShape, HELLO, guild_create, message_create, ready},
};

#[derive(Debug)]
struct Session {
    guilds: u64,
    shape: GuildShape,
    messages: u64,
    authors: u64,
}

/// Serves the gateway and `GET /gateway/bot` on two free ports, after printing
/// `{"api": …, "gateway": …}`.
pub(crate) async fn run(options: &Options) {
    let shape = GuildShape {
        members: options.get("members", 1000),
        ..GuildShape::default()
    };
    let session = Arc::new(Session {
        guilds: options.get("guilds", 1),
        shape,
        messages: options.get("messages", 0),
        authors: options.get("authors", shape.members.clamp(1, 1000)),
    });
    let gateway = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let http = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let url: Arc<str> = format!("ws://{}", gateway.local_addr().expect("bound")).into();
    println!(
        "{}",
        json!({
            "api": format!("http://{}/api", http.local_addr().expect("bound")),
            "gateway": &*url,
        })
    );
    tokio::spawn(mock_rest::serve(http, Some(url)));
    loop {
        let Ok((stream, _)) = gateway.accept().await else {
            continue;
        };
        let session = Arc::clone(&session);
        tokio::spawn(async move {
            if let Err(error) = serve(stream, &session).await {
                eprintln!("[mock-gateway] session ended: {error}");
            }
        });
    }
}

async fn serve(stream: TcpStream, session: &Session) -> Result<(), tokio_websockets::Error> {
    stream.set_nodelay(true)?;
    let (_, mut ws) = ServerBuilder::new().accept(stream).await?;
    ws.send(Message::text(HELLO.to_owned())).await?;
    while let Some(message) = ws.next().await {
        if message?
            .as_text()
            .is_some_and(|text| text.contains(r#""op":2"#))
        {
            break;
        }
    }

    ws.send(Message::text(ready(session.guilds))).await?;
    let mut sequence = 1;
    for guild in 0..session.guilds {
        sequence += 1;
        ws.feed(Message::text(guild_create(sequence, guild, &session.shape)))
            .await?;
    }
    ws.flush().await?;

    let (started, cpu_before) = (Instant::now(), cpu_ms());
    for i in 0..session.messages {
        sequence += 1;
        ws.feed(Message::text(message_create(sequence, i % session.authors)))
            .await?;
        if i % 64 == 63 {
            ws.flush().await?;
        }
    }
    ws.flush().await?;
    if session.messages > 0 {
        let seconds = started.elapsed().as_secs_f64();
        #[expect(
            clippy::cast_precision_loss,
            reason = "message counts are far below 2^52"
        )]
        let rate = session.messages as f64 / seconds;
        eprintln!(
            "[mock-gateway] sent {} messages in {seconds:.2} s ({rate:.0}/s, {:.0} ms CPU)",
            session.messages,
            cpu_ms() - cpu_before,
        );
    }
    // Keep the connection open until the client leaves.
    while let Some(Ok(_)) = ws.next().await {}
    Ok(())
}
