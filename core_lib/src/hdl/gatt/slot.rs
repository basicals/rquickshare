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
