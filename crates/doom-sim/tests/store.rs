//! Ownership/lifecycle assertions distinguish identity from exact scheduling order.
use doom_sim::{ActorId, StoreError, ThinkerStore, Visit};

#[test]
fn growing_traversal_budget_is_explicit_and_retains_completed_updates() {
    let mut store = ThinkerStore::new(8).unwrap();
    store.spawn(1).unwrap();
    let mut updates = Vec::new();
    let result = store.run(3, |visit, context| {
        if let Visit::Think(id) = visit {
            let value = *context.get(id).unwrap();
            updates.push(value);
            context.spawn(value + 1).unwrap();
        }
    });
    assert_eq!(result, Err(StoreError::VisitLimit));
    assert_eq!(updates, [1, 2, 3]);
    assert_eq!(
        store
            .order()
            .filter_map(|id| store.get(id).copied())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
}

#[test]
fn insertion_removal_reuse_stale_handles_and_order() {
    let mut store = ThinkerStore::new(4).unwrap();
    let a = store.spawn(1).unwrap();
    let b = store.spawn(2).unwrap();
    let c = store.spawn(3).unwrap();
    store.mark_remove(b).unwrap();
    assert_eq!(store.get(b), Some(&2));
    assert_eq!(store.is_pending(b), Some(true));
    let mut trace = Vec::new();
    store.run(8, |v, _| trace.push(v)).unwrap();
    assert_eq!(trace, [Visit::Think(a), Visit::Retired(b), Visit::Think(c)]);
    assert!(store.get(b).is_none());
    let d = store.spawn(4).unwrap();
    assert_eq!(d.slot(), b.slot());
    assert_ne!(d.generation(), b.generation());
    assert_eq!(store.order().collect::<Vec<_>>(), [a, c, d]);
    assert!(store.get(b).is_none());
    assert_eq!(
        store.mark_remove(ActorId::from_parts(100, 1)),
        Err(StoreError::InvalidHandle)
    );
}

#[test]
fn self_removal_is_deferred_but_unvisited_removal_skips_callback() {
    let mut store = ThinkerStore::new(8).unwrap();
    let a = store.spawn(1).unwrap();
    let b = store.spawn(2).unwrap();
    let c = store.spawn(3).unwrap();
    let mut d = None;
    let mut first = Vec::new();
    store
        .run(8, |v, context| {
            first.push(v);
            if v == Visit::Think(a) {
                context.mark_remove(a).unwrap();
                context.mark_remove(b).unwrap();
                d = Some(context.spawn(4).unwrap());
            }
        })
        .unwrap();
    assert_eq!(
        first,
        [
            Visit::Think(a),
            Visit::Retired(b),
            Visit::Think(c),
            Visit::Think(d.unwrap())
        ]
    );
    assert_eq!(store.get(a), Some(&1));
    let mut second = Vec::new();
    store
        .run(8, |v, context| {
            second.push(v);
            if let Visit::Retired(id) = v {
                assert!(context.get(id).is_none());
                assert_eq!(context.spawn(9), Err(StoreError::ReadOnlyVisit));
            }
        })
        .unwrap();
    assert_eq!(
        second,
        [Visit::Retired(a), Visit::Think(c), Visit::Think(d.unwrap())]
    );
}

#[test]
fn empty_reinitialization_capacity_and_explicit_visit_budget() {
    let mut store = ThinkerStore::new(3).unwrap();
    store.run(0, |_, _| panic!("empty traversal")).unwrap();
    let old = store.spawn(1).unwrap();
    store.clear();
    store.clear();
    let current = store.spawn(1).unwrap();
    assert!(store.get(old).is_none());
    assert_ne!(old, current);
    assert_eq!(store.run(0, |_, _| {}), Err(StoreError::VisitLimit));
    let mut trace = Vec::new();
    store
        .run(3, |v, context| {
            trace.push(v);
            if let Visit::Think(id) = v {
                let n = *context.get(id).unwrap();
                if n < 3 {
                    context.spawn(n + 1).unwrap();
                }
            }
        })
        .unwrap();
    assert_eq!(trace.len(), 3);
    assert_eq!(
        store
            .order()
            .filter_map(|id| store.get(id).copied())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(store.spawn(4), Err(StoreError::Capacity));
    assert!(ThinkerStore::<i32>::new(65537).is_err());
}
