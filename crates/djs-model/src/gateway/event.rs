//! Typed gateway events and their fast deserialization.

use core::fmt;

use compact_str::CompactString;
use serde::de::{DeserializeSeed, Deserializer, Error as DeError, IgnoredAny, MapAccess, Visitor};

use crate::gateway::{
    OpCode,
    payload::{Hello, MessageCreate, Ready},
};

/// A payload received from the gateway, decoded according to its opcode.
///
/// Decode payloads with [`GatewayEventDeserializer`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GatewayEvent {
    /// An event was dispatched (opcode 0), with its sequence number.
    Dispatch(u64, DispatchEvent),
    /// Discord requests an immediate heartbeat (opcode 1).
    Heartbeat,
    /// The client should reconnect and resume (opcode 7).
    Reconnect,
    /// The session was invalidated (opcode 9). The value says whether the session can be
    /// resumed.
    InvalidSession(bool),
    /// Sent right after connecting, with the heartbeat interval (opcode 10).
    Hello(Hello),
    /// A heartbeat was acknowledged (opcode 11).
    HeartbeatAck,
    /// A payload with an opcode this version of the library does not handle. Its data was
    /// skipped.
    Unknown(OpCode),
}

/// The event carried by a [`GatewayEvent::Dispatch`].
///
/// **Coverage:** Phase 1 decodes the events needed by the vertical slice. Every other event is
/// skipped efficiently (its data is validated but not stored) and reported as
/// [`DispatchEvent::Unknown`] with its name.
///
/// Large events are boxed to keep this enum small.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DispatchEvent {
    /// `MESSAGE_CREATE`: a message was sent.
    MessageCreate(Box<MessageCreate>),
    /// `READY`: the session was established.
    Ready(Box<Ready>),
    /// `RESUMED`: a resume completed and all missed events were replayed.
    Resumed,
    /// An event this version of the library does not decode, with its name
    /// (for example `"GUILD_CREATE"`).
    Unknown(CompactString),
}

impl DispatchEvent {
    /// Returns the event name as sent by Discord, such as `"MESSAGE_CREATE"`.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::MessageCreate(_) => "MESSAGE_CREATE",
            Self::Ready(_) => "READY",
            Self::Resumed => "RESUMED",
            Self::Unknown(name) => name,
        }
    }
}

/// A [`DeserializeSeed`] that decodes a gateway payload into a [`GatewayEvent`].
///
/// Gateway payloads are envelopes — `{"op": 0, "s": 42, "t": "MESSAGE_CREATE", "d": {…}}` — whose
/// `d` type depends on `op` and `t`. Deserializing them naively requires buffering `d` in a
/// generic value first. Instead, this type is constructed with `op`, `s` and `t` known up front
/// (usually via [`from_json`](Self::from_json), which scans only the top-level keys), so `d` is
/// decoded in a single pass directly into its concrete type, and unhandled events are skipped
/// without allocating.
///
/// It works with any serde data format; [`from_json`](Self::from_json) is a JSON-specific
/// helper.
///
/// # Examples
///
/// ```
/// use djs_model::gateway::{GatewayEvent, GatewayEventDeserializer, OpCode};
/// use serde::de::DeserializeSeed;
///
/// let json = r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":41250}}"#;
///
/// let seed = GatewayEventDeserializer::from_json(json).unwrap();
/// assert_eq!(seed.op(), OpCode::Hello);
///
/// let mut deserializer = serde_json::Deserializer::from_str(json);
/// let event = seed.deserialize(&mut deserializer).unwrap();
/// assert!(matches!(event, GatewayEvent::Hello(hello) if hello.heartbeat_interval == 41250));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GatewayEventDeserializer<'a> {
    op: OpCode,
    sequence: Option<u64>,
    event_type: Option<&'a str>,
}

impl<'a> GatewayEventDeserializer<'a> {
    /// Creates a deserializer from an already known opcode, sequence number and event name.
    ///
    /// The `sequence` is used when the payload being deserialized has no `s` field; an `s` field
    /// in the payload takes precedence.
    #[must_use]
    pub const fn new(op: OpCode, sequence: Option<u64>, event_type: Option<&'a str>) -> Self {
        Self {
            op,
            sequence,
            event_type,
        }
    }

    /// Reads `op`, `s` and `t` from the top level of a JSON gateway payload.
    ///
    /// Only the top-level object is scanned; nested values are skipped without being parsed.
    /// Discord sends `t`, `s` and `op` before `d`, in which case scanning stops before reaching
    /// `d` at all. Keys in any order are still handled correctly.
    ///
    /// Returns `None` if the input is not a JSON object with an integer `op` field, or if `t`
    /// contains escape sequences (event names never do).
    #[must_use]
    pub fn from_json(json: &'a str) -> Option<Self> {
        scan::top_level(json)
    }

    /// Returns the opcode.
    #[must_use]
    pub const fn op(&self) -> OpCode {
        self.op
    }

    /// Returns the sequence number, if known.
    #[must_use]
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns the dispatch event name, if any.
    #[must_use]
    pub const fn event_type(&self) -> Option<&'a str> {
        self.event_type
    }
}

impl<'de> DeserializeSeed<'de> for GatewayEventDeserializer<'_> {
    type Value = GatewayEvent;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<GatewayEvent, D::Error> {
        deserializer.deserialize_map(EnvelopeVisitor(self))
    }
}

struct EnvelopeVisitor<'a>(GatewayEventDeserializer<'a>);

/// What was decoded from the `d` field.
enum Data {
    Dispatch(DispatchEvent),
    Event(GatewayEvent),
}

impl EnvelopeVisitor<'_> {
    fn data<'de, A: MapAccess<'de>>(&self, map: &mut A) -> Result<Data, A::Error> {
        let event = match self.0.op {
            OpCode::Dispatch => {
                let name = self.0.event_type.ok_or_else(|| {
                    A::Error::custom("dispatch payload without an event name (`t`)")
                })?;
                let event = match name {
                    "MESSAGE_CREATE" => DispatchEvent::MessageCreate(map.next_value()?),
                    "READY" => DispatchEvent::Ready(map.next_value()?),
                    "RESUMED" => {
                        map.next_value::<IgnoredAny>()?;
                        DispatchEvent::Resumed
                    }
                    other => {
                        map.next_value::<IgnoredAny>()?;
                        DispatchEvent::Unknown(CompactString::from(other))
                    }
                };
                return Ok(Data::Dispatch(event));
            }
            OpCode::InvalidSession => GatewayEvent::InvalidSession(map.next_value()?),
            OpCode::Hello => GatewayEvent::Hello(map.next_value()?),
            op => {
                map.next_value::<IgnoredAny>()?;
                Self::without_data(op)
            }
        };
        Ok(Data::Event(event))
    }

    fn without_data(op: OpCode) -> GatewayEvent {
        match op {
            OpCode::Heartbeat => GatewayEvent::Heartbeat,
            OpCode::Reconnect => GatewayEvent::Reconnect,
            OpCode::HeartbeatAck => GatewayEvent::HeartbeatAck,
            op => GatewayEvent::Unknown(op),
        }
    }
}

impl<'de> Visitor<'de> for EnvelopeVisitor<'_> {
    type Value = GatewayEvent;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a gateway payload object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<GatewayEvent, A::Error> {
        let mut data = None;
        let mut sequence = None;

        while let Some(field) = map.next_key::<Field>()? {
            match field {
                Field::Data => {
                    if data.is_some() {
                        return Err(A::Error::duplicate_field("d"));
                    }
                    data = Some(self.data(&mut map)?);
                }
                Field::Sequence => sequence = map.next_value::<Option<u64>>()?,
                Field::Other => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        match (self.0.op, data) {
            (OpCode::Dispatch, Some(Data::Dispatch(event))) => {
                let sequence = sequence
                    .or(self.0.sequence)
                    .ok_or_else(|| A::Error::missing_field("s"))?;
                Ok(GatewayEvent::Dispatch(sequence, event))
            }
            (_, Some(Data::Event(event))) => Ok(event),
            (OpCode::Dispatch | OpCode::InvalidSession | OpCode::Hello, _) => {
                Err(A::Error::missing_field("d"))
            }
            (op, _) => Ok(Self::without_data(op)),
        }
    }
}

/// The envelope keys the visitor cares about.
enum Field {
    Data,
    Sequence,
    Other,
}

impl<'de> serde::Deserialize<'de> for Field {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldVisitor;

        impl Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a gateway payload key")
            }

            fn visit_str<E: DeError>(self, value: &str) -> Result<Field, E> {
                Ok(match value {
                    "d" => Field::Data,
                    "s" => Field::Sequence,
                    _ => Field::Other,
                })
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

/// A minimal scanner for the top level of a JSON object.
mod scan {
    use super::GatewayEventDeserializer;
    use crate::gateway::OpCode;

    struct Cursor<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl<'a> Cursor<'a> {
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.at).copied()
        }

        fn skip_whitespace(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.at += 1;
            }
        }

        fn eat(&mut self, byte: u8) -> Option<()> {
            self.skip_whitespace();
            (self.peek()? == byte).then(|| self.at += 1)
        }

        /// Reads a string starting at the opening quote. Returns the raw contents (without
        /// quotes) and whether they contain escape sequences.
        fn string(&mut self) -> Option<(&'a [u8], bool)> {
            self.eat(b'"')?;
            let start = self.at;
            let mut escaped = false;
            loop {
                match *self.bytes.get(self.at)? {
                    b'"' => break,
                    b'\\' => {
                        escaped = true;
                        self.at += 2;
                    }
                    _ => self.at += 1,
                }
            }
            let contents = self.bytes.get(start..self.at)?;
            self.at += 1;
            Some((contents, escaped))
        }

        fn literal(&mut self, literal: &[u8]) -> bool {
            self.skip_whitespace();
            let matches = self
                .bytes
                .get(self.at..)
                .is_some_and(|rest| rest.starts_with(literal));
            if matches {
                self.at += literal.len();
            }
            matches
        }

        fn unsigned(&mut self) -> Option<u64> {
            self.skip_whitespace();
            let start = self.at;
            let mut value = 0_u64;
            while let Some(digit @ b'0'..=b'9') = self.peek() {
                value = value
                    .checked_mul(10)?
                    .checked_add(u64::from(digit - b'0'))?;
                self.at += 1;
            }
            (self.at > start).then_some(value)
        }

        /// Skips any JSON value without interpreting it.
        fn skip_value(&mut self) -> Option<()> {
            self.skip_whitespace();
            match self.peek()? {
                b'"' => {
                    self.string()?;
                }
                b'{' | b'[' => {
                    let mut depth = 0_usize;
                    loop {
                        match *self.bytes.get(self.at)? {
                            b'{' | b'[' => {
                                depth += 1;
                                self.at += 1;
                            }
                            b'}' | b']' => {
                                depth -= 1;
                                self.at += 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            b'"' => {
                                self.string()?;
                            }
                            _ => self.at += 1,
                        }
                    }
                }
                _ => {
                    // Number, `true`, `false` or `null`.
                    while !matches!(
                        self.peek()?,
                        b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r'
                    ) {
                        self.at += 1;
                    }
                }
            }
            Some(())
        }
    }

    pub(super) fn top_level(json: &str) -> Option<GatewayEventDeserializer<'_>> {
        let mut cursor = Cursor {
            bytes: json.as_bytes(),
            at: 0,
        };
        let mut op = None;
        let mut sequence = None;
        let mut event_type = None;
        let (mut seen_sequence, mut seen_event_type) = (false, false);

        cursor.eat(b'{')?;
        cursor.skip_whitespace();
        if cursor.peek() != Some(b'}') {
            loop {
                let (key, _) = cursor.string()?;
                cursor.eat(b':')?;
                match key {
                    b"op" => {
                        op = Some(u8::try_from(cursor.unsigned()?).ok()?);
                    }
                    b"s" => {
                        seen_sequence = true;
                        if !cursor.literal(b"null") {
                            sequence = Some(cursor.unsigned()?);
                        }
                    }
                    b"t" => {
                        seen_event_type = true;
                        if !cursor.literal(b"null") {
                            let (name, escaped) = cursor.string()?;
                            if escaped {
                                return None;
                            }
                            event_type = Some(core::str::from_utf8(name).ok()?);
                        }
                    }
                    _ => cursor.skip_value()?,
                }

                // Stop as soon as everything needed is known.
                if let Some(op) = op
                    && (op != OpCode::Dispatch.value() || (seen_sequence && seen_event_type))
                {
                    break;
                }

                cursor.skip_whitespace();
                match cursor.peek()? {
                    b',' => cursor.at += 1,
                    b'}' => break,
                    _ => return None,
                }
            }
        }

        Some(GatewayEventDeserializer::new(
            OpCode::from_value(op?),
            sequence,
            event_type,
        ))
    }
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeSeed;

    use super::*;

    fn decode(json: &str) -> GatewayEvent {
        let seed = GatewayEventDeserializer::from_json(json).expect("scan failed");
        let mut deserializer = serde_json::Deserializer::from_str(json);
        let event = seed
            .deserialize(&mut deserializer)
            .expect("deserialize failed");
        deserializer.end().expect("trailing data");
        event
    }

    #[test]
    fn scan_discord_order() {
        let seed = GatewayEventDeserializer::from_json(
            r#"{"t":"MESSAGE_CREATE","s":3,"op":0,"d":{"never":"read"}}"#,
        )
        .unwrap();
        assert_eq!(seed.op(), OpCode::Dispatch);
        assert_eq!(seed.sequence(), Some(3));
        assert_eq!(seed.event_type(), Some("MESSAGE_CREATE"));
    }

    #[test]
    fn scan_any_order_and_nested_decoys() {
        let json = r#"{ "d" : {"op": 9, "t": "DECOY", "s": 1, "x": [{"op":1}, "\"op\":2"]},
                        "op" : 0 , "s" : 77, "t" : "READY" }"#;
        let seed = GatewayEventDeserializer::from_json(json).unwrap();
        assert_eq!(seed.op(), OpCode::Dispatch);
        assert_eq!(seed.sequence(), Some(77));
        assert_eq!(seed.event_type(), Some("READY"));
    }

    #[test]
    fn scan_rejects_garbage() {
        for json in [
            "",
            "[]",
            "{}",
            r#"{"op":"1"}"#,
            r#"{"op":300}"#,
            "{\"t\":\"A\\u0042\",\"op\":0}",
        ] {
            assert!(
                GatewayEventDeserializer::from_json(json).is_none(),
                "{json}"
            );
        }
    }

    #[test]
    fn hello_and_control_events() {
        assert_eq!(
            decode(r#"{"t":null,"s":null,"op":10,"d":{"heartbeat_interval":45000}}"#),
            GatewayEvent::Hello(Hello {
                heartbeat_interval: 45000
            })
        );
        assert_eq!(decode(r#"{"op":11}"#), GatewayEvent::HeartbeatAck);
        assert_eq!(decode(r#"{"op":1,"d":null}"#), GatewayEvent::Heartbeat);
        assert_eq!(
            decode(r#"{"t":null,"s":null,"op":7,"d":null}"#),
            GatewayEvent::Reconnect
        );
        assert_eq!(
            decode(r#"{"op":9,"d":false}"#),
            GatewayEvent::InvalidSession(false)
        );
        assert_eq!(
            decode(r#"{"op":42,"d":{"x":1}}"#),
            GatewayEvent::Unknown(OpCode::Unknown(42))
        );
    }

    #[test]
    fn unknown_dispatch_is_skipped() {
        let event = decode(r#"{"t":"GUILD_CREATE","s":2,"op":0,"d":{"id":"1","big":[1,2,3]}}"#);
        assert_eq!(
            event,
            GatewayEvent::Dispatch(2, DispatchEvent::Unknown("GUILD_CREATE".into()))
        );
    }

    #[test]
    fn resumed() {
        let event = decode(r#"{"t":"RESUMED","s":10,"op":0,"d":{}}"#);
        assert_eq!(event, GatewayEvent::Dispatch(10, DispatchEvent::Resumed));
    }

    #[test]
    fn dispatch_without_sequence_fails() {
        let json = r#"{"t":"RESUMED","op":0,"d":{}}"#;
        let seed = GatewayEventDeserializer::from_json(json).unwrap();
        let mut deserializer = serde_json::Deserializer::from_str(json);
        assert!(seed.deserialize(&mut deserializer).is_err());
    }
}
