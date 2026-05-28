//! Ring-buffer history of recent numeric telemetry values, keyed by name.

use dashmap::DashMap;
use std::collections::VecDeque;

/// Default capacity (number of records) per key in the history buffer.
pub const DEFAULT_HISTORY_CAPACITY: usize = 500;

/// A single recorded data point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacketRecord {
    /// Unix epoch milliseconds.
    pub timestamp_ms: i64,
    /// The numeric value at that time.
    pub value: f64,
}

/// A thread-safe ring buffer that stores the most recent `capacity` records per key.
pub struct HistoryBuffer {
    data: DashMap<String, VecDeque<PacketRecord>>,
    capacity: usize,
}

impl HistoryBuffer {
    /// Create a new buffer with the given per-key capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: DashMap::new(),
            capacity,
        }
    }

    /// Insert a record under `key`.
    ///
    /// If the deque for `key` is at capacity the oldest record is discarded.
    pub fn insert(&self, key: &str, record: PacketRecord) {
        let mut deque = self
            .data
            .entry(key.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.capacity));

        if deque.len() >= self.capacity {
            deque.pop_front();
        }
        deque.push_back(record);
    }

    /// Return a cloned snapshot of all records for `key`, oldest first.
    pub fn get(&self, key: &str) -> Vec<PacketRecord> {
        self.data
            .get(key)
            .map(|deque| deque.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Return all keys that have at least one record.
    pub fn keys(&self) -> Vec<String> {
        self.data.iter().map(|e| e.key().clone()).collect()
    }

    /// Remove all stored records.
    pub fn clear(&self) {
        self.data.clear();
    }

    /// Total number of records across all keys.
    pub fn len(&self) -> usize {
        self.data.iter().map(|e| e.value().len()).sum()
    }

    /// Returns `true` if the buffer contains no records.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn record(ts: i64, value: f64) -> PacketRecord {
        PacketRecord {
            timestamp_ms: ts,
            value,
        }
    }

    #[test]
    fn insert_and_get() {
        let buf = HistoryBuffer::new(10);
        buf.insert("speed", record(1000, 1.5));
        buf.insert("speed", record(2000, 2.5));

        let records = buf.get("speed");
        assert_eq!(records.len(), 2);
        assert!((records[0].value - 1.5).abs() < 1e-9);
        assert!((records[1].value - 2.5).abs() < 1e-9);
    }

    #[test]
    fn respects_capacity() {
        let buf = HistoryBuffer::new(3);
        for i in 0..5 {
            buf.insert("key", record(i as i64, i as f64));
        }
        let records = buf.get("key");
        assert_eq!(records.len(), 3);
        // Oldest values (0, 1) should be evicted; newest (2, 3, 4) remain
        assert!((records[0].value - 2.0).abs() < 1e-9);
        assert!((records[1].value - 3.0).abs() < 1e-9);
        assert!((records[2].value - 4.0).abs() < 1e-9);
    }

    #[test]
    fn get_missing_key_returns_empty() {
        let buf = HistoryBuffer::new(10);
        assert!(buf.get("nonexistent").is_empty());
    }

    #[test]
    fn keys_returns_all_inserted_keys() {
        let buf = HistoryBuffer::new(10);
        buf.insert("alpha", record(0, 1.0));
        buf.insert("beta", record(0, 2.0));
        let mut keys = buf.keys();
        keys.sort();
        assert_eq!(keys, vec!["alpha", "beta"]);
    }

    #[test]
    fn len_counts_all_records() {
        let buf = HistoryBuffer::new(10);
        buf.insert("a", record(0, 1.0));
        buf.insert("a", record(1, 2.0));
        buf.insert("b", record(2, 3.0));
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn clear_empties_buffer() {
        let buf = HistoryBuffer::new(10);
        buf.insert("x", record(0, 0.0));
        assert_eq!(buf.len(), 1);
        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn multiple_keys_independent() {
        let buf = HistoryBuffer::new(5);
        buf.insert("motor_left", record(1, 100.0));
        buf.insert("motor_right", record(2, 200.0));

        assert_eq!(buf.get("motor_left").len(), 1);
        assert_eq!(buf.get("motor_right").len(), 1);
        assert!((buf.get("motor_left")[0].value - 100.0).abs() < 1e-9);
        assert!((buf.get("motor_right")[0].value - 200.0).abs() < 1e-9);
    }
}
