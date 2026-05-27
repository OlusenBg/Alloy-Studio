//! Broadcast channel wrapper with integrated history storage.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use tokio::sync::broadcast;

use crate::history::{HistoryBuffer, PacketRecord, DEFAULT_HISTORY_CAPACITY};
use crate::protocol::{Packet, PacketKind};

/// Capacity of the broadcast channel (number of in-flight packets before oldest is dropped).
pub const BROADCAST_CAPACITY: usize = 512;

/// A shared telemetry stream: broadcasts every incoming packet and stores
/// numeric values in a ring-buffer history for later charting.
pub struct TelemetryStream {
    tx: broadcast::Sender<Packet>,
    history: Arc<HistoryBuffer>,
    packet_count: AtomicU64,
}

impl Default for TelemetryStream {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryStream {
    /// Create a new stream with a fresh history buffer.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            tx,
            history: Arc::new(HistoryBuffer::new(DEFAULT_HISTORY_CAPACITY)),
            packet_count: AtomicU64::new(0),
        }
    }

    /// Subscribe to the broadcast stream.
    ///
    /// The returned receiver will receive every packet published after this call.
    pub fn subscribe(&self) -> broadcast::Receiver<Packet> {
        self.tx.subscribe()
    }

    /// Access the shared history buffer.
    pub fn history(&self) -> Arc<HistoryBuffer> {
        Arc::clone(&self.history)
    }

    /// Total packets published since the stream was created.
    pub fn packet_count(&self) -> u64 {
        self.packet_count.load(Ordering::Relaxed)
    }

    /// Publish a packet: store numeric values in history, then broadcast.
    ///
    /// Mapping of packet kinds to history keys:
    /// - `KeyValue`    → `"{key}"`
    /// - `Battery`     → `"battery_volts"`
    /// - `Gyro`        → `"gyro_heading"`
    /// - `EncoderTick` → `"encoder_{name}_ticks"` and `"encoder_{name}_velocity"`
    /// - `Imu`         → `"imu_roll"`, `"imu_pitch"`, `"imu_yaw"`
    pub fn publish(&self, packet: Packet) {
        let ts = packet.timestamp_ms;

        match &packet.kind {
            PacketKind::KeyValue { key, value } => {
                self.history.insert(
                    key,
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *value,
                    },
                );
            }
            PacketKind::Battery { volts } => {
                self.history.insert(
                    "battery_volts",
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *volts,
                    },
                );
            }
            PacketKind::Gyro { heading_deg } => {
                self.history.insert(
                    "gyro_heading",
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *heading_deg,
                    },
                );
            }
            PacketKind::EncoderTick {
                name,
                ticks,
                velocity_tps,
            } => {
                self.history.insert(
                    &format!("encoder_{}_ticks", name),
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *ticks as f64,
                    },
                );
                self.history.insert(
                    &format!("encoder_{}_velocity", name),
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *velocity_tps,
                    },
                );
            }
            PacketKind::Imu { roll, pitch, yaw } => {
                self.history.insert(
                    "imu_roll",
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *roll,
                    },
                );
                self.history.insert(
                    "imu_pitch",
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *pitch,
                    },
                );
                self.history.insert(
                    "imu_yaw",
                    PacketRecord {
                        timestamp_ms: ts,
                        value: *yaw,
                    },
                );
            }
            // Non-numeric kinds: broadcast only, no history entry
            PacketKind::KeyString { .. }
            | PacketKind::OpModeStatus { .. }
            | PacketKind::Ping { .. }
            | PacketKind::Pong { .. } => {}
        }

        self.packet_count.fetch_add(1, Ordering::Relaxed);
        // Ignore send errors: they just mean there are no current subscribers.
        let _ = self.tx.send(packet);
    }

    /// Number of active broadcast subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Packet;

    #[test]
    fn publish_key_value_stored_in_history() {
        let stream = TelemetryStream::new();
        let p = Packet::key_value("speed", 42.0);
        stream.publish(p);

        let records = stream.history().get("speed");
        assert_eq!(records.len(), 1);
        assert!((records[0].value - 42.0).abs() < 1e-9);
    }

    #[test]
    fn publish_battery_stored_in_history() {
        let stream = TelemetryStream::new();
        stream.publish(Packet::battery(12.6));

        let records = stream.history().get("battery_volts");
        assert_eq!(records.len(), 1);
        assert!((records[0].value - 12.6).abs() < 1e-9);
    }

    #[test]
    fn publish_gyro_stored_in_history() {
        let stream = TelemetryStream::new();
        stream.publish(Packet::gyro(180.0));

        let records = stream.history().get("gyro_heading");
        assert_eq!(records.len(), 1);
        assert!((records[0].value - 180.0).abs() < 1e-9);
    }

    #[test]
    fn publish_encoder_stored_in_history() {
        let stream = TelemetryStream::new();
        stream.publish(Packet::encoder("left_motor", 9999, 750.0));

        let ticks = stream.history().get("encoder_left_motor_ticks");
        let vel = stream.history().get("encoder_left_motor_velocity");
        assert_eq!(ticks.len(), 1);
        assert!((ticks[0].value - 9999.0).abs() < 1e-9);
        assert!((vel[0].value - 750.0).abs() < 1e-9);
    }

    #[test]
    fn publish_imu_stored_in_history() {
        let stream = TelemetryStream::new();
        stream.publish(Packet::imu(10.0, 20.0, 30.0));

        assert!((stream.history().get("imu_roll")[0].value - 10.0).abs() < 1e-9);
        assert!((stream.history().get("imu_pitch")[0].value - 20.0).abs() < 1e-9);
        assert!((stream.history().get("imu_yaw")[0].value - 30.0).abs() < 1e-9);
    }

    #[test]
    fn subscribe_receives_packets() {
        let stream = TelemetryStream::new();
        let mut rx = stream.subscribe();

        stream.publish(Packet::battery(11.5));

        let received = rx.try_recv().expect("should have received a packet");
        match received.kind {
            crate::protocol::PacketKind::Battery { volts } => {
                assert!((volts - 11.5).abs() < 1e-9);
            }
            other => panic!("unexpected packet kind: {:?}", other),
        }
    }

    #[test]
    fn packet_count_increments() {
        let stream = TelemetryStream::new();
        assert_eq!(stream.packet_count(), 0);

        stream.publish(Packet::battery(12.0));
        stream.publish(Packet::gyro(45.0));
        assert_eq!(stream.packet_count(), 2);
    }

    #[test]
    fn ping_not_stored_in_history() {
        let stream = TelemetryStream::new();
        stream.publish(Packet::ping(1));
        // History should be empty — pings are not numeric data
        assert!(stream.history().is_empty());
        // But packet count should still increment
        assert_eq!(stream.packet_count(), 1);
    }

    #[test]
    fn subscriber_count() {
        let stream = TelemetryStream::new();
        assert_eq!(stream.subscriber_count(), 0);
        let _rx1 = stream.subscribe();
        assert_eq!(stream.subscriber_count(), 1);
        let _rx2 = stream.subscribe();
        assert_eq!(stream.subscriber_count(), 2);
    }
}
