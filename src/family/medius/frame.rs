use crate::capability::Firmware;
use crate::transport::codec::{Codec, Frame, Incoming};

/// just enough of the Medius frame to send a raw QUERY(VERSION) without the crate. a frame is
/// `A5 TYPE SEQ LEN_LO LEN_HI PAYLOAD CRC_LO CRC_HI` and the CRC covers everything after the A5
pub(crate) struct MediusFrames;

const START: u8 = 0xA5;
const QUERY: u8 = 0x05;
const RESP: u8 = 0x06;
const WHAT_VERSION: u8 = 0x00;

/// CRC-16/CCITT-FALSE, so poly 0x1021, init 0xFFFF, no reflection and no final xor
pub(crate) fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub(crate) fn query_version(seq: u8) -> Vec<u8> {
    let body = [QUERY, seq, 0x01, 0x00, WHAT_VERSION];
    let mut frame = vec![START];
    frame.extend_from_slice(&body);
    frame.extend_from_slice(&crc16(&body).to_le_bytes());
    frame
}

/// the firmware a RESP(VERSION) frame reports, or none when it's another frame or its CRC is wrong
pub(crate) fn version_reply(frame: &[u8]) -> Option<Firmware> {
    let body = frame.get(1..frame.len().checked_sub(2)?)?;
    let crc = u16::from_le_bytes([frame[frame.len() - 2], frame[frame.len() - 1]]);
    if crc16(body) != crc || body.first() != Some(&RESP) {
        return None;
    }
    // the payload is what, proto_ver, major, minor, patch, mac[6] and then the name
    let payload = body.get(4..)?;
    if payload.first() != Some(&WHAT_VERSION) || payload.len() < 5 {
        return None;
    }
    Some(Firmware::Version {
        major: u16::from(payload[2]),
        minor: u16::from(payload[3]),
        patch: u16::from(payload[4]),
    })
}

impl Codec for MediusFrames {
    fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame> {
        let start = buffer.iter().position(|&b| b == START);
        let Some(start) = start else {
            buffer.clear();
            return None;
        };
        buffer.drain(..start);
        if buffer.len() < 5 {
            return None;
        }
        let total = 5 + usize::from(u16::from_le_bytes([buffer[3], buffer[4]])) + 2;
        if buffer.len() < total {
            return None;
        }
        Some(buffer.drain(..total).collect())
    }

    fn classify(&mut self, frame: &Frame) -> Incoming {
        // a RESP carries the same SEQ as the query it answers
        if frame.get(1) == Some(&RESP) {
            Incoming::Reply(u32::from(frame[2]))
        } else {
            Incoming::Noise
        }
    }
}
