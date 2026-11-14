//! The bytes of the voice UDP connection that are not encrypted: the RTP header of the
//! packets djs-voice sends, and IP discovery, which tells a client the address and port the
//! voice server sees it at (behind NAT), to give back in Select Protocol.
//!
//! Discord documents both in "Voice Packet Structure" and "IP Discovery" of
//! <https://discord.com/developers/docs/topics/voice-connections>.

#![allow(dead_code, reason = "used by the connection from step 1c")]

use core::net::{IpAddr, SocketAddr};

use crate::crypto::RTP_HEADER;

/// RTP version 2, no padding, no extension, no CSRCs.
const VERSION_FLAGS: u8 = 0x80;
/// The payload type Discord uses for Opus.
const OPUS_PAYLOAD_TYPE: u8 = 0x78;

/// The RTP header of a packet djs-voice sends.
pub(crate) fn rtp_header(sequence: u16, timestamp: u32, ssrc: u32) -> [u8; RTP_HEADER] {
    let mut header = [0; RTP_HEADER];
    header[0] = VERSION_FLAGS;
    header[1] = OPUS_PAYLOAD_TYPE;
    header[2..4].copy_from_slice(&sequence.to_be_bytes());
    header[4..8].copy_from_slice(&timestamp.to_be_bytes());
    header[8..12].copy_from_slice(&ssrc.to_be_bytes());
    header
}

/// A UDP keepalive: the SSRC, big-endian. Sent every few seconds while no audio is, so that
/// NAT keeps the socket's mapping (Discord does not document it; songbird and discord.js
/// send one).
pub(crate) const fn keepalive(ssrc: u32) -> [u8; 4] {
    ssrc.to_be_bytes()
}

/// The length of an IP discovery packet, request or response.
pub(crate) const DISCOVERY: usize = 74;
/// What the length field holds: the packet without its type and length.
const DISCOVERY_LENGTH: u16 = 70;
const REQUEST: u16 = 1;
const RESPONSE: u16 = 2;

/// An IP discovery request for the SSRC from Ready.
pub(crate) fn discovery_request(ssrc: u32) -> [u8; DISCOVERY] {
    let mut packet = [0; DISCOVERY];
    packet[..2].copy_from_slice(&REQUEST.to_be_bytes());
    packet[2..4].copy_from_slice(&DISCOVERY_LENGTH.to_be_bytes());
    packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
    packet
}

/// The address in an IP discovery response to the request for `ssrc`; `None` for any other
/// packet (another type or SSRC, or an address that does not parse).
pub(crate) fn discovery_response(packet: &[u8], ssrc: u32) -> Option<SocketAddr> {
    let packet: &[u8; DISCOVERY] = packet.try_into().ok()?;
    let field = |at: usize| u16::from_be_bytes([packet[at], packet[at + 1]]);
    if field(0) != RESPONSE || field(2) != DISCOVERY_LENGTH || packet[4..8] != ssrc.to_be_bytes() {
        return None;
    }
    // A null-terminated string in 64 bytes.
    let address = &packet[8..72];
    let end = address.iter().position(|&byte| byte == 0)?;
    let ip: IpAddr = core::str::from_utf8(&address[..end]).ok()?.parse().ok()?;
    Some(SocketAddr::new(ip, field(72)))
}

#[cfg(test)]
mod tests {
    use core::net::SocketAddr;

    use super::{DISCOVERY, discovery_request, discovery_response, rtp_header};

    #[test]
    fn headers_follow_discords_layout() {
        assert_eq!(
            rtp_header(0x1234, 0x89AB_CDEF, 0x0102_0304),
            [
                0x80, 0x78, 0x12, 0x34, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x02, 0x03, 0x04
            ]
        );
    }

    #[test]
    fn discovery_requests_follow_discords_layout() {
        let request = discovery_request(0xDEAD_BEEF);
        assert_eq!(request[..8], [0, 1, 0, 70, 0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(request[8..].iter().all(|&byte| byte == 0));
    }

    fn response(ssrc: u32, address: &str, port: u16) -> [u8; DISCOVERY] {
        let mut packet = [0; DISCOVERY];
        packet[..4].copy_from_slice(&[0, 2, 0, 70]);
        packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
        packet[8..8 + address.len()].copy_from_slice(address.as_bytes());
        packet[72..].copy_from_slice(&port.to_be_bytes());
        packet
    }

    #[test]
    fn discovery_responses_give_the_address() {
        assert_eq!(
            discovery_response(&response(7, "203.0.113.9", 50_123), 7),
            Some("203.0.113.9:50123".parse::<SocketAddr>().unwrap())
        );
        assert_eq!(
            discovery_response(&response(7, "2001:db8::1", 4), 7),
            Some("[2001:db8::1]:4".parse::<SocketAddr>().unwrap())
        );
        // The longest address that leaves room for its terminator.
        let long = "0000:0000:0000:0000:0000:ffff:192.168.100.200";
        assert!(discovery_response(&response(7, long, 1), 7).is_some());
    }

    #[test]
    fn other_packets_are_not_responses() {
        let good = response(7, "203.0.113.9", 1);
        assert_eq!(discovery_response(&good, 8), None, "another SSRC");
        assert_eq!(discovery_response(&good[..73], 7), None, "too short");
        let mut request = good;
        request[1] = 1;
        assert_eq!(discovery_response(&request, 7), None, "a request");
        let mut length = good;
        length[3] = 71;
        assert_eq!(discovery_response(&length, 7), None, "another length");
        assert_eq!(
            discovery_response(&response(7, "not an address", 1), 7),
            None
        );
        let mut unterminated = good;
        unterminated[8..72].fill(b'1');
        assert_eq!(discovery_response(&unterminated, 7), None);
    }
}
