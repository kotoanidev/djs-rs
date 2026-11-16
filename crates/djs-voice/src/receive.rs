//! Receiving audio (step 3, D-363–D-373): the public [`VoiceStream`] of one user's packets
//! and its options, and [`Receiving`], what a call's task knows of who sends what.
//!
//! The call's task reads every RTP packet of the session ([`Receiver`](crate::connection::Receiver)):
//! its header says whose it is (the SSRCs the voice gateway reports in Opcode 5), which is
//! enough for [`CallEvent::SpeakingStarted`](crate::CallEvent::SpeakingStarted) and
//! [`SpeakingStopped`](crate::CallEvent::SpeakingStopped). Only the packets of users someone
//! subscribed to are decrypted, put in order by a [`Reorder`] buffer per user (D-366), then
//! copied to each of their subscriptions (D-373). A stream that asked for PCM (D-364)
//! decodes its packets itself, when it is read (D-374): the call's task only routes Opus, so
//! a call with many speakers keeps up with its socket and its own audio, and the decoding
//! is spread over the readers' tasks.

use core::{
    fmt,
    pin::Pin,
    task::{Context, Poll, ready},
    time::Duration,
};
use std::collections::{HashMap, VecDeque};

use djs_model::id::UserId;
use futures_util::Stream;
use tokio::{
    sync::mpsc::{self, error::TrySendError},
    time::Instant,
};

use crate::{codec::Decoder, opus, udp::Rtp};

/// Opus's frame of silence, which Discord clients send after speaking.
const SILENCE: [u8; 3] = [0xF8, 0xFF, 0xFE];
/// How long a missing packet is waited for (D-366).
const REORDER: Duration = Duration::from_millis(60);
/// How long after their last packet a user stops speaking (D-369, as discord.js).
const SPEAKING: Duration = Duration::from_millis(100);
/// A jump of the sequence numbers further than this (a second of 20 ms packets) is a new
/// stream, not lost packets.
const JUMP: u16 = 50;
/// How many packets a [`VoiceStream`] holds for a reader that is behind: about 5 s of
/// 20 ms packets; more are dropped.
pub(crate) const QUEUE: usize = 256;
/// The most samples per channel an Opus packet holds: 120 ms.
const MAX_SAMPLES: usize = 5760;
/// The duration assumed for a packet whose own cannot be read: 20 ms.
const FRAME: u32 = 960;

/// When a [`VoiceStream`] ends by itself (D-365); it always ends with its call.
///
/// discord.js counterpart: `EndBehaviorType` and the `end` option of
/// `VoiceReceiver#subscribe`.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use djs_voice::{EndBehavior, ReceiveOptions};
///
/// // One stream per utterance: it ends a second after the user stops speaking.
/// let options = ReceiveOptions::new().end(EndBehavior::AfterSilence(Duration::from_secs(1)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EndBehavior {
    /// Only when the stream is dropped or the call ends.
    #[default]
    Manual,
    /// When this long has passed since the user's last packet of audio other than silence
    /// (Discord clients send a few frames of silence when they stop speaking). The time
    /// starts at their first packet: a stream of someone who never speaks does not end.
    AfterSilence(Duration),
    /// When this long has passed since the user's last packet, silence or not; from their
    /// first packet too.
    AfterInactivity(Duration),
}

/// How [`Call::subscribe_with`](crate::Call::subscribe_with) receives a user's audio (D-372):
/// the packets as Opus or decoded to PCM (D-364), and when the stream ends (D-365). The
/// default is Opus, ended manually, as discord.js's `VoiceReceiver#subscribe`.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use djs_voice::{EndBehavior, ReceiveOptions};
///
/// // Decoded audio, until a user has said nothing for half a second.
/// let options = ReceiveOptions::new()
///     .decode(true)
///     .end(EndBehavior::AfterSilence(Duration::from_millis(500)));
/// assert_ne!(options, ReceiveOptions::default());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ReceiveOptions {
    decode: bool,
    end: EndBehavior,
}

impl ReceiveOptions {
    /// The default options: Opus packets, [`EndBehavior::Manual`].
    ///
    /// discord.js counterpart: `AudioReceiveStreamOptions` (`@discordjs/voice`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            decode: false,
            end: EndBehavior::Manual,
        }
    }

    /// Whether the packets are decoded (D-364, D-370): [`VoiceData::Pcm`] at 48 kHz, stereo,
    /// interleaved `i16` samples, lost packets concealed by Opus (rebuilt from the next
    /// packet's forward error correction when it has some). Otherwise [`VoiceData::Opus`],
    /// as they came, and [`VoiceData::Lost`] for the lost ones.
    ///
    /// discord.js counterpart: none (`@discordjs/voice` hands Opus packets only).
    #[must_use]
    pub const fn decode(mut self, decode: bool) -> Self {
        self.decode = decode;
        self
    }

    /// When the stream ends by itself (D-365).
    ///
    /// discord.js counterpart: the `end` of `AudioReceiveStreamOptions` (`@discordjs/voice`).
    #[must_use]
    pub const fn end(mut self, end: EndBehavior) -> Self {
        self.end = end;
        self
    }
}

/// A packet of a user's audio, from a [`VoiceStream`] (D-368), in the order the user sent
/// them: each one follows the one before, by [`sequence`](Self::sequence).
///
/// discord.js counterpart: the Opus packets an `AudioReceiveStream` yields.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct VoicePacket {
    /// The RTP sequence number, one more than the packet before (wrapping after 65 535).
    pub sequence: u16,
    /// The RTP timestamp: when the audio starts, in samples per channel at 48 kHz from a
    /// start of the sender's choosing (wrapping after `u32::MAX`). Packets that follow one
    /// another without a pause are 960 apart (20 ms); a pause in speaking shows as a larger
    /// step. A lost packet's timestamp is an estimate.
    pub timestamp: u32,
    /// The audio.
    pub audio: VoiceData,
}

/// The audio of a [`VoicePacket`] (D-368).
///
/// discord.js counterpart: none (`@discordjs/voice` receives Opus only).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VoiceData {
    /// An Opus packet as the user sent it (decrypted): stereo, 48 kHz, usually 20 ms.
    /// `[0xF8, 0xFF, 0xFE]` is a frame of silence, which clients send when they stop.
    Opus(Vec<u8>),
    /// Decoded audio (D-370): 48 kHz, stereo, interleaved `i16` samples (left, right, left,
    /// …), 1 920 of them for 20 ms. A lost packet comes as this too, concealed.
    Pcm(Vec<i16>),
    /// A packet that did not come in time (D-366), in a stream of Opus packets. Only
    /// packets missing between two that came are reported, not the pauses in speaking.
    Lost,
}

/// The packets of one user's audio, from [`Call::subscribe`](crate::Call::subscribe) (D-371).
/// It ends with its call, or as its [`EndBehavior`] says; dropping it unsubscribes.
///
/// A reader that falls more than about 5 s behind loses the packets that do not fit.
/// A stream of decoded audio ([`ReceiveOptions::decode`]) decodes each packet when it is
/// read, on the task that reads it (D-374): about 40 µs of that task's time per 20 ms
/// packet.
///
/// Implements [`Stream`]; [`next`](Self::next) is provided so no extension trait is needed.
///
/// discord.js counterpart: `AudioReceiveStream`.
pub struct VoiceStream {
    rx: mpsc::Receiver<Item>,
    /// For PCM; boxed, as most streams are of Opus.
    decoding: Option<Box<Decoding>>,
}

impl VoiceStream {
    /// The stream of what `rx` receives, decoded if `options` say so. A stream that cannot
    /// decode (libopus cannot make a decoder, which it always can) ends at once.
    pub(crate) fn new(mut rx: mpsc::Receiver<Item>, options: ReceiveOptions) -> Self {
        let decoding = if options.decode {
            let decoding = Decoder::new().map(|decoder| {
                Box::new(Decoding {
                    decoder,
                    pcm: vec![0; MAX_SAMPLES * 2],
                    ahead: None,
                    lost: None,
                })
            });
            if decoding.is_none() {
                rx.close();
            }
            decoding
        } else {
            None
        };
        Self { rx, decoding }
    }

    /// Waits for the next packet; `None` once the stream has ended. Cancel safe: a packet
    /// is never lost by dropping the future before it is ready.
    ///
    /// discord.js counterpart: the `data` event of `AudioReceiveStream` (`@discordjs/voice`).
    pub async fn next(&mut self) -> Option<VoicePacket> {
        core::future::poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// The next packet if one is there, without waiting.
    #[cfg(test)]
    pub(crate) fn try_next(&mut self) -> Result<VoicePacket, mpsc::error::TryRecvError> {
        let mut cx = Context::from_waker(std::task::Waker::noop());
        match Pin::new(self).poll_next(&mut cx) {
            Poll::Ready(Some(packet)) => Ok(packet),
            Poll::Ready(None) => Err(mpsc::error::TryRecvError::Disconnected),
            Poll::Pending => Err(mpsc::error::TryRecvError::Empty),
        }
    }
}

impl fmt::Debug for VoiceStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceStream")
            .field("decode", &self.decoding.is_some())
            .finish_non_exhaustive()
    }
}

impl Stream for VoiceStream {
    type Item = VoicePacket;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<VoicePacket>> {
        let Self { rx, decoding } = self.get_mut();
        let Some(decoding) = decoding else {
            return rx.poll_recv(cx).map(|item| {
                item.map(|item| match item {
                    Item::Packet {
                        sequence,
                        timestamp,
                        opus,
                    } => VoicePacket {
                        sequence,
                        timestamp,
                        audio: VoiceData::Opus(opus),
                    },
                    Item::Lost {
                        sequence,
                        timestamp,
                        ..
                    } => VoicePacket {
                        sequence,
                        timestamp,
                        audio: VoiceData::Lost,
                    },
                })
            });
        };
        decoding.poll(rx, cx)
    }
}

/// A [`VoiceStream`]'s decoding (D-364, D-374).
struct Decoding {
    decoder: Decoder,
    /// Room for a packet's samples.
    pcm: Vec<i16>,
    /// The item after a lost packet, read to rebuild that one: given next.
    ahead: Option<Item>,
    /// A lost packet waiting for the item after it (sequence, timestamp, samples).
    lost: Option<(u16, u32, u32)>,
}

impl Decoding {
    fn poll(
        &mut self,
        rx: &mut mpsc::Receiver<Item>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<VoicePacket>> {
        loop {
            if let Some((sequence, timestamp, samples)) = self.lost {
                // Every gap given up is followed by a packet that came, sent with it: the
                // packet after a lost one may carry it (forward error correction).
                let after = ready!(rx.poll_recv(cx));
                self.lost = None;
                let audio = match &after {
                    Some(Item::Packet {
                        sequence: next,
                        opus,
                        ..
                    }) if *next == sequence.wrapping_add(1) => {
                        decode(&mut self.decoder, &mut self.pcm, opus, true, samples)
                    }
                    _ => decode(&mut self.decoder, &mut self.pcm, &[], false, samples),
                };
                self.ahead = after;
                return Poll::Ready(Some(VoicePacket {
                    sequence,
                    timestamp,
                    audio,
                }));
            }
            let item = match self.ahead.take() {
                Some(item) => item,
                None => match ready!(rx.poll_recv(cx)) {
                    Some(item) => item,
                    None => return Poll::Ready(None),
                },
            };
            match item {
                Item::Packet {
                    sequence,
                    timestamp,
                    opus,
                } => {
                    let audio = decode(&mut self.decoder, &mut self.pcm, &opus, false, FRAME);
                    return Poll::Ready(Some(VoicePacket {
                        sequence,
                        timestamp,
                        audio,
                    }));
                }
                Item::Lost {
                    sequence,
                    timestamp,
                    samples,
                } => self.lost = Some((sequence, timestamp, samples)),
            }
        }
    }
}

/// A packet put in order, or one given up: what a call's task sends a [`VoiceStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    Packet {
        sequence: u16,
        timestamp: u32,
        opus: Vec<u8>,
    },
    Lost {
        sequence: u16,
        timestamp: u32,
        /// How long it is assumed to be: as the packet before.
        samples: u32,
    },
}

/// A packet waiting for a missing one before it.
#[derive(Debug)]
struct Held {
    timestamp: u32,
    opus: Vec<u8>,
}

/// Puts one sender's packets in order (D-366). A packet that follows the one before is
/// passed on at once; after a gap, the packets are held until the missing ones come, for
/// at most [`REORDER`] after the first held one came. Then every gap is given up as lost.
/// Packets older than the one passed on last are dropped (late, or duplicates).
#[derive(Debug, Default)]
struct Reorder {
    /// The sequence number passed on next; `None` before the first packet.
    next: Option<u16>,
    /// The timestamp and duration of the last packet passed on.
    timestamp: u32,
    samples: u32,
    /// `held[i]` is the packet `next + i`, `None` while missing; empty without a gap.
    held: VecDeque<Option<Held>>,
    /// When the first packet after a gap came.
    since: Option<Instant>,
}

impl Reorder {
    fn push(
        &mut self,
        sequence: u16,
        timestamp: u32,
        opus: &[u8],
        now: Instant,
        out: &mut Vec<Item>,
    ) {
        let Some(next) = self.next else {
            self.pass(sequence, timestamp, opus.to_vec(), out);
            return;
        };
        let ahead = sequence.wrapping_sub(next);
        let behind = next.wrapping_sub(sequence);
        if ahead > JUMP {
            if behind != 0 && behind <= JUMP {
                // Late (given up already) or a duplicate.
                return;
            }
            // A new stream: what is held goes first.
            self.flush(out);
            self.pass(sequence, timestamp, opus.to_vec(), out);
            return;
        }
        let index = usize::from(ahead);
        if index == 0 && self.held.is_empty() {
            self.pass(sequence, timestamp, opus.to_vec(), out);
            return;
        }
        if self.held.len() <= index {
            self.held.resize_with(index + 1, || None);
        }
        if self.held[index].is_some() {
            return;
        }
        self.held[index] = Some(Held {
            timestamp,
            opus: opus.to_vec(),
        });
        self.since.get_or_insert(now);
        self.drain(out);
    }

    /// Passes on the held packets that follow the last one passed on.
    fn drain(&mut self, out: &mut Vec<Item>) {
        while let Some(Some(_)) = self.held.front() {
            let Some(Some(held)) = self.held.pop_front() else {
                break;
            };
            let sequence = self.next.unwrap_or_default();
            self.pass(sequence, held.timestamp, held.opus, out);
        }
        if self.held.is_empty() {
            self.since = None;
        }
    }

    /// Gives up the missing packets: passes on everything held, in order.
    fn flush(&mut self, out: &mut Vec<Item>) {
        while let Some(slot) = self.held.pop_front() {
            let sequence = self.next.unwrap_or_default();
            if let Some(held) = slot {
                self.pass(sequence, held.timestamp, held.opus, out);
            } else {
                let timestamp = self.timestamp.wrapping_add(self.samples);
                out.push(Item::Lost {
                    sequence,
                    timestamp,
                    samples: self.samples,
                });
                self.timestamp = timestamp;
                self.next = Some(sequence.wrapping_add(1));
            }
        }
        self.since = None;
    }

    fn pass(&mut self, sequence: u16, timestamp: u32, opus: Vec<u8>, out: &mut Vec<Item>) {
        self.next = Some(sequence.wrapping_add(1));
        self.timestamp = timestamp;
        self.samples = opus::packet_samples(&opus).unwrap_or(FRAME);
        out.push(Item::Packet {
            sequence,
            timestamp,
            opus,
        });
    }

    /// When the missing packets are given up.
    fn deadline(&self) -> Option<Instant> {
        self.since.map(|since| since + REORDER)
    }
}

/// One [`VoiceStream`], as the call's task holds it.
struct Subscription {
    tx: mpsc::Sender<Item>,
    end: EndBehavior,
    /// When it ends ([`EndBehavior`]), once a packet came.
    end_at: Option<Instant>,
}

impl Subscription {
    /// Sends `item`; whether the stream is still there.
    fn send(&mut self, item: &Item, now: Instant) -> bool {
        if let Item::Packet { opus, .. } = item {
            match self.end {
                EndBehavior::Manual => {}
                EndBehavior::AfterSilence(wait) => {
                    if opus[..] != SILENCE || self.end_at.is_none() {
                        self.end_at = Some(now + wait);
                    }
                }
                EndBehavior::AfterInactivity(wait) => self.end_at = Some(now + wait),
            }
        }
        match self.tx.try_send(item.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                tracing::trace!("dropping a received packet: its stream's reader is behind");
                true
            }
            Err(TrySendError::Closed(_)) => false,
        }
    }

    fn ended(&self, now: Instant) -> bool {
        self.end_at.is_some_and(|at| at <= now) || self.tx.is_closed()
    }
}

/// Decodes `packet` (`fec`: the lost one before it from its forward error correction, or
/// empty: concealed) of `samples` (when lost); a packet that does not decode is concealed.
fn decode(
    decoder: &mut Decoder,
    pcm: &mut [i16],
    packet: &[u8],
    fec: bool,
    samples: u32,
) -> VoiceData {
    let lost = usize::try_from(samples).map_or(MAX_SAMPLES, |samples| samples.min(MAX_SAMPLES));
    let written = if packet.is_empty() || fec {
        decoder.decode_i16(packet, fec, &mut pcm[..lost * 2])
    } else {
        decoder
            .decode_i16(packet, false, pcm)
            .or_else(|| decoder.decode_i16(&[], false, &mut pcm[..lost * 2]))
    };
    match written {
        Some(samples) => VoiceData::Pcm(pcm[..samples * 2].to_vec()),
        None => VoiceData::Lost,
    }
}

/// A user someone receives, or who speaks.
#[derive(Default)]
struct User {
    /// When their last packet came: `Some` while they speak (D-369).
    last: Option<Instant>,
    order: Reorder,
    subscriptions: Vec<Subscription>,
}

impl User {
    /// Sends `items` to every subscription; the closed ones are removed.
    fn send(&mut self, items: &[Item], now: Instant) {
        for item in items {
            self.subscriptions
                .retain_mut(|subscription| subscription.send(item, now));
        }
        if self.subscriptions.is_empty() {
            self.order = Reorder::default();
        }
    }
}

/// A packet of a known user, from [`Receiving::arrived`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Arrival {
    pub(crate) user_id: UserId,
    /// Their first packet in a while: they started speaking.
    pub(crate) started: bool,
    /// Someone receives them: open the packet and give it to [`Receiving::opened`].
    pub(crate) wanted: bool,
}

/// What a call's task knows of the audio it receives: whose each SSRC is, who speaks, and
/// the subscriptions.
#[derive(Default)]
pub(crate) struct Receiving {
    /// The users of the SSRCs the voice gateway reported (Opcode 5).
    ssrcs: HashMap<u32, UserId>,
    users: HashMap<UserId, User>,
    /// The items passed on, reused.
    items: Vec<Item>,
}

impl Receiving {
    /// Adds a subscription to `user_id`'s audio.
    pub(crate) fn subscribe(
        &mut self,
        user_id: UserId,
        options: ReceiveOptions,
        tx: mpsc::Sender<Item>,
    ) {
        self.users
            .entry(user_id)
            .or_default()
            .subscriptions
            .push(Subscription {
                tx,
                end: options.end,
                end_at: None,
            });
    }

    /// The voice gateway reported that `user_id` sends with `ssrc` (Opcode 5).
    pub(crate) fn map(&mut self, user_id: UserId, ssrc: u32, now: Instant) {
        if self.ssrcs.get(&ssrc) == Some(&user_id) {
            return;
        }
        let had = self.ssrcs.values().any(|user| *user == user_id);
        self.ssrcs.retain(|_, user| *user != user_id);
        self.ssrcs.insert(ssrc, user_id);
        if had && let Some(user) = self.users.get_mut(&user_id) {
            // Their new source numbers its packets anew.
            let mut items = core::mem::take(&mut self.items);
            user.order.flush(&mut items);
            user.send(&items, now);
            user.order = Reorder::default();
            items.clear();
            self.items = items;
        }
    }

    /// `user_id` left the channel: their packets are not expected any more.
    pub(crate) fn left(&mut self, user_id: UserId) {
        self.ssrcs.retain(|_, user| *user != user_id);
    }

    /// A new connection (another channel's voice server): other users, other SSRCs. Returns
    /// the users who were speaking.
    pub(crate) fn reset(&mut self) -> Vec<UserId> {
        self.ssrcs.clear();
        let mut stopped = Vec::new();
        for (user_id, user) in &mut self.users {
            if user.last.take().is_some() {
                stopped.push(*user_id);
            }
            user.order = Reorder::default();
        }
        self.users.retain(|_, user| !user.subscriptions.is_empty());
        stopped
    }

    /// The packet `rtp` came: whose it is, if the voice gateway said.
    pub(crate) fn arrived(&mut self, rtp: Rtp, now: Instant) -> Option<Arrival> {
        let user_id = *self.ssrcs.get(&rtp.ssrc)?;
        let user = self.users.entry(user_id).or_default();
        let started = user.last.replace(now).is_none();
        Some(Arrival {
            user_id,
            started,
            wanted: !user.subscriptions.is_empty(),
        })
    }

    /// The Opus packet of `rtp`, from `user_id`, opened: put in order and sent on.
    pub(crate) fn opened(&mut self, user_id: UserId, rtp: Rtp, opus: &[u8], now: Instant) {
        let Some(user) = self.users.get_mut(&user_id) else {
            return;
        };
        let mut items = core::mem::take(&mut self.items);
        user.order
            .push(rtp.sequence, rtp.timestamp, opus, now, &mut items);
        user.send(&items, now);
        items.clear();
        self.items = items;
    }

    /// When [`tick`](Self::tick) has something to do.
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.users
            .values()
            .flat_map(|user| {
                let ends = user
                    .subscriptions
                    .iter()
                    .filter_map(|subscription| subscription.end_at)
                    .min();
                [
                    user.last.map(|last| last + SPEAKING),
                    user.order.deadline(),
                    ends,
                ]
            })
            .flatten()
            .min()
    }

    /// Does what is due at `now`: users stop speaking (returned), missing packets are given
    /// up, streams end.
    pub(crate) fn tick(&mut self, now: Instant) -> Vec<UserId> {
        let mut stopped = Vec::new();
        let mut items = core::mem::take(&mut self.items);
        for (user_id, user) in &mut self.users {
            if user.last.is_some_and(|last| last + SPEAKING <= now) {
                user.last = None;
                stopped.push(*user_id);
            }
            if user.order.deadline().is_some_and(|at| at <= now) {
                user.order.flush(&mut items);
                user.send(&items, now);
                items.clear();
            }
            // Dropping a subscription ends its stream, after the packets it holds.
            user.subscriptions
                .retain(|subscription| !subscription.ended(now));
            if user.subscriptions.is_empty() {
                user.order = Reorder::default();
            }
        }
        self.items = items;
        self.users
            .retain(|_, user| user.last.is_some() || !user.subscriptions.is_empty());
        stopped
    }
}

#[cfg(test)]
mod tests;
