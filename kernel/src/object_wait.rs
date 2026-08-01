use bndr_abi::ObjectSignals;

pub const OBJECT_WAIT_MAX_ITEMS: usize = bndr_abi::OBJECT_WAIT_MANY_ARRAY_MAX_ITEMS;

/// Selects the syscall ABI that owns a wait token's eventual completion.
///
/// Item count is deliberately not used as a discriminator: the array ABI may
/// publish one or two items and must still complete through its array result
/// writer. Keeping the kind in the generation-qualified token also prevents a
/// delayed completion from consuming a republished wait with the same PID,
/// handles, masks, and deadline but a different ABI shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectWaitCompletionKind {
    Single,
    LegacyMany,
    Array,
}

/// One generation-qualified handle and its requested signal mask.
///
/// Construction is intentionally infallible so syscall decoding can assemble a
/// bounded request without allocation. `ObjectWaitSlot::publish_many` owns the
/// validation boundary and rejects zero handles or empty signal masks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectWaitItem {
    raw_handle: u64,
    requested: ObjectSignals,
}

impl ObjectWaitItem {
    const EMPTY: Self = Self {
        raw_handle: 0,
        requested: ObjectSignals::NONE,
    };

    pub const fn new(raw_handle: u64, requested: ObjectSignals) -> Self {
        Self {
            raw_handle,
            requested,
        }
    }

    pub const fn raw_handle(self) -> u64 {
        self.raw_handle
    }

    pub const fn requested(self) -> ObjectSignals {
        self.requested
    }
}

/// One bounded wait-any registration.
///
/// The item order is stable and is part of the token's identity. A deadline is
/// an absolute scheduler time; `None` means unbounded and `Some(0)` is valid so
/// the representation remains correct if the scheduler counter wraps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectWaitToken {
    epoch: u64,
    process_id: u64,
    items: [ObjectWaitItem; OBJECT_WAIT_MAX_ITEMS],
    count: u8,
    deadline: Option<u64>,
    completion_kind: ObjectWaitCompletionKind,
}

impl ObjectWaitToken {
    pub const fn epoch(self) -> u64 {
        self.epoch
    }

    pub const fn process_id(self) -> u64 {
        self.process_id
    }

    /// Compatibility accessor for the original one-item scheduler path.
    ///
    /// Every published token has at least one item, so this always returns the
    /// first request. New wait-any code should iterate over `items()` instead.
    pub const fn raw_handle(self) -> u64 {
        self.items[0].raw_handle()
    }

    /// Compatibility accessor for the original one-item scheduler path.
    pub const fn requested(self) -> ObjectSignals {
        self.items[0].requested()
    }

    pub fn items(&self) -> &[ObjectWaitItem] {
        &self.items[..self.count()]
    }

    pub const fn count(self) -> usize {
        self.count as usize
    }

    pub const fn deadline(self) -> Option<u64> {
        self.deadline
    }

    pub const fn completion_kind(self) -> ObjectWaitCompletionKind {
        self.completion_kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishError {
    InvalidKey,
    AlreadyPending,
    EpochExhausted,
}

/// One repeatable, bounded object-wait registration for a scheduler context.
///
/// PID and handles are generation-qualified. `take_exact_token` additionally
/// compares the monotonically increasing wait epoch, every item (including its
/// order and signal mask), and the absolute deadline. A delayed wake therefore
/// cannot consume a later wait after context, process-slot, or handle-slot
/// reuse. Duplicate handles are valid: a wait-any caller may request distinct
/// masks for the same object and retain its original item ordering.
pub struct ObjectWaitSlot {
    next_epoch: u64,
    token: Option<ObjectWaitToken>,
}

impl ObjectWaitSlot {
    pub const fn new() -> Self {
        Self {
            next_epoch: 0,
            token: None,
        }
    }

    /// Publishes the original, unbounded one-item wait shape.
    pub fn publish(
        &mut self,
        process_id: u64,
        raw_handle: u64,
        requested: ObjectSignals,
    ) -> Result<ObjectWaitToken, PublishError> {
        self.publish_with_kind(
            process_id,
            &[ObjectWaitItem::new(raw_handle, requested)],
            None,
            ObjectWaitCompletionKind::Single,
        )
    }

    /// Publishes the legacy allocation-free wait-any request containing at
    /// most [`bndr_abi::OBJECT_WAIT_MANY_ITEM_COUNT`] items.
    pub fn publish_many(
        &mut self,
        process_id: u64,
        items: &[ObjectWaitItem],
        deadline: Option<u64>,
    ) -> Result<ObjectWaitToken, PublishError> {
        if items.len() > bndr_abi::OBJECT_WAIT_MANY_ITEM_COUNT {
            return Err(PublishError::InvalidKey);
        }
        self.publish_with_kind(
            process_id,
            items,
            deadline,
            ObjectWaitCompletionKind::LegacyMany,
        )
    }

    /// Publishes the array wait-any ABI with one through eight items.
    pub fn publish_array(
        &mut self,
        process_id: u64,
        items: &[ObjectWaitItem],
        deadline: Option<u64>,
    ) -> Result<ObjectWaitToken, PublishError> {
        self.publish_with_kind(process_id, items, deadline, ObjectWaitCompletionKind::Array)
    }

    fn publish_with_kind(
        &mut self,
        process_id: u64,
        items: &[ObjectWaitItem],
        deadline: Option<u64>,
        completion_kind: ObjectWaitCompletionKind,
    ) -> Result<ObjectWaitToken, PublishError> {
        if process_id == 0
            || items.is_empty()
            || items.len() > OBJECT_WAIT_MAX_ITEMS
            || (completion_kind == ObjectWaitCompletionKind::Single && items.len() != 1)
            || items
                .iter()
                .any(|item| item.raw_handle() == 0 || item.requested().bits() == 0)
        {
            return Err(PublishError::InvalidKey);
        }
        if self.token.is_some() {
            return Err(PublishError::AlreadyPending);
        }
        let epoch = self
            .next_epoch
            .checked_add(1)
            .filter(|epoch| *epoch != 0)
            .ok_or(PublishError::EpochExhausted)?;

        let mut bounded_items = [ObjectWaitItem::EMPTY; OBJECT_WAIT_MAX_ITEMS];
        bounded_items[..items.len()].copy_from_slice(items);
        let token = ObjectWaitToken {
            epoch,
            process_id,
            items: bounded_items,
            count: items.len() as u8,
            deadline,
            completion_kind,
        };
        self.next_epoch = epoch;
        self.token = Some(token);
        Ok(token)
    }

    pub const fn token(&self) -> Option<ObjectWaitToken> {
        self.token
    }

    pub const fn is_pending(&self) -> bool {
        self.token.is_some()
    }

    pub const fn latest_epoch(&self) -> u64 {
        self.next_epoch
    }

    /// Compatibility consume path for the original one-item scheduler.
    ///
    /// It deliberately refuses a multi-item token even when the first item
    /// matches. Wait-any completion must use `take_exact_token`.
    pub fn take_exact(
        &mut self,
        epoch: u64,
        process_id: u64,
        raw_handle: u64,
    ) -> Option<ObjectWaitToken> {
        let token = self.token?;
        if token.completion_kind() != ObjectWaitCompletionKind::Single
            || token.count() != 1
            || token.epoch != epoch
            || token.process_id != process_id
            || token.raw_handle() != raw_handle
        {
            return None;
        }
        self.token.take()
    }

    /// Consumes only the exact currently published token.
    pub fn take_exact_token(&mut self, expected: &ObjectWaitToken) -> Option<ObjectWaitToken> {
        let token = self.token?;
        if token != *expected {
            return None;
        }
        self.token.take()
    }
}

impl Default for ObjectWaitSlot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use bndr_abi::ObjectSignals;

    use super::{
        OBJECT_WAIT_MAX_ITEMS, ObjectWaitCompletionKind, ObjectWaitItem, ObjectWaitSlot,
        PublishError,
    };

    #[test]
    fn exact_epoch_pid_and_handle_are_all_required_to_consume_one_item() {
        let mut slot = ObjectWaitSlot::new();
        let token = slot
            .publish(0x0000_0001_0000_0002, 0x101, ObjectSignals::READABLE)
            .unwrap();
        assert!(
            slot.take_exact(token.epoch() + 1, token.process_id(), token.raw_handle())
                .is_none()
        );
        assert!(
            slot.take_exact(token.epoch(), 0x0000_0002_0000_0002, token.raw_handle())
                .is_none()
        );
        assert!(
            slot.take_exact(token.epoch(), token.process_id(), 0x201)
                .is_none()
        );
        assert_eq!(
            slot.take_exact(token.epoch(), token.process_id(), token.raw_handle()),
            Some(token)
        );
        assert!(!slot.is_pending());
    }

    #[test]
    fn repeated_waits_get_distinct_epochs_and_cannot_overlap() {
        let mut slot = ObjectWaitSlot::new();
        let first = slot.publish(1, 0x101, ObjectSignals::WRITABLE).unwrap();
        assert_eq!(
            slot.publish(1, 0x101, ObjectSignals::READABLE),
            Err(PublishError::AlreadyPending)
        );
        assert_eq!(slot.take_exact_token(&first), Some(first));
        let second = slot.publish(1, 0x101, ObjectSignals::PEER_CLOSED).unwrap();
        assert!(second.epoch() > first.epoch());
        assert_eq!(slot.latest_epoch(), second.epoch());
    }

    #[test]
    fn stale_token_cannot_consume_republished_identical_wait_after_aba() {
        let mut slot = ObjectWaitSlot::new();
        let items = [ObjectWaitItem::new(0x101, ObjectSignals::READABLE)];
        let stale = slot.publish_many(7, &items, Some(42)).unwrap();
        assert_eq!(slot.take_exact_token(&stale), Some(stale));

        let current = slot.publish_many(7, &items, Some(42)).unwrap();
        assert_ne!(current.epoch(), stale.epoch());
        assert_eq!(slot.take_exact_token(&stale), None);
        assert_eq!(slot.token(), Some(current));
        assert_eq!(slot.take_exact_token(&current), Some(current));
    }

    #[test]
    fn wait_any_preserves_items_count_and_deadline() {
        let mut slot = ObjectWaitSlot::new();
        let items = [
            ObjectWaitItem::new(0x101, ObjectSignals::READABLE),
            ObjectWaitItem::new(0x202, ObjectSignals::SIGNALED),
        ];
        let token = slot.publish_many(9, &items, Some(1234)).unwrap();

        assert_eq!(OBJECT_WAIT_MAX_ITEMS, 8);
        assert_eq!(token.count(), 2);
        assert_eq!(token.items(), &items);
        assert_eq!(token.deadline(), Some(1234));
        assert_eq!(
            token.completion_kind(),
            ObjectWaitCompletionKind::LegacyMany
        );
        assert_eq!(token.raw_handle(), items[0].raw_handle());
        assert_eq!(token.requested(), items[0].requested());
        assert_eq!(slot.token(), Some(token));
        assert_eq!(slot.take_exact_token(&token), Some(token));
    }

    #[test]
    fn duplicate_handles_are_valid_and_keep_distinct_item_masks() {
        let mut slot = ObjectWaitSlot::new();
        let duplicate = [
            ObjectWaitItem::new(0x101, ObjectSignals::READABLE),
            ObjectWaitItem::new(0x101, ObjectSignals::PEER_CLOSED),
        ];
        let token = slot.publish_many(1, &duplicate, None).unwrap();
        assert_eq!(token.items(), &duplicate);
        assert_eq!(token.deadline(), None);

        // The compatibility path cannot partially identify a multi-item wait.
        assert_eq!(
            slot.take_exact(token.epoch(), token.process_id(), token.raw_handle()),
            None
        );
        assert_eq!(slot.take_exact_token(&token), Some(token));
    }

    #[test]
    fn exact_consume_includes_second_item_mask_order_and_deadline() {
        let first = ObjectWaitItem::new(0x101, ObjectSignals::READABLE);
        let second = ObjectWaitItem::new(0x202, ObjectSignals::SIGNALED);
        let mut slot = ObjectWaitSlot::new();
        let actual = slot.publish_many(3, &[first, second], Some(0)).unwrap();

        let mut different_item_slot = ObjectWaitSlot::new();
        let different_item = different_item_slot
            .publish_many(
                3,
                &[
                    first,
                    ObjectWaitItem::new(0x202, ObjectSignals::PEER_CLOSED),
                ],
                Some(0),
            )
            .unwrap();
        assert_eq!(slot.take_exact_token(&different_item), None);

        let mut different_order_slot = ObjectWaitSlot::new();
        let different_order = different_order_slot
            .publish_many(3, &[second, first], Some(0))
            .unwrap();
        assert_eq!(slot.take_exact_token(&different_order), None);

        let mut different_deadline_slot = ObjectWaitSlot::new();
        let different_deadline = different_deadline_slot
            .publish_many(3, &[first, second], None)
            .unwrap();
        assert_eq!(slot.take_exact_token(&different_deadline), None);
        assert_eq!(slot.token(), Some(actual));
        assert_eq!(slot.take_exact_token(&actual), Some(actual));
    }

    #[test]
    fn zero_absolute_deadline_is_valid() {
        let mut slot = ObjectWaitSlot::new();
        let token = slot
            .publish_many(
                1,
                &[ObjectWaitItem::new(1, ObjectSignals::SIGNALED)],
                Some(0),
            )
            .unwrap();
        assert_eq!(token.deadline(), Some(0));
    }

    #[test]
    fn array_wait_accepts_one_two_three_and_eight_items() {
        let all = core::array::from_fn::<_, OBJECT_WAIT_MAX_ITEMS, _>(|index| {
            ObjectWaitItem::new((index + 1) as u64, ObjectSignals::SIGNALED)
        });

        for count in [1, 2, 3, OBJECT_WAIT_MAX_ITEMS] {
            let mut slot = ObjectWaitSlot::new();
            let token = slot.publish_array(11, &all[..count], Some(99)).unwrap();
            assert_eq!(token.count(), count);
            assert_eq!(token.items(), &all[..count]);
            assert_eq!(token.completion_kind(), ObjectWaitCompletionKind::Array);
            assert_eq!(slot.take_exact_token(&token), Some(token));
        }
    }

    #[test]
    fn completion_kind_distinguishes_all_abis_with_identical_token_fields() {
        let items = [
            ObjectWaitItem::new(0x101, ObjectSignals::READABLE),
            ObjectWaitItem::new(0x202, ObjectSignals::PEER_CLOSED),
        ];

        for count in [1, 2] {
            let mut legacy_slot = ObjectWaitSlot::new();
            let legacy = legacy_slot.publish_many(3, &items[..count], None).unwrap();
            let mut array_slot = ObjectWaitSlot::new();
            let array = array_slot.publish_array(3, &items[..count], None).unwrap();

            assert_ne!(legacy, array);
            assert_eq!(
                legacy.completion_kind(),
                ObjectWaitCompletionKind::LegacyMany
            );
            assert_eq!(array.completion_kind(), ObjectWaitCompletionKind::Array);
            assert_eq!(legacy_slot.take_exact_token(&array), None);
            assert_eq!(legacy_slot.token(), Some(legacy));

            if count == 1 {
                let mut single_slot = ObjectWaitSlot::new();
                let single = single_slot
                    .publish(3, items[0].raw_handle(), items[0].requested())
                    .unwrap();
                assert_ne!(single, legacy);
                assert_ne!(single, array);
                assert_eq!(single.completion_kind(), ObjectWaitCompletionKind::Single);
                assert_eq!(
                    array_slot.take_exact(array.epoch(), array.process_id(), array.raw_handle()),
                    None
                );
                assert_eq!(array_slot.token(), Some(array));
            }
        }
    }

    #[test]
    fn stale_full_array_token_cannot_consume_identical_republication_after_aba() {
        let items = core::array::from_fn::<_, OBJECT_WAIT_MAX_ITEMS, _>(|index| {
            ObjectWaitItem::new((index + 1) as u64, ObjectSignals::SIGNALED)
        });
        let mut slot = ObjectWaitSlot::new();
        let stale = slot.publish_array(17, &items, Some(123)).unwrap();
        assert_eq!(slot.take_exact_token(&stale), Some(stale));

        let current = slot.publish_array(17, &items, Some(123)).unwrap();
        assert_ne!(stale.epoch(), current.epoch());
        assert_eq!(slot.take_exact_token(&stale), None);
        assert_eq!(slot.token(), Some(current));
        assert_eq!(slot.take_exact_token(&current), Some(current));
    }

    #[test]
    fn invalid_inputs_do_not_advance_or_publish() {
        let mut slot = ObjectWaitSlot::new();
        let valid = ObjectWaitItem::new(1, ObjectSignals::READABLE);
        let invalid_handle = ObjectWaitItem::new(0, ObjectSignals::READABLE);
        let invalid_signals = ObjectWaitItem::new(1, ObjectSignals::NONE);
        let valid_items = [valid];
        let empty_items: [ObjectWaitItem; 0] = [];
        let invalid_handle_items = [invalid_handle];
        let invalid_signal_items = [invalid_signals];
        let too_many_legacy = [valid; bndr_abi::OBJECT_WAIT_MANY_ITEM_COUNT + 1];

        for (process, items) in [
            (0, valid_items.as_slice()),
            (1, empty_items.as_slice()),
            (1, invalid_handle_items.as_slice()),
            (1, invalid_signal_items.as_slice()),
            (1, too_many_legacy.as_slice()),
        ] {
            assert_eq!(
                slot.publish_many(process, items, None),
                Err(PublishError::InvalidKey)
            );
        }
        let too_many_array = [valid; OBJECT_WAIT_MAX_ITEMS + 1];
        assert_eq!(
            slot.publish_array(1, &too_many_array, None),
            Err(PublishError::InvalidKey)
        );
        assert_eq!(slot.latest_epoch(), 0);
        assert!(!slot.is_pending());
    }
}
