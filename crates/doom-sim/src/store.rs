use std::{error::Error, fmt};

/// Store-scoped stable identity. Slot reuse never determines thinker order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ActorId {
    slot: u32,
    generation: u64,
}
impl ActorId {
    /// Constructs a candidate handle; every store operation validates it.
    pub const fn from_parts(slot: u32, generation: u64) -> Self {
        Self { slot, generation }
    }
    /// Logical slot, independent of heap location/capacity.
    pub const fn slot(self) -> u32 {
        self.slot
    }
    /// Generation; exhausted slots retire permanently instead of wrapping.
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// Actor ownership/scheduling failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// The configured slot limit or allocation was exhausted.
    Capacity,
    /// Slot limit exceeds the supported 65536-slot bound.
    InvalidLimit,
    /// Removed, stale, or fabricated identity.
    InvalidHandle,
    /// Caller-supplied traversal budget exhausted; completed callbacks remain applied.
    VisitLimit,
    /// Retirement observers cannot mutate the store.
    ReadOnlyVisit,
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "thinker store: {self:?}")
    }
}
impl Error for StoreError {}

/// An exact scheduling observation, in traversal order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visit {
    /// Invoke the live thinker callback (it may mark removal or append thinkers).
    Think(ActorId),
    /// Storage has been retired; callback is observational only.
    Retired(ActorId),
}

#[derive(Clone, Debug)]
pub(crate) struct Entry<T> {
    pub value: T,
    pub pending: bool,
    pub prev: Option<ActorId>,
    pub next: Option<ActorId>,
}
#[derive(Clone, Debug)]
pub(crate) struct Slot<T> {
    pub generation: u64,
    pub retired: bool,
    pub entry: Option<Entry<T>>,
}

pub(crate) struct ThinkerCursor {
    next: Option<ActorId>,
    previous: Option<ActorId>,
    remaining: usize,
}
/// Owned generational arena plus a separate safe linked processing order.
#[derive(Clone, Debug)]
pub struct ThinkerStore<T> {
    pub(crate) slots: Vec<Slot<T>>,
    pub(crate) head: Option<ActorId>,
    pub(crate) tail: Option<ActorId>,
    pub(crate) max_slots: u32,
}
impl<T> ThinkerStore<T> {
    /// Creates an empty store. The bound makes snapshots and callback budgets finite.
    pub fn new(max_slots: u32) -> Result<Self, StoreError> {
        if max_slots > 65536 {
            return Err(StoreError::InvalidLimit);
        }
        Ok(Self {
            slots: Vec::new(),
            head: None,
            tail: None,
            max_slots,
        })
    }
    pub(crate) fn entry(&self, id: ActorId) -> Option<&Entry<T>> {
        let slot = self.slots.get(id.slot as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.entry.as_ref()
    }
    fn entry_mut(&mut self, id: ActorId) -> Option<&mut Entry<T>> {
        let slot = self.slots.get_mut(id.slot as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.entry.as_mut()
    }
    /// Reads valid storage, including marked-but-not-yet-retired thinkers.
    pub fn get(&self, id: ActorId) -> Option<&T> {
        Some(&self.entry(id)?.value)
    }
    /// Mutates only storage addressed by a currently valid identity.
    pub fn get_mut(&mut self, id: ActorId) -> Option<&mut T> {
        Some(&mut self.entry_mut(id)?.value)
    }
    /// Pending removal remains readable until the traversal visits it.
    pub fn is_pending(&self, id: ActorId) -> Option<bool> {
        Some(self.entry(id)?.pending)
    }
    pub(crate) fn reserve_spawns(&mut self, count: usize) -> Result<(), StoreError> {
        let available = self
            .slots
            .iter()
            .filter(|s| s.entry.is_none() && !s.retired)
            .count()
            + self.max_slots as usize
            - self.slots.len();
        if count > available {
            return Err(StoreError::Capacity);
        }
        self.slots
            .try_reserve(count.min(self.max_slots as usize - self.slots.len()))
            .map_err(|_| StoreError::Capacity)
    }
    /// Appends to processing order, choosing the lowest nonretired reusable slot.
    pub fn spawn(&mut self, value: T) -> Result<ActorId, StoreError> {
        let index = if let Some(i) = self
            .slots
            .iter()
            .position(|s| s.entry.is_none() && !s.retired)
        {
            i
        } else {
            if self.slots.len() >= self.max_slots as usize {
                return Err(StoreError::Capacity);
            }
            self.slots
                .try_reserve(1)
                .map_err(|_| StoreError::Capacity)?;
            self.slots.push(Slot {
                generation: 1,
                retired: false,
                entry: None,
            });
            self.slots.len() - 1
        };
        let id = ActorId::from_parts(index as u32, self.slots[index].generation);
        self.slots[index].entry = Some(Entry {
            value,
            pending: false,
            prev: self.tail,
            next: None,
        });
        if let Some(tail) = self.tail {
            self.entry_mut(tail).ok_or(StoreError::InvalidHandle)?.next = Some(id);
        } else {
            self.head = Some(id);
        }
        self.tail = Some(id);
        Ok(id)
    }
    /// Marks removal without unlinking. Returns whether the mark changed.
    pub fn mark_remove(&mut self, id: ActorId) -> Result<bool, StoreError> {
        let entry = self.entry_mut(id).ok_or(StoreError::InvalidHandle)?;
        let changed = !entry.pending;
        entry.pending = true;
        Ok(changed)
    }
    /// Logical processing order, including pending thinkers; never arena order.
    pub fn order(&self) -> impl Iterator<Item = ActorId> + '_ {
        let mut cursor = self.head;
        std::iter::from_fn(move || {
            let id = cursor?;
            cursor = self.entry(id)?.next;
            Some(id)
        })
    }
    fn retire(&mut self, id: ActorId) -> Result<Option<ActorId>, StoreError> {
        let entry = self.entry(id).ok_or(StoreError::InvalidHandle)?;
        let (prev, next) = (entry.prev, entry.next);
        if let Some(prev) = prev {
            self.entry_mut(prev).ok_or(StoreError::InvalidHandle)?.next = next;
        } else {
            self.head = next;
        }
        if let Some(next) = next {
            self.entry_mut(next).ok_or(StoreError::InvalidHandle)?.prev = prev;
        } else {
            self.tail = prev;
        }
        let slot = &mut self.slots[id.slot as usize];
        slot.entry = None;
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
        } else {
            slot.retired = true;
        }
        Ok(next)
    }
    pub(crate) fn cursor(&self, budget: usize) -> ThinkerCursor {
        ThinkerCursor {
            next: self.head,
            previous: None,
            remaining: budget,
        }
    }
    pub(crate) fn next_visit(
        &mut self,
        cursor: &mut ThinkerCursor,
    ) -> Result<Option<Visit>, StoreError> {
        if let Some(previous) = cursor.previous.take() {
            cursor.next = self.entry(previous).ok_or(StoreError::InvalidHandle)?.next;
        }
        let Some(id) = cursor.next else {
            return Ok(None);
        };
        if cursor.remaining == 0 {
            return Err(StoreError::VisitLimit);
        }
        cursor.remaining -= 1;
        if self.entry(id).ok_or(StoreError::InvalidHandle)?.pending {
            cursor.next = self.retire(id)?;
            Ok(Some(Visit::Retired(id)))
        } else {
            cursor.previous = Some(id);
            Ok(Some(Visit::Think(id)))
        }
    }
    /// Runs append-sensitive traversal with a finite visit budget. Self-removal
    /// and removal of already-visited thinkers take effect on the next traversal.
    /// Context capabilities prevent recursive traversal/clear during callbacks.
    pub fn run(
        &mut self,
        budget: usize,
        mut callback: impl FnMut(Visit, &mut ThinkerContext<'_, T>),
    ) -> Result<(), StoreError> {
        let mut cursor = self.cursor(budget);
        while let Some(visit) = self.next_visit(&mut cursor)? {
            let writable = matches!(visit, Visit::Think(_));
            callback(
                visit,
                &mut ThinkerContext {
                    store: self,
                    writable,
                },
            );
        }
        Ok(())
    }
    /// Reinitializes order and invalidates occupied handles without resetting
    /// generations. Fresh stores have equivalent IDs; old-store handles are scoped.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            if slot.entry.take().is_some() {
                if let Some(g) = slot.generation.checked_add(1) {
                    slot.generation = g;
                } else {
                    slot.retired = true;
                }
            }
        }
        self.head = None;
        self.tail = None;
    }
}

/// Restricted callback access: no reentrant traversal or initialization.
pub struct ThinkerContext<'a, T> {
    store: &'a mut ThinkerStore<T>,
    writable: bool,
}
impl<T> ThinkerContext<'_, T> {
    /// Reads valid storage during any visit.
    pub fn get(&self, id: ActorId) -> Option<&T> {
        self.store.get(id)
    }
    /// Mutates a live callback's addressed storage; retirement observers are read-only.
    pub fn get_mut(&mut self, id: ActorId) -> Option<&mut T> {
        if self.writable {
            self.store.get_mut(id)
        } else {
            None
        }
    }
    /// Appends a thinker that can run later in this same traversal.
    pub fn spawn(&mut self, value: T) -> Result<ActorId, StoreError> {
        if self.writable {
            self.store.spawn(value)
        } else {
            Err(StoreError::ReadOnlyVisit)
        }
    }
    /// Marks another/current thinker without unlinking it.
    pub fn mark_remove(&mut self, id: ActorId) -> Result<bool, StoreError> {
        if self.writable {
            self.store.mark_remove(id)
        } else {
            Err(StoreError::ReadOnlyVisit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generation_exhaustion_permanently_retires_slot() {
        let mut store = ThinkerStore::new(2).unwrap();
        let first = store.spawn(1).unwrap();
        store.slots[0].generation = u64::MAX;
        let last = ActorId::from_parts(0, u64::MAX);
        store.head = Some(last);
        store.tail = Some(last);
        store.mark_remove(last).unwrap();
        store.run(2, |_, _| {}).unwrap();
        let next = store.spawn(2).unwrap();
        assert_eq!(next.slot(), 1);
        assert!(store.get(first).is_none());
        assert!(store.get(last).is_none());
    }
}
