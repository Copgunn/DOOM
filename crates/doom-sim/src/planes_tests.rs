//! Rust-only tests of `T_MovePlane`, `P_ChangeSector`, `PIT_ChangeSector` and
//! `P_ThingHeightClip` on the committed M3h8 rooms. Expected values come from the
//! original source reading (M3h8 note §2.7) and the C facts pinned by the evidence,
//! never from Rust output.
use super::*;
use crate::player_movers_tests::{movers, room};
use crate::*;

/// The committed setup of a room: THINGs, the player body at `body`, weapons, then
/// `P_SpawnSpecials`.
pub(crate) fn setup(index: usize, body: [i16; 2]) -> (Simulation, Vec<ActorId>, ActorId) {
    let mut s = movers(&room(index)).unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
    let player = s
        .spawn_player_body(PlayerSlot::One, body.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    (s, things, player)
}

/// Runs `f` with a plane host over the committed state, inside one store traversal
/// whose first live actor visit is used as the pass context, then commits the
/// staged sector writes. Returns `f`'s result and the events it emitted.
pub(crate) fn with_planes<R>(
    s: &mut Simulation,
    f: impl FnOnce(&mut PlaneHost<'_, '_>) -> R,
) -> (R, Vec<Event>) {
    let mut writes = StagedWrites::default();
    let mut events = Vec::new();
    let mut f = Some(f);
    let mut out = None;
    let registry = s.registry.clone().unwrap();
    let bound = s.actors.visit_bound();
    let (sectors, lines, world, player, statistics, rng, spatial, bindings, options, boundary) = (
        &s.sectors,
        &s.line_specials,
        &s.world,
        &mut s.original_player,
        &mut s.statistics,
        &mut s.rng,
        &mut s.spatial,
        &mut s.bindings,
        s.actor_options,
        s.boundary.clone(),
    );
    s.actors
        .run(bound, |visit, context| {
            if let (Visit::Think(_), Some(f)) = (visit, f.take()) {
                let mut host = PlaneHost {
                    committed: sectors,
                    committed_lines: lines,
                    writes: &mut writes,
                    access: context,
                    player: player.as_mut(),
                    statistics: statistics.as_mut(),
                    perception: None,
                    registry: &registry,
                    world,
                    options,
                    rng,
                    spatial,
                    bindings,
                    events: &mut events,
                    boundary: boundary.as_deref(),
                    specials: None,
                };
                out = Some(f(&mut host));
            }
        })
        .unwrap();
    writes.commit(&mut s.sectors, &mut s.line_specials);
    (out.expect("a live actor visit"), events)
}
fn units(v: i16) -> Fixed {
    Fixed::from_map_units(v)
}
fn clips(events: &[Event]) -> Vec<(ActorId, ClipOutcome, Fixed, Fixed, Fixed)> {
    events
        .iter()
        .filter_map(|e| match *e {
            Event::Mover(MoverEvent::ThingClipped {
                actor,
                outcome,
                z,
                floor,
                ceiling,
            }) => Some((actor, outcome, z, floor, ceiling)),
            _ => None,
        })
        .collect()
}

/// `door_stuck_shootable` (room 3): a zombieman spawned in room 0 with its box over
/// the closed door sector 2 (0/0). The door's rising steps ignore `nofit` (the
/// `#if 0` restore) and leave it stuck on its floor (V′ case (d)); a closing step
/// while it is stuck is refused and undone (`crushed`), its last step stops short.
#[test]
fn rising_ceiling_ignores_nofit_and_a_closing_step_reverses() {
    let (mut s, things, _) = setup(3, [-200, 192]);
    let zombie = things[0];
    assert_eq!(s.sectors[2].ceiling, Fixed::ZERO);
    let (results, events) = with_planes(&mut s, |h| {
        let up = (0..5)
            .map(|_| {
                h.move_plane(2, units(2), units(124), Plane::Ceiling, 1)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let down = h
            .move_plane(2, units(2), units(0), Plane::Ceiling, -1)
            .unwrap();
        (up, down)
    });
    assert_eq!(results.0, vec![MoveResult::Ok; 5]);
    // Ceiling 10 does not fit a 56-unit zombieman: the down step to 8 is undone.
    assert_eq!(results.1, MoveResult::Crushed);
    assert_eq!(s.sectors[2].ceiling, units(10));
    let zombie_clips: Vec<_> = clips(&events)
        .into_iter()
        .filter(|c| c.0 == zombie)
        .collect();
    // Every clip: stuck shootable on its floor 0 under the door's box-wide ceiling.
    assert!(
        zombie_clips
            .iter()
            .all(|c| c.1 == ClipOutcome::NoFit && c.2 == Fixed::ZERO)
    );
    assert_eq!(zombie_clips[0].4, units(2));
    // Down step: 8 then the restore clip at 10.
    let n = zombie_clips.len();
    assert_eq!(zombie_clips[n - 2].4, units(8));
    assert_eq!(zombie_clips[n - 1].4, units(10));
    let a = s.actor(zombie).unwrap();
    assert_eq!(
        (a.z, a.floor, a.ceiling),
        (Fixed::ZERO, Fixed::ZERO, units(10))
    );
    // The last down step (10 → dest 9 < 10 − 2... i.e. 10 − 2 < 9) is refused too:
    // pastdest, stopped short at 10.
    let (result, _) = with_planes(&mut s, |h| {
        h.move_plane(2, units(2), units(9), Plane::Ceiling, -1)
            .unwrap()
    });
    assert_eq!(result, MoveResult::PastDest);
    assert_eq!(s.sectors[2].ceiling, units(10));
    // Sector-changed records: one per P_ChangeSector, nofit each time here.
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Mover(MoverEvent::SectorChanged {
            sector: 2,
            nofit: true
        })
    )));
}

/// `door_gibs_drop_bonus` (room 2): a zombieman killed in the open door sector 4
/// (0/124) drops a clip; a health bonus lies in the door. Closing at 14 then 12: the
/// dropped clip (height 16) is removed at 14, the bonus is left stuck, the corpse
/// (height 56 >> 2 = 14) still fits at 14 and becomes gibs at 12. Non-shootable
/// things never refuse the step.
#[test]
fn closing_ceiling_removes_drops_leaves_items_and_gibs_corpses() {
    let (mut s, things, _) = setup(2, [-200, 0]);
    let (zombie, bonus) = (things[0], things[1]);
    // P_DamageMobj through the shared path (P_KillMobj and its clip drop).
    with_planes(&mut s, |h| {
        let mut life = h.life();
        let request = DamageRequest {
            target: zombie,
            inflictor: None,
            source: None,
            amount: 100,
        };
        let plan = crate::combat::DamagePlan::current_original(&life, request).unwrap();
        lifecycle::ActorAccess::reserve(&mut life.access, plan.spawn_count()).unwrap();
        life.apply_damage(plan);
    });
    let dropped = s
        .actors
        .order()
        .find(|id| {
            s.actor(*id)
                .and_then(Actor::life)
                .is_some_and(|l| l.flags & MF_DROPPED != 0)
        })
        .expect("P_KillMobj drop");
    assert_eq!(s.actor(zombie).unwrap().height(), units(14));
    s.sectors[4].ceiling = units(16);
    let (results, events) = with_planes(&mut s, |h| {
        [
            h.move_plane(4, units(2), Fixed::ZERO, Plane::Ceiling, -1)
                .unwrap(),
            h.move_plane(4, units(2), Fixed::ZERO, Plane::Ceiling, -1)
                .unwrap(),
        ]
    });
    assert_eq!(results, [MoveResult::Ok; 2]);
    assert_eq!(s.sectors[4].ceiling, units(12));
    let outcome = |id: ActorId| {
        clips(&events)
            .into_iter()
            .filter(|c| c.0 == id)
            .map(|c| c.1)
            .collect::<Vec<_>>()
    };
    assert_eq!(outcome(dropped), vec![ClipOutcome::DroppedRemoved]);
    assert_eq!(
        outcome(bonus),
        vec![ClipOutcome::LeftStuck, ClipOutcome::LeftStuck]
    );
    assert_eq!(outcome(zombie), vec![ClipOutcome::Fits, ClipOutcome::Gibs]);
    // Marked and unlinked; the traversal that ran the steps already retired it.
    assert_ne!(s.actors.is_pending(dropped), Some(false));
    assert!(!s.spatial.order.contains(&dropped));
    let gibs = s.actor(zombie).unwrap();
    assert_eq!(
        gibs.life().unwrap().state,
        Some(StateId(identity::OriginalState::S_GIBS.ordinal()))
    );
    assert_eq!((gibs.height(), gibs.radius()), (Fixed::ZERO, Fixed::ZERO));
    assert_eq!(gibs.life().unwrap().flags & MF_SOLID, 0);
    // The stuck bonus stays on its floor under the closed door (V′ case (b)).
    let b = s.actor(bonus).unwrap();
    assert_eq!(
        (b.z, b.floor, b.ceiling),
        (Fixed::ZERO, Fixed::ZERO, units(12))
    );
    // No random draw on any path.
    assert!(!events.iter().any(|e| matches!(
        e,
        Event::ActorState(lifecycle::ActorStateEvent::Random { .. })
    )));
}

/// `floor_turbo_jump_up` / `item_rises_onto_step` (room 13): the turbo target 5
/// (floor 32) jumps to 72 in one tic; the zombieman and the clip on it rise with it
/// (onfloor), and the clip whose center is in room 0 (floor 0) but whose box
/// overlaps sector 5 gets its first box-wide clip and rises onto 72.
#[test]
fn turbo_jump_up_lifts_onfloor_things_and_a_straddling_item() {
    let (mut s, things, _) = setup(13, [700, 64]);
    let (zombie, on_step, straddling) = (things[0], things[1], things[2]);
    assert_eq!(s.actor(straddling).unwrap().z, Fixed::ZERO);
    let (result, events) = with_planes(&mut s, |h| {
        h.move_plane(5, units(4), units(72), Plane::Floor, -1)
            .unwrap()
    });
    assert_eq!(result, MoveResult::PastDest);
    assert_eq!(s.sectors[5].floor, units(72));
    for id in [zombie, on_step, straddling] {
        let a = s.actor(id).unwrap();
        assert_eq!((a.z, a.floor), (units(72), units(72)), "{id:?}");
    }
    assert!(clips(&events).iter().all(|c| c.1 == ClipOutcome::Fits));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Mover(MoverEvent::PlaneStep {
            sector: 5,
            plane: Plane::Floor,
            result: MoveResult::PastDest,
            ..
        })
    )));
}

/// `player_picks_up_in_clip`: a re-clipped player touches the special things its
/// box overlaps inside `P_ThingHeightClip` (the existing touch path), and the item
/// picked up is not clipped afterwards.
#[test]
fn player_picks_up_an_item_inside_a_height_clip() {
    // The player beside the clip on the turbo step (128, 192), boxes overlapping.
    let (mut s, things, player) = setup(13, [150, 200]);
    let item = things[1];
    let clips_before = s.original_player.unwrap().weapon;
    let (_, events) = with_planes(&mut s, |h| {
        h.move_plane(5, units(4), units(72), Plane::Floor, -1)
            .unwrap()
    });
    assert_ne!(s.actors.is_pending(item), Some(false));
    assert!(events.iter().any(|e| matches!(e, Event::Pickup(_))));
    assert!(clips(&events).iter().all(|c| c.0 != item));
    assert!(clips(&events).iter().any(|c| c.0 == player));
    assert_ne!(s.original_player.unwrap().weapon, clips_before);
    assert_eq!(s.actor(player).unwrap().z, units(72));
}
