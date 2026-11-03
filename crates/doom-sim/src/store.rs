use crate::SectorThinker;
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

/// Identity in the sector-thinker slot class, a namespace disjoint from actor slots
/// (`P_SpawnSpecials` lights, later doors and plats). Slot reuse never determines
/// thinker order either.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SectorThinkerId {
    slot: u32,
    generation: u64,
}
impl SectorThinkerId {
    /// Constructs a candidate handle; every store operation validates it.
    pub const fn from_parts(slot: u32, generation: u64) -> Self {
        Self { slot, generation }
    }
    /// Logical slot in the sector-thinker class.
    pub const fn slot(self) -> u32 {
        self.slot
    }
    /// Generation; exhausted slots retire permanently instead of wrapping.
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// One entry of the shared processing order (`thinkercap` in `p_tick.c`): mobjs and
/// sector thinkers in creation order. The kind selects the handle namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ThinkerId {
    /// An actor (`mobj_t`) in the actor slot class.
    Actor(ActorId),
    /// A sector thinker in its own slot class.
    Sector(SectorThinkerId),
}
impl ThinkerId {
    /// The actor identity, if this entry is an actor.
    pub const fn actor(self) -> Option<ActorId> {
        match self {
            Self::Actor(id) => Some(id),
            Self::Sector(_) => None,
        }
    }
    /// The sector-thinker identity, if this entry is a sector thinker.
    pub const fn sector(self) -> Option<SectorThinkerId> {
        match self {
            Self::Actor(_) => None,
            Self::Sector(id) => Some(id),
        }
    }
}

/// Derived storage availability, not health, shootability or execution permission.
/// No history is stored: after reuse, an older identity is stale. `Retired` below
/// describes an occupant; the DSIM slot `retired` bit instead means exhaustion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorAvailability {
    /// Current occupied storage, not marked for removal (possibly a corpse).
    Live,
    /// Current readable storage, spatially unlinked and awaiting traversal.
    PendingRemoval,
    /// Most recent occupant of a vacant slot, including an exhausted slot.
    Retired,
    /// Generation differs from the current occupant or most recent retirement.
    StaleGeneration,
    /// Current generation names a vacant slot and has not been issued.
    Vacant,
    /// Slot is not allocated, even if it is inside the configured capacity.
    OutOfRange,
    /// Generation zero is never issued.
    InvalidGeneration,
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
    /// A sector-thinker visit that no admitted policy creates; refused, never skipped.
    UnsupportedThinker,
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
    /// Invoke a live sector thinker (it may mark removal or append thinkers).
    SectorThink(SectorThinkerId),
    /// A sector thinker's storage has been retired; observational only.
    SectorRetired(SectorThinkerId),
}
impl Visit {
    fn of(id: ThinkerId, retired: bool) -> Self {
        match (id, retired) {
            (ThinkerId::Actor(id), false) => Self::Think(id),
            (ThinkerId::Actor(id), true) => Self::Retired(id),
            (ThinkerId::Sector(id), false) => Self::SectorThink(id),
            (ThinkerId::Sector(id), true) => Self::SectorRetired(id),
        }
    }
}

/// One occupied slot of either class; `prev`/`next` link the shared order.
#[derive(Clone, Debug)]
pub(crate) struct Entry<T> {
    pub value: T,
    pub pending: bool,
    pub prev: Option<ThinkerId>,
    pub next: Option<ThinkerId>,
}
#[derive(Clone, Debug)]
pub(crate) struct Slot<T> {
    pub generation: u64,
    pub retired: bool,
    pub entry: Option<Entry<T>>,
}

fn slot_entry<V>(slots: &[Slot<V>], slot: u32, generation: u64) -> Option<&Entry<V>> {
    let slot = slots.get(slot as usize)?;
    if slot.generation != generation {
        return None;
    }
    slot.entry.as_ref()
}
fn slot_entry_mut<V>(slots: &mut [Slot<V>], slot: u32, generation: u64) -> Option<&mut Entry<V>> {
    let slot = slots.get_mut(slot as usize)?;
    if slot.generation != generation {
        return None;
    }
    slot.entry.as_mut()
}
fn classify<V>(slots: &[Slot<V>], slot: u32, generation: u64) -> ActorAvailability {
    if generation == 0 {
        return ActorAvailability::InvalidGeneration;
    }
    let Some(slot) = usize::try_from(slot)
        .ok()
        .and_then(|index| slots.get(index))
    else {
        return ActorAvailability::OutOfRange;
    };
    if let Some(entry) = &slot.entry {
        return if slot.generation != generation {
            ActorAvailability::StaleGeneration
        } else if entry.pending {
            ActorAvailability::PendingRemoval
        } else {
            ActorAvailability::Live
        };
    }
    if slot.retired {
        return if generation == slot.generation {
            ActorAvailability::Retired
        } else {
            ActorAvailability::StaleGeneration
        };
    }
    if generation == slot.generation {
        ActorAvailability::Vacant
    } else if generation.checked_add(1) == Some(slot.generation) {
        ActorAvailability::Retired
    } else {
        ActorAvailability::StaleGeneration
    }
}
fn free_slots<V>(slots: &[Slot<V>], max: u32) -> usize {
    slots
        .iter()
        .filter(|s| s.entry.is_none() && !s.retired)
        .count()
        + max as usize
        - slots.len()
}
fn reserve<V>(slots: &mut Vec<Slot<V>>, max: u32, count: usize) -> Result<(), StoreError> {
    if count > free_slots(slots, max) {
        return Err(StoreError::Capacity);
    }
    slots
        .try_reserve(count.min(max as usize - slots.len()))
        .map_err(|_| StoreError::Capacity)
}
/// Lowest nonretired reusable slot, else a fresh one within `max`.
fn claim<V>(slots: &mut Vec<Slot<V>>, max: u32) -> Result<(u32, u64), StoreError> {
    let index = if let Some(i) = slots.iter().position(|s| s.entry.is_none() && !s.retired) {
        i
    } else {
        if slots.len() >= max as usize {
            return Err(StoreError::Capacity);
        }
        slots.try_reserve(1).map_err(|_| StoreError::Capacity)?;
        slots.push(Slot {
            generation: 1,
            retired: false,
            entry: None,
        });
        slots.len() - 1
    };
    Ok((index as u32, slots[index].generation))
}
fn vacate<V>(slot: &mut Slot<V>) -> bool {
    let occupied = slot.entry.take().is_some();
    if occupied {
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
        } else {
            slot.retired = true;
        }
    }
    occupied
}

/// Order links of one shared-order entry.
#[derive(Clone, Copy)]
struct Links {
    pending: bool,
    prev: Option<ThinkerId>,
    next: Option<ThinkerId>,
}

pub(crate) struct ThinkerCursor {
    next: Option<ThinkerId>,
    previous: Option<ThinkerId>,
    remaining: usize,
}
/// Owned generational arena with two slot classes (actors `T`, sector thinkers `S`)
/// plus one safe linked processing order shared by both, exactly `P_RunThinkers`'
/// list: appends run in the same pass, removal is lazy. The sector-thinker class has
/// a capacity fixed at construction (zero unless built by `with_sector_capacity`).
#[derive(Clone, Debug)]
pub struct ThinkerStore<T, S = SectorThinker> {
    pub(crate) slots: Vec<Slot<T>>,
    pub(crate) sector_slots: Vec<Slot<S>>,
    /// First and last entry of the shared order (either kind).
    pub(crate) head: Option<ThinkerId>,
    pub(crate) tail: Option<ThinkerId>,
    pub(crate) max_slots: u32,
    pub(crate) max_sector_slots: u32,
}
impl<T> ThinkerStore<T> {
    /// Creates an empty store without sector thinkers (sector capacity zero, the
    /// simulation's payload type). The bound makes snapshots and callback budgets
    /// finite.
    pub fn new(max_slots: u32) -> Result<Self, StoreError> {
        Self::with_sector_capacity(max_slots, 0)
    }
}
impl<T, S> ThinkerStore<T, S> {
    /// Pure, checked classification. It neither resolves a different generation
    /// nor mutates references, processing order or generation counters.
    pub fn availability(&self, id: ActorId) -> ActorAvailability {
        classify(&self.slots, id.slot, id.generation)
    }
    /// The same classification in the sector-thinker slot class.
    pub fn sector_availability(&self, id: SectorThinkerId) -> ActorAvailability {
        classify(&self.sector_slots, id.slot, id.generation)
    }
    /// Creates an empty store whose sector-thinker class holds at most
    /// `max_sector_slots`; the actor class and its bound are unaffected.
    pub fn with_sector_capacity(max_slots: u32, max_sector_slots: u32) -> Result<Self, StoreError> {
        if max_slots > 65536 || max_sector_slots > 65536 {
            return Err(StoreError::InvalidLimit);
        }
        Ok(Self {
            slots: Vec::new(),
            sector_slots: Vec::new(),
            head: None,
            tail: None,
            max_slots,
            max_sector_slots,
        })
    }
    /// Sector-thinker slot bound fixed at construction.
    pub const fn sector_capacity(&self) -> u32 {
        self.max_sector_slots
    }
    /// One built-in traversal visits each slot of either class at most once
    /// unless a slot retired in that pass is refilled.
    pub(crate) fn visit_bound(&self) -> usize {
        self.max_slots as usize + self.max_sector_slots as usize
    }
    pub(crate) fn entry(&self, id: ActorId) -> Option<&Entry<T>> {
        slot_entry(&self.slots, id.slot, id.generation)
    }
    fn entry_mut(&mut self, id: ActorId) -> Option<&mut Entry<T>> {
        slot_entry_mut(&mut self.slots, id.slot, id.generation)
    }
    fn sector_entry(&self, id: SectorThinkerId) -> Option<&Entry<S>> {
        slot_entry(&self.sector_slots, id.slot, id.generation)
    }
    fn sector_entry_mut(&mut self, id: SectorThinkerId) -> Option<&mut Entry<S>> {
        slot_entry_mut(&mut self.sector_slots, id.slot, id.generation)
    }
    fn links(&self, id: ThinkerId) -> Option<Links> {
        let (pending, prev, next) = match id {
            ThinkerId::Actor(id) => self.entry(id).map(|e| (e.pending, e.prev, e.next))?,
            ThinkerId::Sector(id) => self.sector_entry(id).map(|e| (e.pending, e.prev, e.next))?,
        };
        Some(Links {
            pending,
            prev,
            next,
        })
    }
    /// Successor of a valid entry in the shared order (one slot lookup).
    fn next_of(&self, id: ThinkerId) -> Option<Option<ThinkerId>> {
        match id {
            ThinkerId::Actor(id) => self.entry(id).map(|e| e.next),
            ThinkerId::Sector(id) => self.sector_entry(id).map(|e| e.next),
        }
    }
    fn links_mut(
        &mut self,
        id: ThinkerId,
    ) -> Option<(&mut Option<ThinkerId>, &mut Option<ThinkerId>)> {
        match id {
            ThinkerId::Actor(id) => self.entry_mut(id).map(|e| (&mut e.prev, &mut e.next)),
            ThinkerId::Sector(id) => self
                .sector_entry_mut(id)
                .map(|e| (&mut e.prev, &mut e.next)),
        }
    }
    /// Reads valid storage, including marked-but-not-yet-retired thinkers.
    pub fn get(&self, id: ActorId) -> Option<&T> {
        Some(&self.entry(id)?.value)
    }
    /// Mutates only storage addressed by a currently valid identity.
    pub fn get_mut(&mut self, id: ActorId) -> Option<&mut T> {
        Some(&mut self.entry_mut(id)?.value)
    }
    /// Reads a valid sector thinker, including one marked but not yet retired.
    pub fn sector_get(&self, id: SectorThinkerId) -> Option<&S> {
        Some(&self.sector_entry(id)?.value)
    }
    /// Mutates only a sector thinker addressed by a currently valid identity.
    pub fn sector_get_mut(&mut self, id: SectorThinkerId) -> Option<&mut S> {
        Some(&mut self.sector_entry_mut(id)?.value)
    }
    /// Pending removal remains readable until the traversal visits it.
    pub fn is_pending(&self, id: ActorId) -> Option<bool> {
        Some(self.entry(id)?.pending)
    }
    /// Pending removal of a sector thinker, readable until the traversal visits it.
    pub fn sector_is_pending(&self, id: SectorThinkerId) -> Option<bool> {
        Some(self.sector_entry(id)?.pending)
    }
    pub(crate) fn reserve_spawns(&mut self, count: usize) -> Result<(), StoreError> {
        reserve(&mut self.slots, self.max_slots, count)
    }
    /// Links an entry just written at `id` (prev = tail, next = none) as the new tail.
    fn append(&mut self, id: ThinkerId) -> Result<(), StoreError> {
        if let Some(tail) = self.tail {
            *self.links_mut(tail).ok_or(StoreError::InvalidHandle)?.1 = Some(id);
        } else {
            self.head = Some(id);
        }
        self.tail = Some(id);
        Ok(())
    }
    /// Appends to processing order, choosing the lowest nonretired reusable slot.
    pub fn spawn(&mut self, value: T) -> Result<ActorId, StoreError> {
        let (slot, generation) = claim(&mut self.slots, self.max_slots)?;
        let id = ActorId::from_parts(slot, generation);
        self.slots[slot as usize].entry = Some(Entry {
            value,
            pending: false,
            prev: self.tail,
            next: None,
        });
        self.append(ThinkerId::Actor(id))?;
        Ok(id)
    }
    /// Appends a sector thinker to the shared order, choosing the lowest nonretired
    /// reusable sector slot; refused beyond the construction-time capacity.
    pub fn spawn_sector(&mut self, value: S) -> Result<SectorThinkerId, StoreError> {
        let (slot, generation) = claim(&mut self.sector_slots, self.max_sector_slots)?;
        let id = SectorThinkerId::from_parts(slot, generation);
        self.sector_slots[slot as usize].entry = Some(Entry {
            value,
            pending: false,
            prev: self.tail,
            next: None,
        });
        self.append(ThinkerId::Sector(id))?;
        Ok(id)
    }
    /// Marks removal without unlinking. Returns whether the mark changed.
    pub fn mark_remove(&mut self, id: ActorId) -> Result<bool, StoreError> {
        let entry = self.entry_mut(id).ok_or(StoreError::InvalidHandle)?;
        let changed = !entry.pending;
        entry.pending = true;
        Ok(changed)
    }
    /// Marks a sector thinker's removal without unlinking it.
    pub fn mark_remove_sector(&mut self, id: SectorThinkerId) -> Result<bool, StoreError> {
        let entry = self.sector_entry_mut(id).ok_or(StoreError::InvalidHandle)?;
        let changed = !entry.pending;
        entry.pending = true;
        Ok(changed)
    }
    /// The shared processing order of both kinds, including pending thinkers.
    pub fn shared_order(&self) -> impl Iterator<Item = ThinkerId> + '_ {
        let mut cursor = self.head;
        std::iter::from_fn(move || {
            let id = cursor?;
            cursor = self.next_of(id)?;
            Some(id)
        })
    }
    /// Logical actor processing order, including pending thinkers; never arena
    /// order. It is the actor subsequence of [`Self::shared_order`] and, without
    /// sector thinkers, that whole order. Walked directly (one slot lookup per
    /// entry, sector thinkers stepped over in place) because preflights of every
    /// policy call it several times per tic; it ends where `shared_order` ends.
    pub fn order(&self) -> impl Iterator<Item = ActorId> + '_ {
        let mut cursor = self.head;
        std::iter::from_fn(move || {
            loop {
                match cursor? {
                    ThinkerId::Actor(id) => {
                        cursor = self.entry(id)?.next;
                        return Some(id);
                    }
                    ThinkerId::Sector(id) => cursor = self.sector_entry(id)?.next,
                }
            }
        })
    }
    /// First actor of the actor subsequence.
    pub(crate) fn actor_head(&self) -> Option<ActorId> {
        self.order().next()
    }
    /// Last actor of the actor subsequence.
    pub(crate) fn actor_tail(&self) -> Option<ActorId> {
        let mut cursor = self.tail;
        while let Some(id) = cursor {
            match id {
                ThinkerId::Actor(id) => return Some(id),
                ThinkerId::Sector(_) => cursor = self.links(id)?.prev,
            }
        }
        None
    }
    /// Neighbors of a valid actor in the actor subsequence (sector thinkers skipped).
    pub(crate) fn actor_links(&self, id: ActorId) -> Option<(Option<ActorId>, Option<ActorId>)> {
        let entry = self.entry(id)?;
        let nearest = |mut cursor: Option<ThinkerId>, forward: bool| {
            while let Some(id) = cursor {
                match id {
                    ThinkerId::Actor(id) => return Some(Some(id)),
                    ThinkerId::Sector(_) => {
                        let links = self.links(id)?;
                        cursor = if forward { links.next } else { links.prev };
                    }
                }
            }
            Some(None)
        };
        Some((nearest(entry.prev, false)?, nearest(entry.next, true)?))
    }
    /// Unlinks `id`, whose current links the caller has just read, and vacates it.
    fn retire(&mut self, id: ThinkerId, links: Links) -> Result<Option<ThinkerId>, StoreError> {
        let Links { prev, next, .. } = links;
        if let Some(prev) = prev {
            *self.links_mut(prev).ok_or(StoreError::InvalidHandle)?.1 = next;
        } else {
            self.head = next;
        }
        if let Some(next) = next {
            *self.links_mut(next).ok_or(StoreError::InvalidHandle)?.0 = prev;
        } else {
            self.tail = prev;
        }
        match id {
            ThinkerId::Actor(id) => vacate(&mut self.slots[id.slot as usize]),
            ThinkerId::Sector(id) => vacate(&mut self.sector_slots[id.slot as usize]),
        };
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
            cursor.next = self.next_of(previous).ok_or(StoreError::InvalidHandle)?;
        }
        let Some(id) = cursor.next else {
            return Ok(None);
        };
        if cursor.remaining == 0 {
            return Err(StoreError::VisitLimit);
        }
        cursor.remaining -= 1;
        let links = self.links(id).ok_or(StoreError::InvalidHandle)?;
        if links.pending {
            cursor.next = self.retire(id, links)?;
            Ok(Some(Visit::of(id, true)))
        } else {
            cursor.previous = Some(id);
            Ok(Some(Visit::of(id, false)))
        }
    }
    /// Runs append-sensitive traversal with a finite visit budget. Self-removal
    /// and removal of already-visited thinkers take effect on the next traversal.
    /// Context capabilities prevent recursive traversal/clear during callbacks.
    pub fn run(
        &mut self,
        budget: usize,
        mut callback: impl FnMut(Visit, &mut ThinkerContext<'_, T, S>),
    ) -> Result<(), StoreError> {
        let mut cursor = self.cursor(budget);
        while let Some(visit) = self.next_visit(&mut cursor)? {
            let writable = matches!(visit, Visit::Think(_) | Visit::SectorThink(_));
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
    /// Reinitializes order and invalidates occupied handles of both classes without
    /// resetting generations. Fresh stores have equivalent IDs; old-store handles are
    /// scoped.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            vacate(slot);
        }
        for slot in &mut self.sector_slots {
            vacate(slot);
        }
        self.head = None;
        self.tail = None;
    }
}

/// Restricted callback access: no reentrant traversal or initialization.
pub struct ThinkerContext<'a, T, S = SectorThinker> {
    store: &'a mut ThinkerStore<T, S>,
    writable: bool,
}
impl<T, S> ThinkerContext<'_, T, S> {
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
    /// Reads a valid sector thinker during any visit.
    pub fn sector_get(&self, id: SectorThinkerId) -> Option<&S> {
        self.store.sector_get(id)
    }
    /// Mutates a sector thinker during a live visit; retirement observers are read-only.
    pub fn sector_get_mut(&mut self, id: SectorThinkerId) -> Option<&mut S> {
        if self.writable {
            self.store.sector_get_mut(id)
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
    /// Appends a sector thinker that can run later in this same traversal.
    pub fn spawn_sector(&mut self, value: S) -> Result<SectorThinkerId, StoreError> {
        if self.writable {
            self.store.spawn_sector(value)
        } else {
            Err(StoreError::ReadOnlyVisit)
        }
    }
    pub(crate) fn store(&self) -> &ThinkerStore<T, S> {
        self.store
    }
    /// Proves free capacity for later spawns in this same traversal.
    pub(crate) fn reserve_spawns(&mut self, count: usize) -> Result<(), StoreError> {
        if self.writable {
            self.store.reserve_spawns(count)
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
    /// Marks another/current sector thinker without unlinking it.
    pub fn mark_remove_sector(&mut self, id: SectorThinkerId) -> Result<bool, StoreError> {
        if self.writable {
            self.store.mark_remove_sector(id)
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
        let mut store = ThinkerStore::<i32>::new(2).unwrap();
        let first = store.spawn(1).unwrap();
        store.slots[0].generation = u64::MAX;
        let last = ActorId::from_parts(0, u64::MAX);
        store.head = Some(ThinkerId::Actor(last));
        store.tail = Some(ThinkerId::Actor(last));
        store.mark_remove(last).unwrap();
        store.run(2, |_, _| {}).unwrap();
        let next = store.spawn(2).unwrap();
        assert_eq!(next.slot(), 1);
        assert!(store.get(first).is_none());
        assert!(store.get(last).is_none());
    }

    type Mixed = ThinkerStore<i32, i32>;
    fn values(store: &Mixed) -> Vec<(bool, i32)> {
        store
            .shared_order()
            .map(|id| match id {
                ThinkerId::Actor(id) => (false, *store.get(id).unwrap()),
                ThinkerId::Sector(id) => (true, *store.sector_get(id).unwrap()),
            })
            .collect()
    }
    /// The actor-only head, tail and neighbor links equal those of a store built
    /// from the same actors alone (what every pre-DSIM34 snapshot encodes).
    fn assert_actor_subsequence(store: &Mixed) {
        let actors = store.order().collect::<Vec<_>>();
        assert_eq!(
            actors,
            store
                .shared_order()
                .filter_map(ThinkerId::actor)
                .collect::<Vec<_>>()
        );
        assert_eq!(store.actor_head(), actors.first().copied());
        assert_eq!(store.actor_tail(), actors.last().copied());
        for (i, id) in actors.iter().enumerate() {
            let prev = i.checked_sub(1).map(|p| actors[p]);
            assert_eq!(
                store.actor_links(*id),
                Some((prev, actors.get(i + 1).copied()))
            );
        }
    }

    #[test]
    fn shared_order_interleaves_kinds_in_creation_order() {
        let mut store = Mixed::with_sector_capacity(4, 3).unwrap();
        let a = store.spawn(1).unwrap();
        let s = store.spawn_sector(10).unwrap();
        let b = store.spawn(2).unwrap();
        let t = store.spawn_sector(11).unwrap();
        assert_eq!(
            store.shared_order().collect::<Vec<_>>(),
            [
                ThinkerId::Actor(a),
                ThinkerId::Sector(s),
                ThinkerId::Actor(b),
                ThinkerId::Sector(t)
            ]
        );
        // Separate namespaces: both classes start at slot 0, generation 1.
        assert_eq!((s.slot(), s.generation()), (0, 1));
        assert_eq!((t.slot(), t.generation()), (1, 1));
        assert_eq!(store.order().collect::<Vec<_>>(), [a, b]);
        assert_actor_subsequence(&store);
        assert_eq!(store.actor_links(a), Some((None, Some(b))));
        assert_eq!(store.actor_links(b), Some((Some(a), None)));
        let mut trace = Vec::new();
        store
            .run(store.visit_bound(), |v, _| trace.push(v))
            .unwrap();
        assert_eq!(
            trace,
            [
                Visit::Think(a),
                Visit::SectorThink(s),
                Visit::Think(b),
                Visit::SectorThink(t)
            ]
        );
    }

    #[test]
    fn appends_of_either_kind_run_in_the_same_pass() {
        let mut store = Mixed::with_sector_capacity(4, 3).unwrap();
        let a = store.spawn(1).unwrap();
        let s = store.spawn_sector(10).unwrap();
        let mut spawned = Vec::new();
        let mut trace = Vec::new();
        store
            .run(8, |v, context| {
                trace.push(v);
                match v {
                    Visit::Think(id) if id == a => {
                        spawned.push(ThinkerId::Sector(context.spawn_sector(11).unwrap()));
                    }
                    Visit::SectorThink(id) if id == s => {
                        *context.sector_get_mut(id).unwrap() += 100;
                        spawned.push(ThinkerId::Actor(context.spawn(2).unwrap()));
                    }
                    // The tail itself appends: the new entry still runs this pass.
                    Visit::Think(id) if *context.get(id).unwrap() == 2 => {
                        spawned.push(ThinkerId::Sector(context.spawn_sector(12).unwrap()));
                    }
                    _ => {}
                }
            })
            .unwrap();
        let [
            ThinkerId::Sector(t),
            ThinkerId::Actor(b),
            ThinkerId::Sector(u),
        ] = spawned[..]
        else {
            panic!("three appends");
        };
        assert_eq!(
            trace,
            [
                Visit::Think(a),
                Visit::SectorThink(s),
                Visit::SectorThink(t),
                Visit::Think(b),
                Visit::SectorThink(u)
            ]
        );
        assert_eq!(
            values(&store),
            [(false, 1), (true, 110), (true, 11), (false, 2), (true, 12)]
        );
        assert_actor_subsequence(&store);
    }

    #[test]
    fn removal_of_either_kind_is_lazy_and_relinks_the_actor_subsequence() {
        let mut store = Mixed::with_sector_capacity(4, 2).unwrap();
        let a = store.spawn(1).unwrap();
        let s = store.spawn_sector(10).unwrap();
        let b = store.spawn(2).unwrap();
        let t = store.spawn_sector(11).unwrap();
        let mut first = Vec::new();
        store
            .run(8, |v, context| {
                first.push(v);
                if v == Visit::Think(a) {
                    // Unvisited sector thinker: retired when reached, never run.
                    context.mark_remove_sector(s).unwrap();
                    context.mark_remove(b).unwrap();
                }
                if v == Visit::SectorThink(t) {
                    // Self-removal takes effect on the next traversal.
                    assert!(context.mark_remove_sector(t).unwrap());
                }
                if let Visit::SectorRetired(id) = v {
                    assert!(context.sector_get(id).is_none());
                    assert!(context.sector_get_mut(id).is_none());
                    assert_eq!(context.spawn_sector(0), Err(StoreError::ReadOnlyVisit));
                    assert_eq!(context.spawn(0), Err(StoreError::ReadOnlyVisit));
                }
            })
            .unwrap();
        assert_eq!(
            first,
            [
                Visit::Think(a),
                Visit::SectorRetired(s),
                Visit::Retired(b),
                Visit::SectorThink(t)
            ]
        );
        assert_eq!(store.sector_is_pending(t), Some(true));
        assert_eq!(store.sector_availability(s), ActorAvailability::Retired);
        assert_eq!(
            store.shared_order().collect::<Vec<_>>(),
            [ThinkerId::Actor(a), ThinkerId::Sector(t)]
        );
        assert_actor_subsequence(&store);
        let mut second = Vec::new();
        store.run(8, |v, _| second.push(v)).unwrap();
        assert_eq!(second, [Visit::Think(a), Visit::SectorRetired(t)]);
        assert_eq!(store.tail, Some(ThinkerId::Actor(a)));
        // The lowest vacated sector slot is reused with a new generation.
        let u = store.spawn_sector(12).unwrap();
        assert_eq!((u.slot(), u.generation()), (0, 2));
        assert_eq!(
            store.sector_availability(s),
            ActorAvailability::StaleGeneration
        );
        assert_eq!(store.mark_remove_sector(s), Err(StoreError::InvalidHandle));
        assert_actor_subsequence(&store);
    }

    #[test]
    fn actor_links_skip_runs_of_sector_thinkers() {
        let mut store = Mixed::with_sector_capacity(3, 4).unwrap();
        let s0 = store.spawn_sector(10).unwrap();
        let a = store.spawn(1).unwrap();
        for value in 11..14 {
            store.spawn_sector(value).unwrap();
        }
        let b = store.spawn(2).unwrap();
        assert_eq!(store.head, Some(ThinkerId::Sector(s0)));
        assert_eq!(store.actor_head(), Some(a));
        assert_eq!(store.actor_links(a), Some((None, Some(b))));
        assert_eq!(store.actor_links(b), Some((Some(a), None)));
        assert_eq!(store.spawn_sector(14), Err(StoreError::Capacity));
        let c = store.spawn(3).unwrap();
        store.spawn(4).unwrap_err();
        store.mark_remove(c).unwrap();
        store.run(store.visit_bound(), |_, _| {}).unwrap();
        // The tail is a sector-free actor again; the subsequence stays exact.
        assert_eq!(store.actor_tail(), Some(b));
        assert_actor_subsequence(&store);
    }

    #[test]
    fn sector_capacity_is_fixed_at_construction_and_refuses_explicitly() {
        assert_eq!(ThinkerStore::<i32>::new(2).unwrap().sector_capacity(), 0);
        let mut none = Mixed::with_sector_capacity(2, 0).unwrap();
        assert_eq!(none.visit_bound(), 2);
        assert_eq!(none.spawn_sector(1), Err(StoreError::Capacity));
        assert!(none.sector_slots.is_empty() && none.head.is_none());
        let mut two = Mixed::with_sector_capacity(1, 2).unwrap();
        assert_eq!(two.visit_bound(), 3);
        two.spawn_sector(1).unwrap();
        two.spawn_sector(2).unwrap();
        let before = values(&two);
        assert_eq!(two.spawn_sector(3), Err(StoreError::Capacity));
        // A refused sector spawn leaves the actor class and the order untouched.
        let a = two.spawn(4).unwrap();
        assert_eq!(values(&two), [before, vec![(false, 4)]].concat());
        assert_eq!(two.spawn(5), Err(StoreError::Capacity));
        assert_eq!(two.order().collect::<Vec<_>>(), [a]);
        assert!(Mixed::with_sector_capacity(1, 65537).is_err());
        assert!(Mixed::with_sector_capacity(65536, 65536).is_ok());
        // Clear vacates both classes without resetting generations.
        two.clear();
        assert!(two.shared_order().next().is_none());
        assert_eq!(two.spawn_sector(6).unwrap().generation(), 2);
    }

    #[test]
    fn actor_only_store_is_unchanged_by_the_sector_class() {
        let mut store = ThinkerStore::<i32>::new(4).unwrap();
        let ids = (0..3).map(|v| store.spawn(v).unwrap()).collect::<Vec<_>>();
        assert_eq!(
            store.shared_order().collect::<Vec<_>>(),
            ids.iter()
                .copied()
                .map(ThinkerId::Actor)
                .collect::<Vec<_>>()
        );
        assert_eq!(store.head, Some(ThinkerId::Actor(ids[0])));
        assert_eq!(store.tail, Some(ThinkerId::Actor(ids[2])));
        for (i, id) in ids.iter().enumerate() {
            let entry = store.entry(*id).unwrap();
            assert_eq!(
                store.actor_links(*id),
                Some((
                    entry.prev.and_then(ThinkerId::actor),
                    entry.next.and_then(ThinkerId::actor)
                ))
            );
            assert_eq!(entry.prev.is_some(), i > 0);
        }
    }
}
