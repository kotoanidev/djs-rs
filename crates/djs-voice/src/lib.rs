#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod call;
mod codec;
mod connection;
mod crypto;
mod dave;
mod error;
mod gateway;
mod input;
mod manager;
mod ogg;
mod opus;
mod payload;
mod queue;
mod tls;
mod track;
mod udp;

pub use crate::{
    call::{Call, CallEvent, CallEvents, DisconnectReason},
    error::{InputError, JoinError, TrackError},
    gateway::VoiceGateway,
    input::Input,
    manager::{JoinOptions, VoiceManager},
    queue::TrackQueue,
    track::{Loop, PlayState, TrackEnd, TrackEvent, TrackEvents, TrackHandle, TrackInfo},
};
