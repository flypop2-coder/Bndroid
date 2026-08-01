/// Encodes one nonzero-generation process ID for a bounded process table.
///
/// The low 32 bits contain `slot + 1`; the high 32 bits contain the generation.
/// Returning `None` keeps malformed table capacities, slots, and generations
/// out of the runtime process table before an ID can be published.
pub const fn encode_process_id(slot: usize, generation: u32, capacity: usize) -> Option<u64> {
    if capacity == 0 || slot >= capacity || generation == 0 || slot >= u32::MAX as usize {
        return None;
    }
    Some(((generation as u64) << 32) | (slot as u64 + 1))
}

/// Decodes a process ID only when it belongs to the supplied bounded table.
pub const fn decode_process_id(raw: u64, capacity: usize) -> Option<(usize, u32)> {
    let encoded = (raw & 0xffff_ffff) as usize;
    let generation = (raw >> 32) as u32;
    if capacity == 0 || encoded == 0 || encoded > capacity || generation == 0 {
        None
    } else {
        Some((encoded - 1, generation))
    }
}

/// A retained process-exit result keyed by one generation-qualified PID.
///
/// Each dynamic process-table slot owns one `CompletionSlot`. A mismatched PID
/// can inspect but never consume that slot's retained value, and independent
/// slots can retain different child completions concurrently.
pub struct CompletionSlot<T> {
    entry: Option<(u64, T)>,
}

impl<T> CompletionSlot<T> {
    pub const fn new() -> Self {
        Self { entry: None }
    }

    pub fn is_pending(&self) -> bool {
        self.entry.is_some()
    }

    pub fn pending_target(&self) -> Option<u64> {
        self.entry.as_ref().map(|(target, _)| *target)
    }

    pub fn publish(&mut self, target: u64, value: T) -> Result<(), T> {
        if target == 0 || self.entry.is_some() {
            return Err(value);
        }
        self.entry = Some((target, value));
        Ok(())
    }

    pub fn take(&mut self, target: u64) -> Option<T> {
        if self.pending_target() != Some(target) {
            return None;
        }
        self.entry.take().map(|(_, value)| value)
    }
}

impl<T> Default for CompletionSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CompletionSlot, decode_process_id, encode_process_id};

    #[test]
    fn four_slot_process_ids_round_trip_and_reject_out_of_range_values() {
        for slot in 0..4 {
            let raw = encode_process_id(slot, 7, 4).unwrap();
            assert_eq!(decode_process_id(raw, 4), Some((slot, 7)));
        }
        assert_eq!(encode_process_id(4, 1, 4), None);
        assert_eq!(encode_process_id(0, 0, 4), None);
        assert_eq!(decode_process_id((1_u64 << 32) | 5, 4), None);
        assert_eq!(decode_process_id(1, 4), None);
    }

    #[test]
    fn mismatched_generation_cannot_consume_a_completion() {
        let mut slot = CompletionSlot::new();
        let first_pid = 0x0000_0001_0000_0002;
        let reused_pid = 0x0000_0002_0000_0002;
        assert_eq!(slot.publish(first_pid, 0xc11d_u64), Ok(()));
        assert_eq!(slot.take(reused_pid), None);
        assert_eq!(slot.pending_target(), Some(first_pid));
        assert_eq!(slot.take(first_pid), Some(0xc11d));
        assert!(!slot.is_pending());
    }

    #[test]
    fn rejects_zero_and_overwriting_an_unconsumed_completion() {
        let mut slot = CompletionSlot::new();
        assert_eq!(slot.publish(0, 1_u64), Err(1));
        assert_eq!(slot.publish(7, 2), Ok(()));
        assert_eq!(slot.publish(8, 3), Err(3));
        assert_eq!(slot.take(7), Some(2));
    }

    #[test]
    fn independent_dynamic_slots_retain_non_target_completions() {
        let manager = encode_process_id(1, 1, 4).unwrap();
        let provider = encode_process_id(2, 1, 4).unwrap();
        let client = encode_process_id(3, 1, 4).unwrap();
        let mut slots = [
            CompletionSlot::new(),
            CompletionSlot::new(),
            CompletionSlot::new(),
        ];
        assert_eq!(slots[0].publish(manager, 11_u64), Ok(()));
        assert_eq!(slots[1].publish(provider, 22_u64), Ok(()));
        assert_eq!(slots[2].publish(client, 33_u64), Ok(()));
        for (index, target) in [manager, provider, client].into_iter().enumerate() {
            for (slot_index, slot) in slots.iter_mut().enumerate() {
                if slot_index != index {
                    assert_eq!(slot.take(target), None);
                }
            }
        }
        assert_eq!(slots[2].take(client), Some(33));
        assert_eq!(slots[1].take(provider), Some(22));
        assert_eq!(slots[0].take(manager), Some(11));
    }
}
