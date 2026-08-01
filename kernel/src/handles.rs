use alloc::alloc::{Layout, alloc};
use alloc::boxed::Box;
use core::ptr::{self, NonNull};

use bndr_abi::{HandleValue, Rights};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleError {
    InvalidHandle,
    AccessDenied,
    RightsEscalation,
    TableFull,
}

#[derive(Debug)]
struct HandleEntry<T> {
    object: T,
    rights: Rights,
}

#[derive(Debug)]
struct HandleSlot<T> {
    generation: u32,
    retired: bool,
    reserved: bool,
    entry: Option<HandleEntry<T>>,
}

/// A fixed-capacity, generational process handle table.
///
/// Low 8 handle bits encode `slot + 1`; high 24 bits encode a nonzero
/// generation. This supports up to 255 live slots while making deliberate
/// generation exhaustion 256 times harder than a 16/16 split. A slot is still
/// permanently retired rather than wrapping, so a stale integer can never
/// become valid again through ABA reuse.
#[derive(Debug)]
pub struct HandleTable<T, const N: usize> {
    slots: [HandleSlot<T>; N],
    len: usize,
}

/// An object and its unchanged rights after the source handle has committed a
/// move. The value is intentionally opaque: only a reserved destination slot
/// can turn it back into a live handle.
#[derive(Debug)]
pub struct OwnedHandle<T> {
    entry: HandleEntry<T>,
}

impl<T> OwnedHandle<T> {
    pub fn new(object: T, rights: Rights) -> Self {
        Self {
            entry: HandleEntry { object, rights },
        }
    }

    pub const fn rights(&self) -> Rights {
        self.entry.rights
    }

    pub const fn object(&self) -> &T {
        &self.entry.object
    }

    /// Decomposes the transport value for a channel representation that stores
    /// the object and rights separately. Ownership of both parts remains
    /// move-only.
    pub fn into_parts(self) -> (T, Rights) {
        (self.entry.object, self.entry.rights)
    }
}

/// A source-side move transaction. Dropping it rolls the entry back into the
/// same slot and generation, so every path except `commit` preserves the exact
/// original raw handle.
#[must_use = "dropping a pending handle transfer rolls it back"]
pub struct PendingHandleTransfer<'a, T, const N: usize> {
    table: &'a mut HandleTable<T, N>,
    index: usize,
    generation: u32,
    entry: Option<HandleEntry<T>>,
}

/// A source-side close transaction. The entry is hidden and its slot reserved
/// while the caller performs any object-specific teardown. Dropping the guard
/// restores the exact raw handle; `commit` is the only generation-changing
/// operation.
#[must_use = "dropping a pending handle close rolls it back"]
pub struct PendingHandleClose<'a, T, const N: usize> {
    table: &'a mut HandleTable<T, N>,
    index: usize,
    generation: u32,
    entry: Option<HandleEntry<T>>,
}

impl<T, const N: usize> PendingHandleClose<'_, T, N> {
    pub fn object(&self) -> &T {
        &self
            .entry
            .as_ref()
            .unwrap_or_else(|| panic!("completed handle close lost its entry"))
            .object
    }

    pub fn commit(mut self) -> T {
        let entry = self
            .entry
            .take()
            .unwrap_or_else(|| panic!("handle close committed twice"));
        self.table
            .commit_source_removal(self.index, self.generation);
        entry.object
    }

    pub fn rollback(mut self) -> HandleValue {
        let entry = self
            .entry
            .take()
            .unwrap_or_else(|| panic!("handle close rolled back twice"));
        self.table
            .rollback_source_transfer(self.index, self.generation, entry);
        encode(self.index, self.generation)
    }
}

impl<T, const N: usize> Drop for PendingHandleClose<'_, T, N> {
    fn drop(&mut self) {
        if let Some(entry) = self.entry.take() {
            self.table
                .rollback_source_transfer(self.index, self.generation, entry);
        }
    }
}

impl<T, const N: usize> PendingHandleTransfer<'_, T, N> {
    pub fn original_handle(&self) -> HandleValue {
        encode(self.index, self.generation)
    }

    pub fn rights(&self) -> Rights {
        self.entry
            .as_ref()
            .unwrap_or_else(|| panic!("completed handle transfer lost its entry"))
            .rights
    }

    pub fn object(&self) -> &T {
        &self
            .entry
            .as_ref()
            .unwrap_or_else(|| panic!("completed handle transfer lost its entry"))
            .object
    }

    /// Invalidates the source handle and yields its move-only payload. This is
    /// the commit point: the source generation advances (or retires) before the
    /// payload can be published by a channel.
    pub fn commit(mut self) -> OwnedHandle<T> {
        let entry = self
            .entry
            .take()
            .unwrap_or_else(|| panic!("handle transfer committed twice"));
        self.table
            .commit_source_transfer(self.index, self.generation);
        OwnedHandle { entry }
    }

    /// Explicitly restores the source entry and returns the unchanged raw
    /// handle. Dropping the transaction has the same restoration semantics.
    pub fn rollback(mut self) -> HandleValue {
        let entry = self
            .entry
            .take()
            .unwrap_or_else(|| panic!("handle transfer rolled back twice"));
        self.table
            .rollback_source_transfer(self.index, self.generation, entry);
        encode(self.index, self.generation)
    }
}

impl<T, const N: usize> Drop for PendingHandleTransfer<'_, T, N> {
    fn drop(&mut self) {
        if let Some(entry) = self.entry.take() {
            self.table
                .rollback_source_transfer(self.index, self.generation, entry);
        }
    }
}

/// One destination slot reserved before a channel message is removed. Dropping
/// the reservation releases capacity without changing its generation.
#[must_use = "dropping a handle-slot reservation releases it"]
pub struct HandleSlotReservation<'a, T, const N: usize> {
    table: &'a mut HandleTable<T, N>,
    index: Option<usize>,
    generation: u32,
}

impl<T, const N: usize> HandleSlotReservation<'_, T, N> {
    /// Returns the raw handle that a successful insertion will publish. This
    /// is read-only: observing a reservation neither commits it nor advances
    /// the slot generation.
    pub fn prospective_handle(&self) -> HandleValue {
        let index = self
            .index
            .unwrap_or_else(|| panic!("completed handle reservation has no prospective handle"));
        encode(index, self.generation)
    }

    /// Inserts an already committed transfer. The operation is infallible
    /// because the slot was reserved before the channel transaction began.
    pub fn insert(self, transferred: OwnedHandle<T>) -> HandleValue {
        let (object, rights) = transferred.into_parts();
        self.insert_parts(object, rights)
    }

    /// Variant for channel messages that carry their move-only object and
    /// rights as separate fields.
    pub fn insert_parts(mut self, object: T, rights: Rights) -> HandleValue {
        let index = self
            .index
            .take()
            .unwrap_or_else(|| panic!("handle reservation used twice"));
        self.table
            .insert_reserved_at(index, self.generation, HandleEntry { object, rights })
    }

    pub fn release(mut self) {
        let index = self
            .index
            .take()
            .unwrap_or_else(|| panic!("handle reservation released twice"));
        self.table.release_reserved_at(index, self.generation);
    }
}

impl<T, const N: usize> Drop for HandleSlotReservation<'_, T, N> {
    fn drop(&mut self) {
        if let Some(index) = self.index.take() {
            self.table.release_reserved_at(index, self.generation);
        }
    }
}

impl<T, const N: usize> HandleTable<T, N> {
    pub fn new() -> Self {
        assert!(N != 0 && N <= u8::MAX as usize);
        Self {
            slots: core::array::from_fn(|_| HandleSlot {
                generation: 1,
                retired: false,
                reserved: false,
                entry: None,
            }),
            len: 0,
        }
    }

    /// Fallibly initializes the fixed slot array directly in heap storage.
    /// This avoids materializing a process-sized table on a bounded kernel
    /// exception stack during process creation.
    pub fn try_new_boxed() -> Option<Box<Self>> {
        if N == 0 || N > u8::MAX as usize {
            return None;
        }
        let layout = Layout::new::<Self>();
        let table = NonNull::new(unsafe { alloc(layout) }.cast::<Self>())?;
        unsafe {
            let slots = ptr::addr_of_mut!((*table.as_ptr()).slots).cast::<HandleSlot<T>>();
            for index in 0..N {
                slots.add(index).write(HandleSlot {
                    generation: 1,
                    retired: false,
                    reserved: false,
                    entry: None,
                });
            }
            ptr::addr_of_mut!((*table.as_ptr()).len).write(0);
            Some(Box::from_raw(table.as_ptr()))
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterates over live entries without exposing slot generations or
    /// allowing mutation. Process lifecycle validation uses this to prove a
    /// capability topology instead of merely counting handles.
    pub fn live_entries(&self) -> impl Iterator<Item = (&T, Rights)> {
        self.slots.iter().filter_map(|slot| {
            slot.entry
                .as_ref()
                .map(|entry| (&entry.object, entry.rights))
        })
    }

    pub fn available(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !slot.retired && !slot.reserved && slot.entry.is_none())
            .count()
    }

    /// Reserves destination capacity without allocating or consuming an
    /// object. A later `insert` cannot fail; every other path releases the slot
    /// when the returned guard is dropped.
    pub fn reserve_slot(&mut self) -> Result<HandleSlotReservation<'_, T, N>, HandleError> {
        let index = self
            .empty_slot_indices()
            .next()
            .ok_or(HandleError::TableFull)?;
        let slot = &mut self.slots[index];
        debug_assert!(!slot.retired && !slot.reserved && slot.entry.is_none());
        slot.reserved = true;
        Ok(HandleSlotReservation {
            generation: slot.generation,
            table: self,
            index: Some(index),
        })
    }

    pub fn insert(&mut self, object: T, rights: Rights) -> Result<HandleValue, HandleError> {
        let index = self
            .empty_slot_indices()
            .next()
            .ok_or(HandleError::TableFull)?;
        Ok(self.insert_at(index, object, rights))
    }

    /// Inserts a move-only transport value without losing it when the table is
    /// full. Capacity is the only fallible condition.
    pub fn try_insert_owned(
        &mut self,
        owned: OwnedHandle<T>,
    ) -> Result<HandleValue, OwnedHandle<T>> {
        let Some(index) = self.empty_slot_indices().next() else {
            return Err(owned);
        };
        let (object, rights) = owned.into_parts();
        Ok(self.insert_at(index, object, rights))
    }

    pub fn insert_pair(
        &mut self,
        first: T,
        second: T,
        rights: Rights,
    ) -> Result<(HandleValue, HandleValue), HandleError> {
        let (first_index, second_index) = {
            let mut empty = self.empty_slot_indices();
            let first_index = empty.next().ok_or(HandleError::TableFull)?;
            let second_index = empty.next().ok_or(HandleError::TableFull)?;
            (first_index, second_index)
        };
        let first_handle = self.insert_at(first_index, first, rights);
        let second_handle = self.insert_at(second_index, second, rights);
        Ok((first_handle, second_handle))
    }

    pub fn get(&self, handle: HandleValue, required: Rights) -> Result<&T, HandleError> {
        let entry = self.entry(handle)?;
        if !entry.rights.contains(required) {
            return Err(HandleError::AccessDenied);
        }
        Ok(&entry.object)
    }

    pub fn rights(&self, handle: HandleValue) -> Result<Rights, HandleError> {
        Ok(self.entry(handle)?.rights)
    }

    /// Immediately commits a move out of this table. Both the caller's
    /// required rights and `TRANSFER` are checked before any mutation.
    pub fn take_owned(
        &mut self,
        handle: HandleValue,
        required: Rights,
    ) -> Result<OwnedHandle<T>, HandleError> {
        let pending = self.begin_transfer_requiring(handle, required)?;
        Ok(pending.commit())
    }

    /// Begins a move-only transfer. Decoding, liveness, and `TRANSFER` rights
    /// are all checked before the table is changed. Until commit, the source
    /// slot is reserved against reuse and the guard owns rollback.
    pub fn begin_transfer(
        &mut self,
        handle: HandleValue,
    ) -> Result<PendingHandleTransfer<'_, T, N>, HandleError> {
        self.begin_transfer_requiring(handle, Rights::NONE)
    }

    /// Begins an object-specific close transaction without requiring TRANSFER
    /// rights. The hidden entry remains owned by the same table and is
    /// restored automatically unless teardown commits successfully.
    pub fn begin_close(
        &mut self,
        handle: HandleValue,
    ) -> Result<PendingHandleClose<'_, T, N>, HandleError> {
        let (index, generation) = decode(handle).ok_or(HandleError::InvalidHandle)?;
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(HandleError::InvalidHandle)?;
        if slot.retired || slot.reserved || slot.generation != generation {
            return Err(HandleError::InvalidHandle);
        }
        let entry = slot.entry.take().ok_or(HandleError::InvalidHandle)?;
        slot.reserved = true;
        Ok(PendingHandleClose {
            table: self,
            index,
            generation,
            entry: Some(entry),
        })
    }

    fn begin_transfer_requiring(
        &mut self,
        handle: HandleValue,
        required: Rights,
    ) -> Result<PendingHandleTransfer<'_, T, N>, HandleError> {
        let (index, generation) = decode(handle).ok_or(HandleError::InvalidHandle)?;
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(HandleError::InvalidHandle)?;
        if slot.retired || slot.reserved || slot.generation != generation {
            return Err(HandleError::InvalidHandle);
        }
        let entry = slot.entry.as_ref().ok_or(HandleError::InvalidHandle)?;
        if !entry.rights.contains(Rights::TRANSFER) || !entry.rights.contains(required) {
            return Err(HandleError::AccessDenied);
        }
        let entry = slot
            .entry
            .take()
            .unwrap_or_else(|| panic!("validated transfer source lost its entry"));
        slot.reserved = true;
        Ok(PendingHandleTransfer {
            table: self,
            index,
            generation,
            entry: Some(entry),
        })
    }

    pub fn close(&mut self, handle: HandleValue) -> Result<T, HandleError> {
        let (index, generation) = decode(handle).ok_or(HandleError::InvalidHandle)?;
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(HandleError::InvalidHandle)?;
        if slot.retired || slot.reserved || slot.generation != generation {
            return Err(HandleError::InvalidHandle);
        }
        let entry = slot.entry.take().ok_or(HandleError::InvalidHandle)?;
        self.len -= 1;
        advance_or_retire(slot);
        Ok(entry.object)
    }

    fn insert_at(&mut self, index: usize, object: T, rights: Rights) -> HandleValue {
        let slot = &mut self.slots[index];
        debug_assert!(
            !slot.retired && !slot.reserved && slot.entry.is_none() && slot.generation != 0
        );
        slot.entry = Some(HandleEntry { object, rights });
        self.len += 1;
        encode(index, slot.generation)
    }

    fn commit_source_transfer(&mut self, index: usize, generation: u32) {
        self.commit_source_removal(index, generation);
    }

    fn commit_source_removal(&mut self, index: usize, generation: u32) {
        let slot = self
            .slots
            .get_mut(index)
            .unwrap_or_else(|| panic!("transfer source slot escaped its table"));
        if slot.retired
            || !slot.reserved
            || slot.generation != generation
            || slot.entry.is_some()
            || self.len == 0
        {
            panic!("transfer source reservation was corrupted before commit");
        }
        slot.reserved = false;
        self.len -= 1;
        advance_or_retire(slot);
    }

    fn rollback_source_transfer(&mut self, index: usize, generation: u32, entry: HandleEntry<T>) {
        let slot = self
            .slots
            .get_mut(index)
            .unwrap_or_else(|| panic!("transfer source slot escaped its table"));
        if slot.retired || !slot.reserved || slot.generation != generation || slot.entry.is_some() {
            panic!("transfer source reservation was corrupted before rollback");
        }
        slot.entry = Some(entry);
        slot.reserved = false;
    }

    fn insert_reserved_at(
        &mut self,
        index: usize,
        generation: u32,
        entry: HandleEntry<T>,
    ) -> HandleValue {
        let slot = self
            .slots
            .get_mut(index)
            .unwrap_or_else(|| panic!("destination reservation escaped its table"));
        if slot.retired || !slot.reserved || slot.generation != generation || slot.entry.is_some() {
            panic!("destination handle reservation was corrupted before insertion");
        }
        slot.entry = Some(entry);
        slot.reserved = false;
        self.len += 1;
        encode(index, generation)
    }

    fn release_reserved_at(&mut self, index: usize, generation: u32) {
        let slot = self
            .slots
            .get_mut(index)
            .unwrap_or_else(|| panic!("destination reservation escaped its table"));
        if slot.retired || !slot.reserved || slot.generation != generation || slot.entry.is_some() {
            panic!("destination handle reservation was corrupted before release");
        }
        slot.reserved = false;
    }

    fn entry(&self, handle: HandleValue) -> Result<&HandleEntry<T>, HandleError> {
        let (index, generation) = decode(handle).ok_or(HandleError::InvalidHandle)?;
        let slot = self.slots.get(index).ok_or(HandleError::InvalidHandle)?;
        if slot.retired || slot.reserved || slot.generation != generation {
            return Err(HandleError::InvalidHandle);
        }
        slot.entry.as_ref().ok_or(HandleError::InvalidHandle)
    }

    fn empty_slot_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            (!slot.retired && !slot.reserved && slot.entry.is_none()).then_some(index)
        })
    }
}

impl<T: Clone, const N: usize> HandleTable<T, N> {
    pub fn duplicate(
        &mut self,
        handle: HandleValue,
        requested: Rights,
    ) -> Result<HandleValue, HandleError> {
        let entry = self.entry(handle)?;
        if !entry.rights.contains(Rights::DUPLICATE) {
            return Err(HandleError::AccessDenied);
        }
        if !requested.is_subset_of(entry.rights) {
            return Err(HandleError::RightsEscalation);
        }
        let object = entry.object.clone();
        self.insert(object, requested)
    }
}

impl<T, const N: usize> Default for HandleTable<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

const MAX_GENERATION: u32 = 0x00ff_ffff;

fn advance_or_retire<T>(slot: &mut HandleSlot<T>) {
    debug_assert!(!slot.reserved && slot.entry.is_none());
    if slot.generation == MAX_GENERATION {
        slot.retired = true;
        slot.generation = 0;
    } else {
        slot.generation += 1;
    }
}

fn encode(index: usize, generation: u32) -> HandleValue {
    let slot = u8::try_from(index + 1).expect("handle table capacity was validated");
    HandleValue::from_raw((generation << 8) | u32::from(slot))
}

fn decode(handle: HandleValue) -> Option<(usize, u32)> {
    let raw = handle.raw();
    let slot = (raw & 0xff) as u8;
    let generation = raw >> 8;
    if slot == 0 || generation == 0 {
        return None;
    }
    Some((usize::from(slot - 1), generation))
}

#[cfg(test)]
mod tests {
    use bndr_abi::Rights;

    use super::{HandleError, HandleTable, MAX_GENERATION};

    #[test]
    fn close_invalidates_stale_values_before_slot_reuse() {
        let mut table = HandleTable::<u64, 2>::new();
        let old = table.insert(10, Rights::READ).unwrap();
        assert_eq!(table.close(old), Ok(10));
        let fresh = table.insert(20, Rights::READ).unwrap();
        assert_ne!(old, fresh);
        assert_eq!(
            table.get(old, Rights::READ),
            Err(HandleError::InvalidHandle)
        );
        assert_eq!(table.get(fresh, Rights::READ), Ok(&20));
    }

    #[test]
    fn pending_close_rolls_back_exactly_and_commits_only_after_teardown() {
        let mut table = HandleTable::<u64, 2>::new();
        let handle = table.insert(42, Rights::READ).unwrap();
        {
            let pending = table.begin_close(handle).unwrap();
            assert_eq!(*pending.object(), 42);
        }
        assert_eq!(table.get(handle, Rights::READ), Ok(&42));
        assert_eq!(table.len(), 1);

        let pending = table.begin_close(handle).unwrap();
        assert_eq!(pending.commit(), 42);
        assert_eq!(table.len(), 0);
        assert_eq!(
            table.get(handle, Rights::READ),
            Err(HandleError::InvalidHandle)
        );
    }

    #[test]
    fn duplicate_can_only_reduce_rights() {
        let mut table = HandleTable::<u64, 4>::new();
        let original = table
            .insert(7, Rights::CHANNEL_DEFAULT)
            .expect("original handle");
        let reduced = table.duplicate(original, Rights::READ).unwrap();
        assert_eq!(table.rights(reduced), Ok(Rights::READ));
        assert_eq!(
            table.get(reduced, Rights::WRITE),
            Err(HandleError::AccessDenied)
        );
        assert_eq!(
            table.duplicate(reduced, Rights::READ),
            Err(HandleError::AccessDenied)
        );
        assert_eq!(
            table.duplicate(original, Rights::ADMIN),
            Err(HandleError::RightsEscalation)
        );
    }

    #[test]
    fn transfer_permission_failure_preserves_the_source() {
        let mut table = HandleTable::<u64, 2>::new();
        let handle = table.insert(41, Rights::READ).unwrap();

        assert!(matches!(
            table.take_owned(handle, Rights::READ),
            Err(HandleError::AccessDenied)
        ));
        assert_eq!(table.len(), 1);
        assert_eq!(table.rights(handle), Ok(Rights::READ));
        assert_eq!(table.get(handle, Rights::READ), Ok(&41));

        let transferable = table.insert(42, Rights::TRANSFER).unwrap();
        assert!(matches!(
            table.take_owned(transferable, Rights::WRITE),
            Err(HandleError::AccessDenied)
        ));
        assert_eq!(table.rights(transferable), Ok(Rights::TRANSFER));
        assert_eq!(table.get(transferable, Rights::NONE), Ok(&42));
    }

    #[test]
    fn full_destination_and_dropped_transfer_restore_the_source() {
        let mut source = HandleTable::<u64, 1>::new();
        let original = source.insert(52, transferable_rights()).unwrap();
        let mut destination = HandleTable::<u64, 1>::new();
        let occupied = destination.insert(7, Rights::READ).unwrap();

        {
            let pending = source.begin_transfer(original).unwrap();
            assert!(matches!(
                destination.reserve_slot(),
                Err(HandleError::TableFull)
            ));
            drop(pending);
        }

        assert_eq!(source.len(), 1);
        assert_eq!(source.get(original, Rights::READ), Ok(&52));
        assert_eq!(source.rights(original), Ok(transferable_rights()));
        assert_eq!(destination.get(occupied, Rights::READ), Ok(&7));
    }

    #[test]
    fn explicit_transfer_rollback_restores_the_same_raw_handle() {
        let mut table = HandleTable::<u64, 2>::new();
        let original = table.insert(63, transferable_rights()).unwrap();

        let pending = table.begin_transfer(original).unwrap();
        assert_eq!(pending.original_handle(), original);
        assert_eq!(pending.rights(), transferable_rights());
        assert_eq!(pending.object(), &63);
        let restored = pending.rollback();

        assert_eq!(restored, original);
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(original, Rights::READ), Ok(&63));
        assert_eq!(table.rights(original), Ok(transferable_rights()));
    }

    #[test]
    fn committed_transfer_is_stale_at_source_and_infallible_at_destination() {
        let mut source = HandleTable::<u64, 2>::new();
        let original = source.insert(74, transferable_rights()).unwrap();
        let mut destination = HandleTable::<u64, 2>::new();

        let transferred = source.take_owned(original, Rights::READ).unwrap();
        assert_eq!(transferred.rights(), transferable_rights());
        assert_eq!(transferred.object(), &74);
        assert_eq!(source.len(), 0);
        assert_eq!(
            source.get(original, Rights::NONE),
            Err(HandleError::InvalidHandle)
        );

        let received = destination.try_insert_owned(transferred).unwrap();
        assert_eq!(destination.len(), 1);
        assert_eq!(destination.rights(received), Ok(transferable_rights()));
        assert_eq!(destination.get(received, Rights::READ), Ok(&74));
    }

    #[test]
    fn full_table_returns_the_same_owned_handle_without_loss() {
        let mut destination = HandleTable::<u64, 1>::new();
        destination.insert(1, Rights::READ).unwrap();
        let owned = super::OwnedHandle::new(75, transferable_rights());

        let returned = destination.try_insert_owned(owned).unwrap_err();
        assert_eq!(returned.object(), &75);
        assert_eq!(returned.rights(), transferable_rights());
        let (object, rights) = returned.into_parts();
        assert_eq!(object, 75);
        assert_eq!(rights, transferable_rights());
        assert_eq!(destination.len(), 1);
    }

    #[test]
    fn dropped_destination_reservation_releases_capacity() {
        let mut table = HandleTable::<u64, 1>::new();
        let generation = table.slots[0].generation;
        let reservation = table.reserve_slot().unwrap();
        let prospective = reservation.prospective_handle();
        drop(reservation);

        assert_eq!(table.slots[0].generation, generation);
        assert_eq!(table.available(), 1);
        let reservation = table.reserve_slot().unwrap();
        assert_eq!(reservation.prospective_handle(), prospective);
        let handle = reservation.insert_parts(85, Rights::READ);
        assert_eq!(handle, prospective);
        assert_eq!(table.get(handle, Rights::READ), Ok(&85));
    }

    #[test]
    fn pair_insertion_is_atomic_when_only_one_slot_remains() {
        let mut table = HandleTable::<u64, 2>::new();
        let existing = table.insert(1, Rights::READ).unwrap();
        assert_eq!(
            table.insert_pair(2, 3, Rights::READ),
            Err(HandleError::TableFull)
        );
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(existing, Rights::READ), Ok(&1));
    }

    #[test]
    fn retiring_max_generation_prevents_aba_wrap() {
        let mut table = HandleTable::<u64, 1>::new();
        table.slots[0].generation = MAX_GENERATION;
        let last = table.insert(9, Rights::READ).unwrap();
        assert_eq!(table.close(last), Ok(9));
        assert_eq!(table.available(), 0);
        assert_eq!(table.insert(10, Rights::READ), Err(HandleError::TableFull));
        assert_eq!(
            table.get(last, Rights::READ),
            Err(HandleError::InvalidHandle)
        );
    }

    #[test]
    fn transfer_commit_retires_max_generation_but_rollback_does_not() {
        let mut source = HandleTable::<u64, 1>::new();
        source.slots[0].generation = MAX_GENERATION;
        let last = source.insert(96, transferable_rights()).unwrap();

        let restored = source.begin_transfer(last).unwrap().rollback();
        assert_eq!(restored, last);
        assert!(!source.slots[0].retired);
        assert_eq!(source.get(last, Rights::READ), Ok(&96));

        let transferred = source.begin_transfer(last).unwrap().commit();
        assert!(source.slots[0].retired);
        assert_eq!(source.available(), 0);
        assert_eq!(
            source.get(last, Rights::NONE),
            Err(HandleError::InvalidHandle)
        );
        assert_eq!(
            source.insert(97, transferable_rights()),
            Err(HandleError::TableFull)
        );

        let mut destination = HandleTable::<u64, 1>::new();
        let received = destination.reserve_slot().unwrap().insert(transferred);
        assert_eq!(destination.get(received, Rights::READ), Ok(&96));
        assert_eq!(destination.rights(received), Ok(transferable_rights()));
    }

    #[test]
    fn rejects_zero_out_of_range_and_wrong_generation() {
        let table = HandleTable::<u64, 2>::new();
        for raw in [0, 1, 0x0000_0103, 0x0000_0201] {
            assert_eq!(
                table.get(bndr_abi::HandleValue::from_raw(raw), Rights::NONE),
                Err(HandleError::InvalidHandle)
            );
        }
    }

    #[test]
    fn boxed_constructor_initializes_slots_without_a_table_temporary() {
        let mut table = HandleTable::<u64, 32>::try_new_boxed().unwrap();
        assert_eq!(table.len(), 0);
        assert_eq!(table.available(), 32);
        let handle = table.insert(42, Rights::READ).unwrap();
        assert_eq!(table.get(handle, Rights::READ), Ok(&42));
        assert_eq!(table.close(handle), Ok(42));
        assert_eq!(table.available(), 32);
        assert!(HandleTable::<u64, 0>::try_new_boxed().is_none());
    }

    fn transferable_rights() -> Rights {
        Rights::from_bits(Rights::READ.bits() | Rights::WRITE.bits() | Rights::TRANSFER.bits())
            .unwrap()
    }
}
