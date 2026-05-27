//! Wire protocol: 4-byte big-endian length prefix + JSON payload.

use bytes::{Buf, BufMut, BytesMut};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::TelemetryError;

/// Maximum allowed packet payload size (64 KiB).
pub const MAX_PACKET_BYTES: u32 = 64 * 1024;

// ── Packet types ──────────────────────────────────────────────────────────────

/// The variant-level data for a telemetry packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PacketKind {
    /// A named numeric key/value pair.
    KeyValue { key: String, value: f64 },
    /// A named string key/value pair.
    KeyString { key: String, value: String },
    /// Encoder position and velocity.
    EncoderTick {
        name: String,
        ticks: i64,
        velocity_tps: f64,
    },
    /// Battery voltage reading.
    Battery { volts: f64 },
    /// Gyro heading.
    Gyro { heading_deg: f64 },
    /// IMU orientation (all angles in degrees).
    Imu { roll: f64, pitch: f64, yaw: f64 },
    /// OpMode lifecycle notification.
    OpModeStatus {
        name: String,
        /// One of `"RUNNING"`, `"STOPPED"`, `"INIT"`, etc.
        status: String,
    },
    /// Ping — the robot sends this; the server should reply with `Pong`.
    Ping { seq: u64 },
    /// Pong — the server's reply to `Ping`.
    Pong { seq: u64 },
}

/// A framed telemetry packet with a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Unix epoch milliseconds at the time of creation.
    pub timestamp_ms: i64,
    #[serde(flatten)]
    pub kind: PacketKind,
}

impl Packet {
    /// Create a packet stamped with the current UTC time.
    pub fn now(kind: PacketKind) -> Self {
        Self {
            timestamp_ms: Utc::now().timestamp_millis(),
            kind,
        }
    }

    /// Convenience constructor for a `KeyValue` packet.
    pub fn key_value(key: impl Into<String>, value: f64) -> Self {
        Self::now(PacketKind::KeyValue {
            key: key.into(),
            value,
        })
    }

    /// Convenience constructor for a `Battery` packet.
    pub fn battery(volts: f64) -> Self {
        Self::now(PacketKind::Battery { volts })
    }

    /// Convenience constructor for a `Gyro` packet.
    pub fn gyro(heading_deg: f64) -> Self {
        Self::now(PacketKind::Gyro { heading_deg })
    }

    /// Convenience constructor for an `Imu` packet.
    pub fn imu(roll: f64, pitch: f64, yaw: f64) -> Self {
        Self::now(PacketKind::Imu { roll, pitch, yaw })
    }

    /// Convenience constructor for an `EncoderTick` packet.
    pub fn encoder(name: impl Into<String>, ticks: i64, velocity_tps: f64) -> Self {
        Self::now(PacketKind::EncoderTick {
            name: name.into(),
            ticks,
            velocity_tps,
        })
    }

    /// Convenience constructor for a `Ping` packet.
    pub fn ping(seq: u64) -> Self {
        Self::now(PacketKind::Ping { seq })
    }
}

// ── Codec ─────────────────────────────────────────────────────────────────────

/// A `tokio-util` codec that frames packets as:
///
/// ```text
/// [4-byte big-endian length][JSON payload of `length` bytes]
/// ```
pub struct PacketCodec;

impl Encoder<Packet> for PacketCodec {
    type Error = TelemetryError;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_vec(&item)?;
        let len = json.len() as u32;
        dst.reserve(4 + json.len());
        dst.put_u32(len);
        dst.put_slice(&json);
        Ok(())
    }
}

impl Decoder for PacketCodec {
    type Item = Packet;
    type Error = TelemetryError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least the 4-byte length header
        if src.len() < 4 {
            return Ok(None);
        }

        // Peek at the length without consuming
        let len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]);

        // Sanity-check the declared length
        if len > MAX_PACKET_BYTES {
            return Err(TelemetryError::InvalidLength(len));
        }

        let total = 4 + len as usize;

        // Need the full frame in the buffer
        if src.len() < total {
            src.reserve(total - src.len());
            return Ok(None);
        }

        // Consume the 4-byte header
        src.advance(4);

        // Read the payload
        let payload = src.split_to(len as usize);
        let packet = serde_json::from_slice::<Packet>(&payload)?;
        Ok(Some(packet))
    }
}

/// A convenience type alias for a framed `tokio` I/O stream using `PacketCodec`.
pub type TelemetryFramed<T> = tokio_util::codec::Framed<T, PacketCodec>;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn roundtrip(packet: Packet) -> Packet {
        let mut codec = PacketCodec;
        let mut buf = BytesMut::new();
        codec.encode(packet, &mut buf).expect("encode failed");

        let decoded = codec.decode(&mut buf).expect("decode error").expect("no packet");
        decoded
    }

    #[test]
    fn encode_decode_key_value() {
        let p = Packet::key_value("speed", 1.23);
        let decoded = roundtrip(p);
        match decoded.kind {
            PacketKind::KeyValue { key, value } => {
                assert_eq!(key, "speed");
                assert!((value - 1.23).abs() < 1e-9);
            }
            other => panic!("unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn encode_decode_battery() {
        let p = Packet::battery(12.6);
        let decoded = roundtrip(p);
        match decoded.kind {
            PacketKind::Battery { volts } => assert!((volts - 12.6).abs() < 1e-9),
            other => panic!("unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn encode_decode_imu() {
        let p = Packet::imu(1.0, 2.0, 3.0);
        let decoded = roundtrip(p);
        match decoded.kind {
            PacketKind::Imu { roll, pitch, yaw } => {
                assert!((roll - 1.0).abs() < 1e-9);
                assert!((pitch - 2.0).abs() < 1e-9);
                assert!((yaw - 3.0).abs() < 1e-9);
            }
            other => panic!("unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn encode_decode_encoder() {
        let p = Packet::encoder("left_motor", 12345, 500.0);
        let decoded = roundtrip(p);
        match decoded.kind {
            PacketKind::EncoderTick {
                name,
                ticks,
                velocity_tps,
            } => {
                assert_eq!(name, "left_motor");
                assert_eq!(ticks, 12345);
                assert!((velocity_tps - 500.0).abs() < 1e-9);
            }
            other => panic!("unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn encode_decode_ping() {
        let p = Packet::ping(42);
        let decoded = roundtrip(p);
        match decoded.kind {
            PacketKind::Ping { seq } => assert_eq!(seq, 42),
            other => panic!("unexpected kind: {:?}", other),
        }
    }

    #[test]
    fn partial_data_returns_none() {
        let mut codec = PacketCodec;
        let p = Packet::battery(11.0);
        let mut buf = BytesMut::new();
        codec.encode(p, &mut buf).unwrap();

        // Truncate to partial data
        let partial = buf.split_to(buf.len() - 3);
        let mut partial = partial; // make mutable
        // This should need more data, not error
        // We need a new partial buffer that is missing the last 3 bytes
        // but has the full length header
        let result = codec.decode(&mut partial);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn rejects_oversized_packet() {
        let mut codec = PacketCodec;
        let mut buf = BytesMut::new();
        // Write a length that exceeds MAX_PACKET_BYTES
        buf.put_u32(MAX_PACKET_BYTES + 1);
        buf.put_slice(&vec![0u8; 100]); // dummy payload (won't be read)
        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(TelemetryError::InvalidLength(_))));
    }

    #[test]
    fn multiple_packets_in_buffer() {
        let mut codec = PacketCodec;
        let mut buf = BytesMut::new();

        codec.encode(Packet::battery(12.0), &mut buf).unwrap();
        codec.encode(Packet::gyro(90.0), &mut buf).unwrap();

        let p1 = codec.decode(&mut buf).unwrap().unwrap();
        let p2 = codec.decode(&mut buf).unwrap().unwrap();

        assert!(matches!(p1.kind, PacketKind::Battery { .. }));
        assert!(matches!(p2.kind, PacketKind::Gyro { .. }));
    }
}
