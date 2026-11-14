//! DAVE, Discord's end-to-end encryption of calls (D-312, step 1d): the MLS group and the
//! media keys through `davey`, driven by the voice gateway's DAVE opcodes (21 to 31).
//!
//! The flow follows Discord's documentation and the sample session manager of `libdave`:
//! a session is created with the protocol version of Session Description (or of Prepare
//! Epoch 1) and sends a key package; the gateway's external sender proposes members, whose
//! proposals are committed; a commit or a welcome prepares a transition (acknowledged with
//! Opcode 23) that the gateway then executes, which switches the media key. A commit or a
//! welcome that cannot be processed is reported (Opcode 31) and the session starts over
//! with a new key package. Users the gateway says are in the call (Opcodes 11 and 13), and
//! the bot itself, are the only members proposals and welcomes may add.
//!
//! Discord documents DAVE in "End-to-End Encryption (DAVE Protocol)" of
//! <https://discord.com/developers/docs/topics/voice-connections>, and in its whitepaper,
//! <https://daveprotocol.com>.

#![allow(dead_code, reason = "used by `Call` from step 1e")]

use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU16,
};

use davey::{Codec, DaveSession, MediaType, ProposalsOperationType};

use crate::payload::{self, Event, ProposalsOperation};

/// How long frames that are not end-to-end encrypted are still accepted after a call
/// upgrades to DAVE, in seconds (for receiving).
const UPGRADE_PASSTHROUGH: u32 = 10;

/// A message the connection sends for DAVE.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reply {
    Text(String),
    Binary(Vec<u8>),
}

/// What [`Dave::handle`] did with an event.
#[derive(Debug)]
pub(crate) enum Handled {
    /// A DAVE event, handled: the replies to send.
    Replies(Vec<Reply>),
    /// Not (only) a DAVE event: the call gets it.
    Forward(Event),
}

/// The DAVE state of a call.
pub(crate) struct Dave {
    user_id: u64,
    channel_id: u64,
    /// The protocol version media is sent with now; 0: transport encryption only.
    version: u16,
    session: Option<DaveSession>,
    /// The external sender package, kept for a session created after it arrived.
    external_sender: Option<Vec<u8>>,
    /// The users in the call, and the bot: the only members proposals and welcomes may add.
    recognized: HashSet<u64>,
    /// Transitions prepared and not executed yet, by ID: their protocol version.
    transitions: HashMap<u16, u16>,
    /// The frame being encrypted, reused.
    frame: Vec<u8>,
}

impl core::fmt::Debug for Dave {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Dave")
            .field("user_id", &self.user_id)
            .field("channel_id", &self.channel_id)
            .field("version", &self.version)
            .field("session", &self.session.as_ref().map(DaveSession::status))
            .field("recognized", &self.recognized.len())
            .field("transitions", &self.transitions)
            .finish_non_exhaustive()
    }
}

impl Dave {
    /// The DAVE state of the bot `user_id` in the voice channel `channel_id`.
    pub(crate) fn new(user_id: u64, channel_id: u64) -> Self {
        Self {
            user_id,
            channel_id,
            version: 0,
            session: None,
            external_sender: None,
            recognized: HashSet::from([user_id]),
            transitions: HashMap::new(),
            frame: Vec::new(),
        }
    }

    /// Starts a voice session (a new one after identifying anew, too) with the DAVE version of
    /// its Session Description; returns the key package to send if DAVE is on.
    ///
    /// Nothing of an earlier voice session is kept: the gateway sends its external sender and
    /// the users in the call again, and the bot joins the group anew, with a new identity key
    /// (D-333). Media waits for the new group's key ([`Dave::ready`]).
    pub(crate) fn start(&mut self, version: u16) -> Vec<Reply> {
        self.version = version;
        self.session = None;
        self.external_sender = None;
        self.recognized.clear();
        self.recognized.insert(self.user_id);
        self.transitions.clear();
        NonZeroU16::new(version).map_or_else(Vec::new, |version| self.init(version))
    }

    /// Whether media can be sent: without DAVE, or with a key to encrypt with.
    pub(crate) fn ready(&self) -> bool {
        self.version == 0 || self.session.as_ref().is_some_and(DaveSession::can_encrypt)
    }

    /// The protocol version media is sent with now.
    pub(crate) const fn version(&self) -> u16 {
        self.version
    }

    /// The frame to send for the Opus packet `opus`: the packet itself without DAVE, else its
    /// end-to-end encrypted frame; `None` while DAVE has no key yet (send nothing).
    pub(crate) fn encrypt<'a>(&'a mut self, opus: &'a [u8]) -> Option<&'a [u8]> {
        if self.version == 0 {
            return Some(opus);
        }
        let session = self.session.as_mut()?;
        match session.encrypt_into(MediaType::AUDIO, Codec::OPUS, opus, &mut self.frame) {
            Ok(()) => Some(&self.frame),
            Err(error) => {
                tracing::trace!(%error, "not sending a frame DAVE cannot encrypt");
                None
            }
        }
    }

    /// Handles a voice gateway event: DAVE's are handled here (their replies returned), the
    /// users joining and leaving are noted and forwarded, the rest forwarded.
    pub(crate) fn handle(&mut self, event: Event) -> Handled {
        let replies = match event {
            Event::ClientsConnect(users) => {
                self.recognized.extend(users.iter().map(|user| user.get()));
                return Handled::Forward(Event::ClientsConnect(users));
            }
            Event::ClientDisconnect(user) => {
                if user.get() != self.user_id {
                    self.recognized.remove(&user.get());
                }
                return Handled::Forward(Event::ClientDisconnect(user));
            }
            Event::DavePrepareTransition {
                transition_id,
                protocol_version,
            } => {
                if protocol_version == 0
                    && let Some(session) = &mut self.session
                {
                    // Accept the frames of members who stop encrypting first.
                    session.set_passthrough_mode(true, None);
                }
                self.prepare(transition_id, protocol_version)
            }
            Event::DaveExecuteTransition { transition_id } => {
                self.execute(transition_id);
                Vec::new()
            }
            Event::DavePrepareEpoch {
                epoch,
                protocol_version,
            } => match NonZeroU16::new(protocol_version) {
                // Epoch 1 is a new group; later epochs come with their commits.
                Some(version) if epoch == 1 => self.init(version),
                _ => Vec::new(),
            },
            Event::DaveExternalSender(package) => {
                if let Some(session) = &mut self.session
                    && let Err(error) = session.set_external_sender(&package)
                {
                    tracing::warn!(%error, "could not set the DAVE external sender");
                }
                self.external_sender = Some(package);
                Vec::new()
            }
            Event::DaveProposals { operation, data } => self.proposals(operation, &data),
            Event::DaveAnnounceCommit {
                transition_id,
                commit,
            } => self.commit(transition_id, &commit),
            Event::DaveWelcome {
                transition_id,
                welcome,
            } => self.welcome(transition_id, &welcome),
            other => return Handled::Forward(other),
        };
        Handled::Replies(replies)
    }

    /// A new MLS session (or the current one, reset) for `version`, and its key package.
    fn init(&mut self, version: NonZeroU16) -> Vec<Reply> {
        let session = match self.session.take() {
            Some(mut session) => match session.reinit(version, self.user_id, self.channel_id, None)
            {
                Ok(()) => session,
                Err(error) => {
                    tracing::warn!(%error, "could not reinitialize the DAVE session");
                    return Vec::new();
                }
            },
            None => match DaveSession::new(version, self.user_id, self.channel_id, None) {
                Ok(mut session) => {
                    if let Some(package) = &self.external_sender
                        && let Err(error) = session.set_external_sender(package)
                    {
                        tracing::warn!(%error, "could not set the DAVE external sender");
                    }
                    session
                }
                Err(error) => {
                    tracing::warn!(%error, "could not create a DAVE session");
                    return Vec::new();
                }
            },
        };
        let session = self.session.insert(session);
        match session.create_key_package() {
            Ok(package) => vec![Reply::Binary(payload::key_package(&package))],
            Err(error) => {
                tracing::warn!(%error, "could not create a DAVE key package");
                Vec::new()
            }
        }
    }

    fn recognized(&self) -> Vec<u64> {
        self.recognized.iter().copied().collect()
    }

    fn proposals(&mut self, operation: ProposalsOperation, data: &[u8]) -> Vec<Reply> {
        let recognized = self.recognized();
        let Some(session) = &mut self.session else {
            return Vec::new();
        };
        let operation = match operation {
            ProposalsOperation::Append => ProposalsOperationType::APPEND,
            ProposalsOperation::Revoke => ProposalsOperationType::REVOKE,
        };
        match session.process_proposals(operation, data, Some(&recognized)) {
            Ok(Some(commit_welcome)) => vec![Reply::Binary(payload::commit_welcome(
                &commit_welcome.commit,
                commit_welcome.welcome.as_deref(),
            ))],
            Ok(None) => Vec::new(),
            Err(error) => {
                tracing::warn!(%error, "could not process DAVE proposals");
                Vec::new()
            }
        }
    }

    fn commit(&mut self, transition_id: u16, commit: &[u8]) -> Vec<Reply> {
        let Some(session) = &mut self.session else {
            return Vec::new();
        };
        match session.process_commit(commit) {
            Ok(true) => {
                let version = session.protocol_version().get();
                self.prepare(transition_id, version)
            }
            Ok(false) => Vec::new(),
            Err(error) => {
                tracing::warn!(%error, "a DAVE commit could not be processed");
                self.invalid(transition_id)
            }
        }
    }

    fn welcome(&mut self, transition_id: u16, welcome: &[u8]) -> Vec<Reply> {
        let recognized = self.recognized();
        let Some(session) = &mut self.session else {
            return Vec::new();
        };
        match session.process_welcome(welcome, Some(&recognized)) {
            Ok(()) => {
                let version = session.protocol_version().get();
                self.prepare(transition_id, version)
            }
            Err(error) => {
                tracing::warn!(%error, "a DAVE welcome could not be processed");
                self.invalid(transition_id)
            }
        }
    }

    /// Opcode 31, then a new start: the gateway removes and re-adds the bot.
    fn invalid(&mut self, transition_id: u16) -> Vec<Reply> {
        let mut replies = vec![Reply::Text(payload::invalid_commit_welcome(transition_id))];
        if let Some(version) = self.session.as_ref().map(DaveSession::protocol_version) {
            replies.extend(self.init(version));
        }
        replies
    }

    /// Prepares a transition; the one with ID 0 happens at once, the others are acknowledged
    /// (Opcode 23) and wait for their execution.
    fn prepare(&mut self, transition_id: u16, version: u16) -> Vec<Reply> {
        if let Some(session) = &mut self.session
            && let Err(error) = session.prepare_transition(transition_id, version)
        {
            tracing::warn!(%error, "could not prepare a DAVE transition");
        }
        if transition_id == 0 {
            self.switch(version);
            Vec::new()
        } else {
            self.transitions.insert(transition_id, version);
            vec![Reply::Text(payload::transition_ready(transition_id))]
        }
    }

    fn execute(&mut self, transition_id: u16) {
        if let Some(session) = &mut self.session
            && let Err(error) = session.execute_transition(transition_id)
        {
            tracing::warn!(%error, "could not execute a DAVE transition");
        }
        if let Some(version) = self.transitions.remove(&transition_id) {
            self.switch(version);
        }
    }

    /// Media is sent with `version` from now on.
    fn switch(&mut self, version: u16) {
        let upgraded = self.version == 0 && version != 0;
        self.version = version;
        if upgraded && let Some(session) = &mut self.session {
            // Frames of members still without DAVE stay accepted for a while.
            session.set_passthrough_mode(false, Some(UPGRADE_PASSTHROUGH));
        }
    }
}

#[cfg(test)]
pub(crate) mod mls;

#[cfg(test)]
mod tests {
    use djs_model::id::UserId;

    use super::{
        Dave, Handled, Reply,
        mls::{self, ExternalSender},
    };
    use crate::payload::{Event, ProposalsOperation};

    const BOT: u64 = 1_234_123_412_341_234;
    const OTHER: u64 = 5_678_567_856_785_678;
    const CHANNEL: u64 = 42;
    /// No replies.
    const NONE: [Reply; 0] = [];

    fn replies(handled: Handled) -> Vec<Reply> {
        match handled {
            Handled::Replies(replies) => replies,
            Handled::Forward(event) => panic!("{event:?} was forwarded"),
        }
    }

    /// The key package in a reply.
    fn key_package(replies: &[Reply]) -> Vec<u8> {
        match replies {
            [Reply::Binary(message)] if message.first() == Some(&26) => message[1..].to_vec(),
            other => panic!("not a key package: {other:?}"),
        }
    }

    #[test]
    fn without_dave_frames_go_out_as_they_are() {
        let mut dave = Dave::new(BOT, CHANNEL);
        assert_eq!(dave.start(0), NONE);
        assert!(dave.ready());
        assert_eq!(dave.encrypt(b"opus"), Some(&b"opus"[..]));
    }

    #[test]
    fn the_bot_joins_through_a_welcome_and_its_frames_decrypt() {
        let sender = ExternalSender::new(CHANNEL);
        let mut dave = Dave::new(BOT, CHANNEL);
        let package = key_package(&dave.start(1));
        assert!(!dave.ready(), "no key before the group");
        assert_eq!(dave.encrypt(b"opus"), None);
        assert_eq!(
            replies(dave.handle(Event::DaveExternalSender(sender.package()))),
            NONE
        );
        // Another member creates the group with the bot in it.
        let mut other = sender.member(OTHER);
        let welcome = sender.create_group(&mut other, &package, &[BOT, OTHER]);
        let Handled::Forward(_) = dave.handle(Event::ClientsConnect(vec![UserId::new(OTHER)]))
        else {
            panic!("users joining are for the call too");
        };
        assert_eq!(
            replies(dave.handle(Event::DaveWelcome {
                transition_id: 3,
                welcome
            })),
            [Reply::Text(r#"{"op":23,"d":{"transition_id":3}}"#.into())]
        );
        assert!(!dave.ready(), "not before the transition is executed");
        assert_eq!(
            replies(dave.handle(Event::DaveExecuteTransition { transition_id: 3 })),
            NONE
        );
        other.prepare_transition(3, 1).unwrap();
        other.execute_transition(3).unwrap();
        assert!(dave.ready());
        assert_eq!(dave.version(), 1);
        let frame = dave.encrypt(b"an opus frame").unwrap().to_vec();
        assert_ne!(frame, b"an opus frame");
        let decrypted = other.decrypt(BOT, davey::MediaType::AUDIO, &frame).unwrap();
        assert_eq!(decrypted, b"an opus frame");
    }

    #[test]
    fn the_bot_creates_the_group_by_committing_proposals() {
        let sender = ExternalSender::new(CHANNEL);
        let mut dave = Dave::new(BOT, CHANNEL);
        key_package(&dave.start(1));
        dave.handle(Event::DaveExternalSender(sender.package()));
        dave.handle(Event::ClientsConnect(vec![UserId::new(OTHER)]));
        let mut other = sender.member(OTHER);
        let other_package = other.create_key_package().unwrap();
        let proposals = sender.propose_adds(0, &[&other_package]);
        let [Reply::Binary(commit_welcome)] = &replies(dave.handle(Event::DaveProposals {
            operation: ProposalsOperation::Append,
            data: proposals,
        }))[..] else {
            panic!("no commit");
        };
        assert_eq!(commit_welcome[0], 28);
        let (commit, welcome) = mls::split_commit_welcome(&commit_welcome[1..]);
        // The gateway announces the bot's commit, and welcomes the other member.
        assert_eq!(
            replies(dave.handle(Event::DaveAnnounceCommit {
                transition_id: 1,
                commit
            })),
            [Reply::Text(r#"{"op":23,"d":{"transition_id":1}}"#.into())]
        );
        other
            .process_welcome(&welcome.unwrap(), Some(&[BOT, OTHER]))
            .unwrap();
        other.prepare_transition(1, 1).unwrap();
        dave.handle(Event::DaveExecuteTransition { transition_id: 1 });
        other.execute_transition(1).unwrap();
        let frame = dave.encrypt(b"frame").unwrap().to_vec();
        assert_eq!(
            other.decrypt(BOT, davey::MediaType::AUDIO, &frame).unwrap(),
            b"frame"
        );
    }

    #[test]
    fn unprocessable_welcomes_are_reported_and_the_bot_starts_over() {
        let sender = ExternalSender::new(CHANNEL);
        let mut dave = Dave::new(BOT, CHANNEL);
        dave.start(1);
        dave.handle(Event::DaveExternalSender(sender.package()));
        let replies = replies(dave.handle(Event::DaveWelcome {
            transition_id: 9,
            welcome: vec![1, 2, 3],
        }));
        assert_eq!(
            replies[0],
            Reply::Text(r#"{"op":31,"d":{"transition_id":9}}"#.into())
        );
        key_package(&replies[1..]);
    }

    #[test]
    fn a_welcome_with_unknown_members_is_refused() {
        let sender = ExternalSender::new(CHANNEL);
        let mut dave = Dave::new(BOT, CHANNEL);
        let package = key_package(&dave.start(1));
        dave.handle(Event::DaveExternalSender(sender.package()));
        let mut other = sender.member(OTHER);
        // OTHER never said it was in the call (no Opcode 11).
        let welcome = sender.create_group(&mut other, &package, &[BOT, OTHER]);
        let replies = replies(dave.handle(Event::DaveWelcome {
            transition_id: 2,
            welcome,
        }));
        assert_eq!(
            replies[0],
            Reply::Text(r#"{"op":31,"d":{"transition_id":2}}"#.into())
        );
    }

    #[test]
    fn a_new_epoch_one_restarts_with_a_key_package() {
        let mut dave = Dave::new(BOT, CHANNEL);
        assert_eq!(dave.start(0), NONE);
        key_package(&replies(dave.handle(Event::DavePrepareEpoch {
            epoch: 1,
            protocol_version: 1,
        })));
        let later = dave.handle(Event::DavePrepareEpoch {
            epoch: 2,
            protocol_version: 1,
        });
        assert_eq!(replies(later), NONE, "later epochs come with commits");
    }

    #[test]
    fn a_downgrade_sends_frames_as_they_are_after_its_execution() {
        let sender = ExternalSender::new(CHANNEL);
        let mut dave = Dave::new(BOT, CHANNEL);
        let package = key_package(&dave.start(1));
        dave.handle(Event::DaveExternalSender(sender.package()));
        dave.handle(Event::ClientsConnect(vec![UserId::new(OTHER)]));
        let mut other = sender.member(OTHER);
        let welcome = sender.create_group(&mut other, &package, &[BOT, OTHER]);
        dave.handle(Event::DaveWelcome {
            transition_id: 0,
            welcome,
        });
        assert!(dave.ready(), "transition 0 happens at once");
        assert_ne!(dave.encrypt(b"x").unwrap(), b"x");
        assert_eq!(
            replies(dave.handle(Event::DavePrepareTransition {
                transition_id: 5,
                protocol_version: 0
            })),
            [Reply::Text(r#"{"op":23,"d":{"transition_id":5}}"#.into())]
        );
        assert_ne!(
            dave.encrypt(b"x").unwrap(),
            b"x",
            "still encrypted until executed"
        );
        dave.handle(Event::DaveExecuteTransition { transition_id: 5 });
        assert_eq!(dave.version(), 0);
        assert_eq!(dave.encrypt(b"x"), Some(&b"x"[..]));
    }

    #[test]
    fn a_new_voice_session_starts_dave_over() {
        let sender = ExternalSender::new(CHANNEL);
        let mut dave = Dave::new(BOT, CHANNEL);
        let package = key_package(&dave.start(1));
        dave.handle(Event::DaveExternalSender(sender.package()));
        dave.handle(Event::ClientsConnect(vec![UserId::new(OTHER)]));
        let mut other = sender.member(OTHER);
        let welcome = sender.create_group(&mut other, &package, &[BOT, OTHER]);
        dave.handle(Event::DaveWelcome {
            transition_id: 0,
            welcome,
        });
        assert!(dave.ready());
        // Identified anew: the old group's key is not used, the users come again.
        let new_package = key_package(&dave.start(1));
        assert_ne!(new_package, package);
        assert!(!dave.ready());
        assert_eq!(dave.recognized.len(), 1);
        assert!(dave.external_sender.is_none());
    }

    #[test]
    fn other_events_are_forwarded() {
        let mut dave = Dave::new(BOT, CHANNEL);
        assert!(matches!(
            dave.handle(Event::Resumed),
            Handled::Forward(Event::Resumed)
        ));
        // The bot stays recognized even if the gateway says it left.
        dave.handle(Event::ClientDisconnect(UserId::new(BOT)));
        assert!(dave.recognized.contains(&BOT));
    }
}
