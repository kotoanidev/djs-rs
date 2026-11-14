//! The voice gateway's messages, version 8: JSON text messages `{"op", "d", "seq"}` and the
//! binary DAVE messages `[sequence (u16, server only)][opcode][payload]`, both ways, and its
//! close codes.
//!
//! Discord documents them in <https://discord.com/developers/docs/topics/voice-connections>
//! and the voice tables of
//! <https://discord.com/developers/docs/topics/opcodes-and-status-codes>; the DAVE messages
//! in the protocol's whitepaper, <https://daveprotocol.com/#voice-gateway-opcodes>.

#![allow(dead_code, reason = "used by the connection from step 1c")]

use core::{fmt, net::IpAddr};

use djs_model::id::{GuildId, UserId};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::crypto::Mode;

/// The voice gateway version djs-voice speaks: `?v=8` on the connection URL.
pub(crate) const VERSION: u8 = 8;

/// Opcodes.
mod op {
    pub(super) const IDENTIFY: u8 = 0;
    pub(super) const SELECT_PROTOCOL: u8 = 1;
    pub(super) const READY: u8 = 2;
    pub(super) const HEARTBEAT: u8 = 3;
    pub(super) const SESSION_DESCRIPTION: u8 = 4;
    pub(super) const SPEAKING: u8 = 5;
    pub(super) const HEARTBEAT_ACK: u8 = 6;
    pub(super) const RESUME: u8 = 7;
    pub(super) const HELLO: u8 = 8;
    pub(super) const RESUMED: u8 = 9;
    pub(super) const CLIENTS_CONNECT: u8 = 11;
    pub(super) const CLIENT_DISCONNECT: u8 = 13;
    pub(super) const DAVE_PREPARE_TRANSITION: u8 = 21;
    pub(super) const DAVE_EXECUTE_TRANSITION: u8 = 22;
    pub(super) const DAVE_TRANSITION_READY: u8 = 23;
    pub(super) const DAVE_PREPARE_EPOCH: u8 = 24;
    pub(super) const DAVE_EXTERNAL_SENDER: u8 = 25;
    pub(super) const DAVE_KEY_PACKAGE: u8 = 26;
    pub(super) const DAVE_PROPOSALS: u8 = 27;
    pub(super) const DAVE_COMMIT_WELCOME: u8 = 28;
    pub(super) const DAVE_ANNOUNCE_COMMIT: u8 = 29;
    pub(super) const DAVE_WELCOME: u8 = 30;
    pub(super) const DAVE_INVALID_COMMIT_WELCOME: u8 = 31;
}

/// The speaking flags of Opcode 5: normal voice.
pub(crate) const MICROPHONE: u32 = 1 << 0;
/// Context audio, without the speaking indicator.
pub(crate) const SOUNDSHARE: u32 = 1 << 1;
/// Priority speaker: other speakers are quieter.
pub(crate) const PRIORITY: u32 = 1 << 2;

/// A JSON message to the voice gateway.
#[derive(Serialize)]
struct Outgoing<'a, D> {
    op: u8,
    d: &'a D,
}

fn json<D: Serialize>(op: u8, d: &D) -> String {
    // Serializing these plain structures cannot fail.
    serde_json::to_string(&Outgoing { op, d }).unwrap_or_default()
}

/// Opcode 0 Identify: starts a new voice session. `max_dave_protocol_version` is the newest
/// DAVE version supported (0 for none).
pub(crate) fn identify(
    server_id: GuildId,
    user_id: UserId,
    session_id: &str,
    token: &str,
    max_dave_protocol_version: u16,
) -> String {
    #[derive(Serialize)]
    struct Identify<'a> {
        server_id: GuildId,
        user_id: UserId,
        session_id: &'a str,
        token: &'a str,
        max_dave_protocol_version: u16,
    }
    json(
        op::IDENTIFY,
        &Identify {
            server_id,
            user_id,
            session_id,
            token,
            max_dave_protocol_version,
        },
    )
}

/// Opcode 1 Select Protocol: the address and port IP discovery found, and the encryption
/// mode.
pub(crate) fn select_protocol(address: IpAddr, port: u16, mode: Mode) -> String {
    #[derive(Serialize)]
    struct Data {
        address: IpAddr,
        port: u16,
        mode: &'static str,
    }
    #[derive(Serialize)]
    struct SelectProtocol {
        protocol: &'static str,
        data: Data,
    }
    json(
        op::SELECT_PROTOCOL,
        &SelectProtocol {
            protocol: "udp",
            data: Data {
                address,
                port,
                mode: mode.name(),
            },
        },
    )
}

/// Opcode 3 Heartbeat: `nonce` comes back in the acknowledgement; `seq_ack` is the last
/// sequence number received (omitted before the first).
pub(crate) fn heartbeat(nonce: u64, seq_ack: Option<u64>) -> String {
    #[derive(Serialize)]
    struct Heartbeat {
        t: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        seq_ack: Option<u64>,
    }
    json(op::HEARTBEAT, &Heartbeat { t: nonce, seq_ack })
}

/// Opcode 5 Speaking: required once before sending audio; `delay` is 0 for bots.
pub(crate) fn speaking(flags: u32, ssrc: u32) -> String {
    #[derive(Serialize)]
    struct Speaking {
        #[serde(rename = "speaking")]
        flags: u32,
        delay: u32,
        ssrc: u32,
    }
    json(
        op::SPEAKING,
        &Speaking {
            flags,
            delay: 0,
            ssrc,
        },
    )
}

/// Opcode 7 Resume: continues a session on a new connection; the server re-sends the
/// messages after `seq_ack`.
pub(crate) fn resume(
    server_id: GuildId,
    session_id: &str,
    token: &str,
    seq_ack: Option<u64>,
) -> String {
    #[derive(Serialize)]
    struct Resume<'a> {
        server_id: GuildId,
        session_id: &'a str,
        token: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        seq_ack: Option<u64>,
    }
    json(
        op::RESUME,
        &Resume {
            server_id,
            session_id,
            token,
            seq_ack,
        },
    )
}

#[derive(Serialize)]
struct Transition {
    transition_id: u16,
}

/// Opcode 23 DAVE Transition Ready: the client is ready for the announced transition.
pub(crate) fn transition_ready(transition_id: u16) -> String {
    json(op::DAVE_TRANSITION_READY, &Transition { transition_id })
}

/// Opcode 31 DAVE MLS Invalid Commit Welcome: the commit or welcome of a transition could
/// not be processed; the gateway removes and re-adds the client.
pub(crate) fn invalid_commit_welcome(transition_id: u16) -> String {
    json(
        op::DAVE_INVALID_COMMIT_WELCOME,
        &Transition { transition_id },
    )
}

/// Binary Opcode 26 DAVE MLS Key Package: an MLS key package for being added to the group.
pub(crate) fn key_package(package: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(1 + package.len());
    message.push(op::DAVE_KEY_PACKAGE);
    message.extend_from_slice(package);
    message
}

/// Binary Opcode 28 DAVE MLS Commit Welcome: an MLS commit, and the welcome of the members it
/// adds if it adds any.
pub(crate) fn commit_welcome(commit: &[u8], welcome: Option<&[u8]>) -> Vec<u8> {
    let welcome = welcome.unwrap_or_default();
    let mut message = Vec::with_capacity(1 + commit.len() + welcome.len());
    message.push(op::DAVE_COMMIT_WELCOME);
    message.extend_from_slice(commit);
    message.extend_from_slice(welcome);
    message
}

/// Opcode 2 Ready: the session's SSRC, the voice server's UDP address and the encryption
/// modes it offers. (Its `heartbeat_interval` is wrong and ignored, as Discord says.)
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Ready {
    pub(crate) ssrc: u32,
    pub(crate) ip: IpAddr,
    pub(crate) port: u16,
    pub(crate) modes: Vec<String>,
}

/// Opcode 4 Session Description: the mode selected, the key, and the DAVE version the call
/// starts with (0: not end-to-end encrypted).
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct SessionDescription {
    pub(crate) mode: String,
    pub(crate) secret_key: [u8; 32],
    #[serde(default)]
    pub(crate) dave_protocol_version: u16,
}

impl fmt::Debug for SessionDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionDescription")
            .field("mode", &self.mode)
            .field("secret_key", &"<redacted>")
            .field("dave_protocol_version", &self.dave_protocol_version)
            .finish()
    }
}

/// Opcode 5 Speaking, from the server: a user's SSRC and speaking flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Speaking {
    pub(crate) user_id: UserId,
    pub(crate) ssrc: u32,
    #[serde(rename = "speaking")]
    pub(crate) flags: u32,
}

/// What the operation of an Opcode 27 DAVE MLS Proposals does with its proposals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProposalsOperation {
    /// Append the MLS proposal messages that follow.
    Append,
    /// Revoke the proposals whose references follow.
    Revoke,
}

/// A message from the voice gateway.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Event {
    /// Opcode 2.
    Ready(Ready),
    /// Opcode 4.
    SessionDescription(SessionDescription),
    /// Opcode 5.
    Speaking(Speaking),
    /// Opcode 6: the nonce of the heartbeat acknowledged.
    HeartbeatAck(u64),
    /// Opcode 8: the heartbeat interval in milliseconds.
    Hello(f64),
    /// Opcode 9.
    Resumed,
    /// Opcode 11: users who joined the call.
    ClientsConnect(Vec<UserId>),
    /// Opcode 13: a user who left the call.
    ClientDisconnect(UserId),
    /// Opcode 21: a downgrade (to `protocol_version` 0) is announced.
    DavePrepareTransition {
        transition_id: u16,
        protocol_version: u16,
    },
    /// Opcode 22: the transition announced is executed.
    DaveExecuteTransition { transition_id: u16 },
    /// Opcode 24: a DAVE version change or a new group (epoch 1) is announced.
    DavePrepareEpoch { epoch: u64, protocol_version: u16 },
    /// Binary opcode 25: the external sender (the voice gateway) to put in the MLS group.
    DaveExternalSender(Vec<u8>),
    /// Binary opcode 27: MLS proposals to append, or references of proposals to revoke.
    DaveProposals {
        operation: ProposalsOperation,
        data: Vec<u8>,
    },
    /// Binary opcode 29: the commit to apply for a transition.
    DaveAnnounceCommit { transition_id: u16, commit: Vec<u8> },
    /// Binary opcode 30: the welcome into the group for a transition.
    DaveWelcome {
        transition_id: u16,
        welcome: Vec<u8>,
    },
    /// Any other opcode (Discord sends some it does not document); ignored.
    Other(u8),
}

/// A message from the voice gateway, with its sequence number if it has one (for the
/// buffered resume of version 8).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Incoming {
    pub(crate) seq: Option<u64>,
    pub(crate) event: Event,
}

/// Why a message from the voice gateway could not be read.
#[derive(Debug)]
pub(crate) enum DecodeError {
    /// Not the JSON of a message, or its data does not fit its opcode.
    Json(serde_json::Error),
    /// A binary message too short for its opcode, or with an unknown proposals operation.
    Binary { op: Option<u8> },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "a voice gateway message is malformed: {error}"),
            Self::Binary { op: Some(op) } => {
                write!(
                    f,
                    "a binary voice gateway message (opcode {op}) is malformed"
                )
            }
            Self::Binary { op: None } => f.write_str("a binary voice gateway message is empty"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Binary { .. } => None,
        }
    }
}

/// A JSON message's envelope.
#[derive(Deserialize)]
struct Envelope {
    op: u8,
    #[serde(default)]
    d: Value,
    #[serde(default)]
    seq: Option<u64>,
}

fn data<T: DeserializeOwned>(d: Value) -> Result<T, DecodeError> {
    serde_json::from_value(d).map_err(DecodeError::Json)
}

/// Reads a text message from the voice gateway.
pub(crate) fn decode_text(text: &str) -> Result<Incoming, DecodeError> {
    #[derive(Deserialize)]
    struct Nonce {
        t: u64,
    }
    #[derive(Deserialize)]
    struct Hello {
        heartbeat_interval: f64,
    }
    #[derive(Deserialize)]
    struct UserIds {
        user_ids: Vec<UserId>,
    }
    #[derive(Deserialize)]
    struct User {
        user_id: UserId,
    }
    #[derive(Deserialize)]
    struct Prepare {
        transition_id: u16,
        protocol_version: u16,
    }
    #[derive(Deserialize)]
    struct Execute {
        transition_id: u16,
    }
    #[derive(Deserialize)]
    struct Epoch {
        epoch: u64,
        protocol_version: u16,
    }

    let Envelope { op, d, seq } = serde_json::from_str(text).map_err(DecodeError::Json)?;
    let event = match op {
        op::READY => Event::Ready(data(d)?),
        op::SESSION_DESCRIPTION => Event::SessionDescription(data(d)?),
        op::SPEAKING => Event::Speaking(data(d)?),
        op::HEARTBEAT_ACK => Event::HeartbeatAck(data::<Nonce>(d)?.t),
        op::HELLO => Event::Hello(data::<Hello>(d)?.heartbeat_interval),
        op::RESUMED => Event::Resumed,
        op::CLIENTS_CONNECT => Event::ClientsConnect(data::<UserIds>(d)?.user_ids),
        op::CLIENT_DISCONNECT => Event::ClientDisconnect(data::<User>(d)?.user_id),
        op::DAVE_PREPARE_TRANSITION => {
            let Prepare {
                transition_id,
                protocol_version,
            } = data(d)?;
            Event::DavePrepareTransition {
                transition_id,
                protocol_version,
            }
        }
        op::DAVE_EXECUTE_TRANSITION => Event::DaveExecuteTransition {
            transition_id: data::<Execute>(d)?.transition_id,
        },
        op::DAVE_PREPARE_EPOCH => {
            let Epoch {
                epoch,
                protocol_version,
            } = data(d)?;
            Event::DavePrepareEpoch {
                epoch,
                protocol_version,
            }
        }
        other => Event::Other(other),
    };
    Ok(Incoming { seq, event })
}

/// Reads a binary message from the voice gateway: its sequence number, opcode and payload.
pub(crate) fn decode_binary(bytes: &[u8]) -> Result<Incoming, DecodeError> {
    let &[s0, s1, op, ref payload @ ..] = bytes else {
        return Err(DecodeError::Binary {
            op: bytes.get(2).copied(),
        });
    };
    let malformed = DecodeError::Binary { op: Some(op) };
    let transition = |payload: &[u8]| match payload {
        &[t0, t1, ref rest @ ..] => Some((u16::from_be_bytes([t0, t1]), rest.to_vec())),
        _ => None,
    };
    let event = match op {
        op::DAVE_EXTERNAL_SENDER => Event::DaveExternalSender(payload.to_vec()),
        op::DAVE_PROPOSALS => {
            let (operation, data) = match payload {
                [0, data @ ..] => (ProposalsOperation::Append, data),
                [1, data @ ..] => (ProposalsOperation::Revoke, data),
                _ => return Err(malformed),
            };
            Event::DaveProposals {
                operation,
                data: data.to_vec(),
            }
        }
        op::DAVE_ANNOUNCE_COMMIT => {
            let (transition_id, commit) = transition(payload).ok_or(malformed)?;
            Event::DaveAnnounceCommit {
                transition_id,
                commit,
            }
        }
        op::DAVE_WELCOME => {
            let (transition_id, welcome) = transition(payload).ok_or(malformed)?;
            Event::DaveWelcome {
                transition_id,
                welcome,
            }
        }
        other => Event::Other(other),
    };
    Ok(Incoming {
        seq: Some(u64::from(u16::from_be_bytes([s0, s1]))),
        event,
    })
}

/// What a client does after the voice gateway closed its connection with a code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AfterClose {
    /// Connect again and resume the session (Opcode 7).
    Resume,
    /// Connect again and identify anew with the same voice server details (Opcode 0).
    Identify,
    /// Stop: the call is over, or reconnecting cannot help.
    Stop,
}

/// What to do after a close with `code`; `None` for a connection lost without one.
pub(crate) const fn after_close(code: Option<u16>) -> AfterClose {
    match code {
        // 4006 session no longer valid, 4009 session timeout.
        Some(4006 | 4009) => AfterClose::Identify,
        // 4004 authentication failed, 4011 server not found, 4014 disconnected (kicked,
        // moved, the main gateway session dropped: the bot's voice state tells), 4017 DAVE
        // required, 4021 rate limited, 4022 call terminated; and the codes of a client bug:
        // 4001 unknown opcode, 4002 failed to decode, 4003 not authenticated, 4005 already
        // authenticated, 4012 unknown protocol, 4016 unknown encryption mode, 4020 bad
        // request.
        Some(
            4001 | 4002 | 4003 | 4004 | 4005 | 4011 | 4012 | 4014 | 4016 | 4017 | 4020 | 4021
            | 4022,
        ) => AfterClose::Stop,
        // 4015 voice server crashed, a connection lost without a code, and anything else.
        _ => AfterClose::Resume,
    }
}

#[cfg(test)]
mod tests {
    use core::net::{IpAddr, Ipv4Addr};

    use djs_model::id::{GuildId, UserId};
    use serde_json::{Value, json};

    use super::{
        AfterClose, DecodeError, Event, Incoming, MICROPHONE, PRIORITY, ProposalsOperation, Ready,
        SessionDescription, Speaking, after_close, commit_welcome, decode_binary, decode_text,
        heartbeat, identify, invalid_commit_welcome, key_package, resume, select_protocol,
        speaking, transition_ready,
    };
    use crate::crypto::Mode;

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn commands_match_discords_examples() {
        // The examples of Discord's voice connection guide.
        assert_eq!(
            parsed(&identify(
                GuildId::new(41_771_983_423_143_937),
                UserId::new(104_694_319_306_248_192),
                "my_session_id",
                "my_token",
                1,
            )),
            json!({"op": 0, "d": {
                "server_id": "41771983423143937",
                "user_id": "104694319306248192",
                "session_id": "my_session_id",
                "token": "my_token",
                "max_dave_protocol_version": 1
            }})
        );
        assert_eq!(
            parsed(&select_protocol(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                1337,
                Mode::Aes256Gcm
            )),
            json!({"op": 1, "d": {"protocol": "udp", "data": {
                "address": "127.0.0.1", "port": 1337, "mode": "aead_aes256_gcm_rtpsize"
            }}})
        );
        assert_eq!(
            parsed(&heartbeat(1_501_184_119_561, Some(10))),
            json!({"op": 3, "d": {"t": 1_501_184_119_561_u64, "seq_ack": 10}})
        );
        assert_eq!(parsed(&heartbeat(5, None)), json!({"op": 3, "d": {"t": 5}}));
        assert_eq!(
            parsed(&speaking(MICROPHONE | PRIORITY, 1)),
            json!({"op": 5, "d": {"speaking": 5, "delay": 0, "ssrc": 1}})
        );
        assert_eq!(
            parsed(&resume(
                GuildId::new(41_771_983_423_143_937),
                "my_session_id",
                "my_token",
                Some(10)
            )),
            json!({"op": 7, "d": {
                "server_id": "41771983423143937",
                "session_id": "my_session_id",
                "token": "my_token",
                "seq_ack": 10
            }})
        );
        assert_eq!(
            parsed(&transition_ready(10)),
            json!({"op": 23, "d": {"transition_id": 10}})
        );
        assert_eq!(
            parsed(&invalid_commit_welcome(32)),
            json!({"op": 31, "d": {"transition_id": 32}})
        );
    }

    #[test]
    fn binary_commands_start_with_their_opcode() {
        assert_eq!(key_package(&[1, 2, 3]), [26, 1, 2, 3]);
        assert_eq!(commit_welcome(&[1, 2], None), [28, 1, 2]);
        assert_eq!(commit_welcome(&[1, 2], Some(&[3, 4])), [28, 1, 2, 3, 4]);
    }

    fn event(text: &str) -> Incoming {
        decode_text(text).unwrap()
    }

    #[test]
    fn server_messages_are_read() {
        assert_eq!(
            event(
                r#"{"op":2,"d":{"ssrc":1,"ip":"127.0.0.1","port":1234,"modes":
                ["aead_aes256_gcm_rtpsize","aead_xchacha20_poly1305_rtpsize"],
                "heartbeat_interval":1,"experiments":["x"]}}"#
            ),
            Incoming {
                seq: None,
                event: Event::Ready(Ready {
                    ssrc: 1,
                    ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                    port: 1234,
                    modes: vec![
                        "aead_aes256_gcm_rtpsize".into(),
                        "aead_xchacha20_poly1305_rtpsize".into()
                    ],
                })
            }
        );
        let key: Vec<u8> = (0..32).collect();
        let description = event(
            &json!({"op": 4, "d": {
                "mode": "aead_aes256_gcm_rtpsize", "secret_key": key,
                "dave_protocol_version": 1, "audio_codec": "opus"
            }, "seq": 3})
            .to_string(),
        );
        assert_eq!(description.seq, Some(3));
        let Event::SessionDescription(description) = description.event else {
            panic!("{description:?}");
        };
        assert_eq!(
            (description.mode.as_str(), description.secret_key[31]),
            ("aead_aes256_gcm_rtpsize", 31)
        );
        assert_eq!(description.dave_protocol_version, 1);
        assert!(
            !format!("{description:?}").contains("31"),
            "the key is redacted"
        );
        assert_eq!(
            event(r#"{"op":5,"d":{"speaking":0,"delay":0,"ssrc":110,"user_id":"7"},"seq":10}"#),
            Incoming {
                seq: Some(10),
                event: Event::Speaking(Speaking {
                    user_id: UserId::new(7),
                    ssrc: 110,
                    flags: 0
                })
            }
        );
        assert_eq!(
            event(r#"{"op":6,"d":{"t":1501184119561}}"#).event,
            Event::HeartbeatAck(1_501_184_119_561)
        );
        assert_eq!(
            event(r#"{"op":8,"d":{"heartbeat_interval":41250.5}}"#).event,
            Event::Hello(41_250.5)
        );
        assert_eq!(event(r#"{"op":9,"d":null}"#).event, Event::Resumed);
        assert_eq!(event(r#"{"op":9}"#).event, Event::Resumed);
        assert_eq!(
            event(r#"{"op":11,"d":{"user_ids":["1","2"]}}"#).event,
            Event::ClientsConnect(vec![UserId::new(1), UserId::new(2)])
        );
        assert_eq!(
            event(r#"{"op":13,"d":{"user_id":"3"}}"#).event,
            Event::ClientDisconnect(UserId::new(3))
        );
        assert_eq!(
            event(r#"{"op":21,"d":{"transition_id":10,"protocol_version":0}}"#).event,
            Event::DavePrepareTransition {
                transition_id: 10,
                protocol_version: 0
            }
        );
        assert_eq!(
            event(r#"{"op":22,"d":{"transition_id":10}}"#).event,
            Event::DaveExecuteTransition { transition_id: 10 }
        );
        assert_eq!(
            event(r#"{"op":24,"d":{"epoch":1,"protocol_version":1}}"#).event,
            Event::DavePrepareEpoch {
                epoch: 1,
                protocol_version: 1
            }
        );
        // Opcodes Discord sends without documenting them are kept apart, not errors.
        assert_eq!(
            event(r#"{"op":18,"d":{"user_id":"1","flags":2}}"#).event,
            Event::Other(18)
        );
    }

    #[test]
    fn malformed_text_messages_fail() {
        for text in [
            "",
            "not json",
            r#"{"d":{}}"#,
            r#"{"op":2,"d":{"ssrc":1}}"#,
            r#"{"op":4,"d":{"mode":"m","secret_key":[1,2,3]}}"#,
            r#"{"op":6,"d":{"t":-1}}"#,
            r#"{"op":13,"d":{"user_id":"0"}}"#,
        ] {
            assert!(
                matches!(decode_text(text), Err(DecodeError::Json(_))),
                "{text}"
            );
        }
    }

    #[test]
    fn binary_messages_are_read() {
        assert_eq!(
            decode_binary(&[0x01, 0x02, 25, 9, 8, 7]).unwrap(),
            Incoming {
                seq: Some(0x0102),
                event: Event::DaveExternalSender(vec![9, 8, 7])
            }
        );
        assert_eq!(
            decode_binary(&[0, 5, 27, 0, 1, 2]).unwrap().event,
            Event::DaveProposals {
                operation: ProposalsOperation::Append,
                data: vec![1, 2]
            }
        );
        assert_eq!(
            decode_binary(&[0, 5, 27, 1, 3]).unwrap().event,
            Event::DaveProposals {
                operation: ProposalsOperation::Revoke,
                data: vec![3]
            }
        );
        assert_eq!(
            decode_binary(&[0xFF, 0xFF, 29, 0x01, 0x00, 4, 5]).unwrap(),
            Incoming {
                seq: Some(0xFFFF),
                event: Event::DaveAnnounceCommit {
                    transition_id: 256,
                    commit: vec![4, 5]
                }
            }
        );
        assert_eq!(
            decode_binary(&[0, 1, 30, 0, 7, 6]).unwrap().event,
            Event::DaveWelcome {
                transition_id: 7,
                welcome: vec![6]
            }
        );
        assert_eq!(decode_binary(&[0, 1, 99]).unwrap().event, Event::Other(99));
    }

    #[test]
    fn malformed_binary_messages_fail() {
        for (bytes, op) in [
            (&[][..], None),
            (&[0, 1][..], None),
            (&[0, 1, 27][..], Some(27)),
            (&[0, 1, 27, 2, 1][..], Some(27)),
            (&[0, 1, 29, 0][..], Some(29)),
            (&[0, 1, 30][..], Some(30)),
        ] {
            assert!(
                matches!(decode_binary(bytes), Err(DecodeError::Binary { op: found }) if found == op),
                "{bytes:?}"
            );
        }
    }

    #[test]
    fn close_codes_decide_what_follows() {
        assert_eq!(after_close(None), AfterClose::Resume);
        assert_eq!(after_close(Some(1006)), AfterClose::Resume);
        assert_eq!(after_close(Some(4015)), AfterClose::Resume);
        assert_eq!(after_close(Some(4006)), AfterClose::Identify);
        assert_eq!(after_close(Some(4009)), AfterClose::Identify);
        for code in [4004, 4014, 4017, 4021, 4022, 4001, 4020] {
            assert_eq!(after_close(Some(code)), AfterClose::Stop, "{code}");
        }
    }

    #[test]
    fn session_descriptions_default_to_no_dave() {
        let key = [0_u8; 32];
        let description: SessionDescription = serde_json::from_value(json!({
            "mode": "aead_xchacha20_poly1305_rtpsize", "secret_key": key
        }))
        .unwrap();
        assert_eq!(description.dave_protocol_version, 0);
    }
}
