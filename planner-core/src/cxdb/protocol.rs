//! # CXDB Binary Protocol
//!
//! MessagePack-framed binary protocol for high-throughput turn ingestion.
//! The protocol is simple:
//!
//! ```text
//! ┌────────┬──────────┬─────────┐
//! │ length │ msg_type │ payload │
//! │ 4 bytes│ 1 byte   │ N bytes │
//! └────────┴──────────┴─────────┘
//! ```
//!
//! ## Message Types
//! - `0x01` StoreTurn — write a turn + blob
//! - `0x02` StoreTurnAck — acknowledgement
//! - `0x03` Ping — keep-alive
//! - `0x04` Pong — keep-alive response

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Wire frame
// ---------------------------------------------------------------------------

/// Maximum frame size: 32 MiB (header + payload).
pub const MAX_FRAME_SIZE: u32 = 32 * 1024 * 1024;

/// Protocol message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    StoreTurn = 0x01,
    StoreTurnAck = 0x02,
    Ping = 0x03,
    Pong = 0x04,
}

impl TryFrom<u8> for MessageType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(MessageType::StoreTurn),
            0x02 => Ok(MessageType::StoreTurnAck),
            0x03 => Ok(MessageType::Ping),
            0x04 => Ok(MessageType::Pong),
            _ => Err(ProtocolError::UnknownMessageType(value)),
        }
    }
}

/// A framed protocol message.
#[derive(Debug, Clone)]
pub struct Frame {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Encode a frame into wire bytes: [length:4][msg_type:1][payload:N].
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() as u32 + 1; // +1 for msg_type byte
        let mut buf = Vec::with_capacity(4 + 1 + self.payload.len());
        buf.extend_from_slice(&payload_len.to_be_bytes());
        buf.push(self.msg_type as u8);
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode a frame from wire bytes. Returns (frame, bytes_consumed).
    pub fn decode(buf: &[u8]) -> Result<(Frame, usize), ProtocolError> {
        if buf.len() < 5 {
            return Err(ProtocolError::InsufficientData);
        }

        let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if length > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge(length));
        }

        let total_len = 4 + length as usize;
        if buf.len() < total_len {
            return Err(ProtocolError::InsufficientData);
        }

        let msg_type = MessageType::try_from(buf[4])?;
        let payload = buf[5..total_len].to_vec();

        Ok((Frame { msg_type, payload }, total_len))
    }
}

// ---------------------------------------------------------------------------
// StoreTurn message payload
// ---------------------------------------------------------------------------

/// StoreTurn wire message — the client sends this to write a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreTurnMessage {
    pub turn_id: String,
    pub type_id: String,
    pub parent_id: Option<String>,
    pub blob_hash: String,
    pub blob_data: Vec<u8>,
    pub run_id: String,
    pub execution_id: String,
    pub produced_by: String,
    pub created_at: String,
    pub note: Option<String>,
    pub project_id: Option<String>,
}

/// StoreTurnAck — the server sends this after successful write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreTurnAck {
    pub turn_id: String,
    pub blob_hash: String,
    pub deduped: bool,
}

// ---------------------------------------------------------------------------
// Protocol errors
// ---------------------------------------------------------------------------

/// Protocol-level errors.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unknown message type: 0x{0:02x}")]
    UnknownMessageType(u8),

    #[error("Frame too large: {0} bytes (max {MAX_FRAME_SIZE})")]
    FrameTooLarge(u32),

    #[error("Insufficient data for frame decode")]
    InsufficientData,

    #[error("Invalid MessagePack payload: {0}")]
    InvalidPayload(String),

    #[error("Invalid UUID in message: {0}")]
    InvalidUuid(String),
}

// ---------------------------------------------------------------------------
// Encode/decode helpers
// ---------------------------------------------------------------------------

impl StoreTurnMessage {
    /// Serialize to MessagePack bytes.
    pub fn to_msgpack(&self) -> Result<Vec<u8>, ProtocolError> {
        rmp_serde::to_vec(self)
            .map_err(|e| ProtocolError::InvalidPayload(e.to_string()))
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_msgpack(data: &[u8]) -> Result<Self, ProtocolError> {
        rmp_serde::from_slice(data)
            .map_err(|e| ProtocolError::InvalidPayload(e.to_string()))
    }

    /// Convert to a Frame ready for wire transmission.
    pub fn into_frame(self) -> Result<Frame, ProtocolError> {
        Ok(Frame {
            msg_type: MessageType::StoreTurn,
            payload: self.to_msgpack()?,
        })
    }

    /// Extract turn_id as UUID.
    pub fn parse_turn_id(&self) -> Result<Uuid, ProtocolError> {
        Uuid::parse_str(&self.turn_id)
            .map_err(|e| ProtocolError::InvalidUuid(e.to_string()))
    }
}

impl StoreTurnAck {
    /// Serialize to MessagePack bytes.
    pub fn to_msgpack(&self) -> Result<Vec<u8>, ProtocolError> {
        rmp_serde::to_vec(self)
            .map_err(|e| ProtocolError::InvalidPayload(e.to_string()))
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_msgpack(data: &[u8]) -> Result<Self, ProtocolError> {
        rmp_serde::from_slice(data)
            .map_err(|e| ProtocolError::InvalidPayload(e.to_string()))
    }

    /// Convert to a Frame ready for wire transmission.
    pub fn into_frame(self) -> Result<Frame, ProtocolError> {
        Ok(Frame {
            msg_type: MessageType::StoreTurnAck,
            payload: self.to_msgpack()?,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let frame = Frame {
            msg_type: MessageType::StoreTurn,
            payload: b"hello world".to_vec(),
        };

        let encoded = frame.encode();
        let (decoded, consumed) = Frame::decode(&encoded).unwrap();

        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.msg_type, MessageType::StoreTurn);
        assert_eq!(decoded.payload, b"hello world");
    }

    #[test]
    fn frame_ping_pong() {
        let ping = Frame {
            msg_type: MessageType::Ping,
            payload: vec![],
        };

        let encoded = ping.encode();
        let (decoded, _) = Frame::decode(&encoded).unwrap();
        assert_eq!(decoded.msg_type, MessageType::Ping);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn frame_decode_insufficient_data() {
        let result = Frame::decode(&[0x00, 0x00]);
        assert!(matches!(result, Err(ProtocolError::InsufficientData)));
    }

    #[test]
    fn frame_decode_unknown_type() {
        let buf = [0x00, 0x00, 0x00, 0x01, 0xFF]; // length=1, type=0xFF
        let result = Frame::decode(&buf);
        assert!(matches!(result, Err(ProtocolError::UnknownMessageType(0xFF))));
    }

    #[test]
    fn frame_too_large_rejected() {
        let mut buf = [0u8; 5];
        buf[0..4].copy_from_slice(&(MAX_FRAME_SIZE + 1).to_be_bytes());
        buf[4] = 0x01;
        let result = Frame::decode(&buf);
        assert!(matches!(result, Err(ProtocolError::FrameTooLarge(_))));
    }

    #[test]
    fn store_turn_message_roundtrip() {
        let msg = StoreTurnMessage {
            turn_id: Uuid::new_v4().to_string(),
            type_id: "planner.intake.v1".into(),
            parent_id: None,
            blob_hash: "abc123".into(),
            blob_data: vec![1, 2, 3, 4],
            run_id: Uuid::new_v4().to_string(),
            execution_id: "exec-1".into(),
            produced_by: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            note: Some("test note".into()),
            project_id: None,
        };

        let bytes = msg.to_msgpack().unwrap();
        let decoded = StoreTurnMessage::from_msgpack(&bytes).unwrap();

        assert_eq!(decoded.turn_id, msg.turn_id);
        assert_eq!(decoded.type_id, msg.type_id);
        assert_eq!(decoded.blob_data, msg.blob_data);
    }

    #[test]
    fn store_turn_message_into_frame() {
        let msg = StoreTurnMessage {
            turn_id: Uuid::new_v4().to_string(),
            type_id: "planner.intake.v1".into(),
            parent_id: None,
            blob_hash: "abc123".into(),
            blob_data: vec![],
            run_id: Uuid::new_v4().to_string(),
            execution_id: "exec-1".into(),
            produced_by: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            note: None,
            project_id: None,
        };

        let frame = msg.into_frame().unwrap();
        assert_eq!(frame.msg_type, MessageType::StoreTurn);
        assert!(!frame.payload.is_empty());
    }

    #[test]
    fn store_turn_ack_roundtrip() {
        let ack = StoreTurnAck {
            turn_id: Uuid::new_v4().to_string(),
            blob_hash: "deadbeef".into(),
            deduped: true,
        };

        let bytes = ack.to_msgpack().unwrap();
        let decoded = StoreTurnAck::from_msgpack(&bytes).unwrap();

        assert_eq!(decoded.turn_id, ack.turn_id);
        assert!(decoded.deduped);
    }

    #[test]
    fn message_type_conversions() {
        assert_eq!(MessageType::try_from(0x01).unwrap(), MessageType::StoreTurn);
        assert_eq!(MessageType::try_from(0x02).unwrap(), MessageType::StoreTurnAck);
        assert_eq!(MessageType::try_from(0x03).unwrap(), MessageType::Ping);
        assert_eq!(MessageType::try_from(0x04).unwrap(), MessageType::Pong);
        assert!(MessageType::try_from(0x99).is_err());
    }
}
