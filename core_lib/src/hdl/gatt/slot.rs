//! Bounded coherent long-read snapshots. Eviction fails a continuation, never splices it.
use std::{collections::VecDeque, sync::Arc};

use bluer::Address;

const READ_LIMIT: usize = 32;

pub(super) struct SlotValue {
    current: Arc<[u8]>,
    reads: VecDeque<(Address, Arc<[u8]>)>,
    coherent: bool,
}
impl SlotValue {
    pub fn new(bytes: Vec<u8>, coherent: bool) -> Self {
        Self {
            current: bytes.into(),
            reads: VecDeque::new(),
            coherent,
        }
    }
    pub fn install(&mut self, bytes: Vec<u8>) {
        self.current = bytes.into();
    }
    pub fn read(&mut self, peer: Address, offset: usize) -> Option<Vec<u8>> {
        if !self.coherent {
            return Some(self.current.get(offset..).unwrap_or_default().to_vec());
        }
        if offset == 0 {
            self.reads.retain(|(key, _)| *key != peer);
            if self.reads.len() == READ_LIMIT {
                self.reads.pop_front();
            }
            self.reads.push_back((peer, self.current.clone()));
        }
        let index = self.reads.iter().position(|(key, _)| *key == peer)?;
        let bytes = self.reads[index].1.clone();
        let response = bytes.get(offset..)?.to_vec();
        if offset == bytes.len() {
            self.reads.remove(index);
        }
        Some(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(value: u8) -> Address {
        format!("00:00:00:00:00:{value:02X}").parse().unwrap()
    }

    #[test]
    fn given_legacy_gatt_slot_when_offset_or_advertisement_changes_then_current_bytes_are_read_without_peer_state() {
        // Arrange: the old non-coherent advertising contract never pins a peer.
        let mut slot = SlotValue::new(b"Old".to_vec(), false);

        // Act / Assert: out-of-range reads are empty, not a cached or stale tail.
        for offset in [0, 1, 2, 3, 4, usize::MAX] {
            let expected = b"Old".get(offset..).unwrap_or_default();
            assert_eq!(slot.read(peer(1), offset).unwrap(), expected, "offset={offset}");
            assert_eq!(slot.read(peer(2), offset).unwrap(), expected, "offset={offset}");
            assert!(slot.reads.is_empty());
        }

        slot.install(b"New revision".to_vec());
        assert_eq!(slot.read(peer(1), 1).unwrap(), b"ew revision");
        assert_eq!(slot.read(peer(2), 0).unwrap(), b"New revision");
        assert!(slot.reads.is_empty());
        slot.install(Vec::new());
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"");
        assert_eq!(slot.read(peer(2), usize::MAX).unwrap(), b"");
        assert!(slot.reads.is_empty());
    }

    #[test]
    fn given_coherent_unprimed_or_empty_slot_when_peer_reads_then_continuation_needs_owned_snapshot() {
        // Arrange / Assert: a peer cannot request bytes without slot-zero setup.
        let mut slot = SlotValue::new(b"Hello".to_vec(), true);
        for offset in [1, 5, 6, usize::MAX] {
            assert_eq!(slot.read(peer(1), offset), None, "offset={offset}");
            assert!(slot.reads.is_empty());
        }
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"Hello");
        assert_eq!(slot.read(peer(2), 1), None);
        assert_eq!(slot.read(peer(1), 6), None);
        // A malformed overrun cannot leak current data or consume a valid snapshot.
        assert_eq!(slot.read(peer(1), 1).unwrap(), b"ello");
        assert_eq!(slot.read(peer(1), 5).unwrap(), b"");
        assert_eq!(slot.read(peer(1), 1), None);
        assert!(slot.reads.is_empty());

        // Empty advertising data still requires a fresh slot-zero request.
        slot.install(Vec::new());
        assert_eq!(slot.read(peer(1), 1), None);
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"");
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"");
        assert_eq!(slot.read(peer(1), usize::MAX), None);
        assert!(slot.reads.is_empty());
    }

    #[test]
    fn given_coherent_peer_restart_when_advertisement_rotates_then_no_peer_splices_or_replays_a_foreign_epoch() {
        // Arrange: each peer owns a distinct coherent long-read snapshot.
        let mut slot = SlotValue::new(b"first".to_vec(), true);
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"first");
        assert_eq!(slot.read(peer(2), 0).unwrap(), b"first");
        slot.install(b"later".to_vec());

        // Act: only peer 1 explicitly restarts its own slot-zero read.
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"later");

        // Assert: the other peer remains pinned to the original revision.
        assert_eq!(slot.reads.len(), 2);
        assert_eq!(slot.read(peer(1), 1).unwrap(), b"ater");
        assert_eq!(slot.read(peer(2), 1).unwrap(), b"irst");
        assert_eq!(slot.read(peer(1), 5).unwrap(), b"");
        assert_eq!(slot.read(peer(1), 1), None);
        assert_eq!(slot.read(peer(2), 5).unwrap(), b"");
        assert_eq!(slot.read(peer(2), 1), None);
        assert_eq!(slot.reads.len(), 0);
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"later");
    }

    #[test]
    fn given_coherent_slot_capacity_when_thirty_third_peer_arrives_then_oldest_is_evicted_without_dropping_other_owners() {
        // Arrange: all 32 admitted owners retain the same old revision.
        let mut slot = SlotValue::new(b"old".to_vec(), true);
        for index in 0..READ_LIMIT {
            assert_eq!(slot.read(peer(index as u8), 0).unwrap(), b"old");
        }
        assert_eq!(slot.reads.len(), READ_LIMIT);

        // Act: peer 33 evicts the oldest owner, without growing the bounded cache.
        assert_eq!(slot.read(peer(READ_LIMIT as u8), 0).unwrap(), b"old");
        assert_eq!(slot.reads.len(), READ_LIMIT);
        assert_eq!(slot.read(peer(0), 1), None);
        slot.install(b"new".to_vec());
        // Refreshing an existing owner must not evict any other owner.
        assert_eq!(slot.read(peer(2), 0).unwrap(), b"new");
        assert_eq!(slot.reads.len(), READ_LIMIT);

        // Assert: pinned peers cannot be silently moved to the newer revision.
        assert_eq!(slot.read(peer(1), 1).unwrap(), b"ld");
        assert_eq!(slot.read(peer(2), 1).unwrap(), b"ew");
        assert_eq!(slot.read(peer(READ_LIMIT as u8), 1).unwrap(), b"ld");
        assert_eq!(slot.read(peer(31), 1).unwrap(), b"ld");
        assert_eq!(slot.read(peer(0), 1), None);
        assert_eq!(slot.reads.len(), READ_LIMIT);
    }

    #[test]
    fn given_long_read_when_name_changes_then_continuation_uses_original_bytes() {
        let mut slot = SlotValue::new(b"Original name".to_vec(), true);
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"Original name");
        slot.install(b"New name".to_vec());
        assert_eq!(slot.read(peer(1), 4).unwrap(), b"inal name");
        assert_eq!(slot.read(peer(2), 0).unwrap(), b"New name");
        assert_eq!(slot.read(peer(2), 4).unwrap(), b"name");
        assert_eq!(slot.read(peer(3), 4), None);
        assert_eq!(slot.read(peer(1), 14), None);
        assert_eq!(slot.read(peer(1), 13).unwrap(), b"");
        assert_eq!(slot.read(peer(1), 4), None);
        assert_eq!(slot.read(peer(1), 0).unwrap(), b"New name");
    }

    #[test]
    fn given_bounded_read_slots_when_old_slot_is_evicted_then_continuation_fails_without_splicing()
    {
        let mut slot = SlotValue::new(b"Original".to_vec(), true);
        for index in 0..=READ_LIMIT as u8 {
            slot.read(peer(index), 0).unwrap();
        }
        assert_eq!(slot.reads.len(), READ_LIMIT);
        slot.install(b"Changed".to_vec());
        assert_eq!(slot.read(peer(0), 2), None);
        assert_eq!(slot.read(peer(1), 2).unwrap(), b"iginal");
        for _ in 0..64 {
            slot.read(peer(2), 0).unwrap();
        }
        assert_eq!(slot.reads.len(), READ_LIMIT);
        assert_eq!(slot.read(peer(2), 2).unwrap(), b"anged");
        let mut legacy = SlotValue::new(b"Static".to_vec(), false);
        assert_eq!(legacy.read(peer(0), 2).unwrap(), b"atic");
        assert_eq!(legacy.read(peer(0), 100).unwrap(), b"");
        assert!(legacy.reads.is_empty());
    }
}
