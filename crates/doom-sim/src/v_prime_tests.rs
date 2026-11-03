//! The vertical invariant V′ and the relaxed item witness of the movers policy (M3h8
//! note §4.1, §4.2, plan step 12): soaks on the committed doorway and lift rooms
//! asserting both after every tic, a corpse and the player crunched to giblets, and
//! the refusal of states outside V′ with the complete state preserved. Rust-only
//! safety tests: the expected facts come from the source reading of the note
//! (p_map.c `PIT_ChangeSector`, `P_ThingHeightClip`), never from Rust output.
use crate::movers_replay::{case, command, replay_observed, setup_map};
use crate::*;

/// Asserts V′ for every non-impact actor and the relaxed item witness for every
/// item, as the tic-boundary preflight checks them; returns the cases seen.
fn assert_v_prime(s: &Simulation, label: &str, op: usize) -> [usize; 5] {
    let admission = s.movers.as_ref().unwrap();
    let policy = s.player_policy();
    let mut seen = [0; 5];
    for id in s.actor_order() {
        if s.actors.is_pending(id) != Some(false) {
            continue;
        }
        let a = s.actor(id).unwrap();
        let l = a.life().unwrap();
        let unlinked_blast = original_registry::is_fireball(l.actor_type)
            && l.flags & original_player::MF_MISSILE == 0;
        if original_registry::is_impact(l.actor_type) || unlinked_blast {
            continue;
        }
        let case = player_map_scale::v_prime_case(a, admission.reach, admission.delta)
            .unwrap_or_else(|| panic!("{label} op{op}: {id:?} outside V′: {a:?}"));
        seen[usize::from(case as u8 - b'a')] += 1;
        assert!(player_map_scale::v_prime_admitted(a, admission).is_ok());
        if world_pickups::kind_for(policy, l.actor_type).is_some() {
            world_pickups::validate_item(s.room(), s.item_sectors(), *a)
                .unwrap_or_else(|e| panic!("{label} op{op}: item {id:?}: {e}"));
            assert_eq!(a.z, a.floor, "{label} op{op}: item {id:?} off its floor");
        }
    }
    seen
}

/// A deterministic command stream (xorshift): walking, turning, firing and using in
/// bursts; a dead player gets the empty command (the only dead command admitted
/// besides `BT_USE` alone, which requests the reborn and would end the run).
struct Commands(u32);
impl Commands {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn command(&mut self, dead: bool) -> [TicCommand; 4] {
        let r = self.next();
        let mut commands = [TicCommand::default(); 4];
        if dead {
            return commands;
        }
        let mut buttons = 0;
        if r % 7 == 0 {
            buttons |= 1;
        }
        if (r >> 3) % 9 == 0 {
            buttons |= 2;
        }
        commands[0] = TicCommand {
            forward: ((r >> 8) % 51) as i8 - 20,
            side: ((r >> 14) % 21) as i8 - 10,
            angle_turn: (((r >> 19) % 9) as i16 - 4) * 1024,
            buttons,
        };
        commands
    }
}

/// 1,200 tics with four seeds on every doorway map (the empty room, the gibs and
/// stuck-shootable populations) and every lift map: V′ and the item witness hold
/// after every tic, and every refusal-free tic advances.
#[test]
fn soaks_keep_v_prime_and_the_item_witness_on_the_doorway_and_lift_rooms() {
    let rooms = [
        ("door_use_wait_close", 0),
        ("door_monster_back_side", 1),
        ("door_gibs_drop_bonus", 2),
        ("door_stuck_shootable", 3),
        ("lift_player", 7),
        ("lift_retrigger_beside_removed", 8),
        ("lift_monster_rider", 9),
        ("lift_monster_cross", 10),
        ("lift_last_step_short", 11),
        ("lift_reverse_at_top", 12),
    ];
    let mut seen = [0usize; 5];
    for (label, map) in rooms {
        let base = case(label);
        assert_eq!(base.map, map);
        for seed in [1u8, 77, 151, 254] {
            let mut c = base.clone();
            c.seed = seed;
            let (mut s, _, _) = setup_map(&c, crate::player_movers_tests::room(c.map).map());
            let mut commands = Commands(0x9e37_79b9 ^ (u32::from(seed) << 8) ^ map as u32);
            for op in 1..=1200 {
                let dead = s.original_player.as_ref().unwrap().state == PlayerBodyState::Dead;
                s.advance(commands.command(dead))
                    .unwrap_or_else(|e| panic!("{label} seed {seed} op{op}: {e}"));
                for (total, n) in seen.iter_mut().zip(assert_v_prime(&s, label, op)) {
                    *total += n;
                }
            }
        }
    }
    // Every soak keeps the ordinary case; the stuck shootable room reaches (d).
    assert!(seen[0] > 0 && seen[3] > 0, "{seen:?}");
}

/// Both V′ door scenarios satisfy V′ and the item witness at every op of the C
/// replay, and reach the cases they were built for: the bonus stuck under the
/// closed door (b) and the shootable zombieman stuck under it (d). In
/// `door_gibs_drop_bonus` the corpse is crunched to giblets (p_map.c:1269–1279):
/// `S_GIBS`, `MF_SOLID` cleared, height and radius 0, and stays so.
#[test]
fn the_door_scenarios_reach_the_stuck_cases_at_every_op_inside_v_prime() {
    for (label, stuck) in [("door_gibs_drop_bonus", 1), ("door_stuck_shootable", 3)] {
        let c = case(label);
        let mut reached = 0;
        let mut gibbed = 0;
        let (_, compared) = replay_observed(&c, c.tics, |s, op| {
            reached += assert_v_prime(s, label, op)[stuck];
            gibbed += s
                .actor_order()
                .filter(|&id| s.actor(id).is_some_and(planes::is_gibs))
                .count();
        });
        assert_eq!(compared.ops, c.tics + 1, "{label}");
        assert!(reached > 0, "{label}");
        assert_eq!(gibbed > 0, label == "door_gibs_drop_bonus", "{label}");
    }
}

/// The player killed standing in the doorway of `door_reverse_on_player` (the C
/// replay up to op 140, then 200 damage): the closing door no longer reverses on the
/// non-shootable corpse (`nofit` is set only for shootable things, p_map.c:1290–1296),
/// so it crunches the dead player to giblets (p_map.c:1269–1279: `S_GIBS`,
/// `MF_SOLID` cleared, height and radius 0) and closes; every later tic keeps V′.
#[test]
fn a_closing_door_crunches_the_dead_player_to_giblets() {
    let c = case("door_reverse_on_player");
    let (mut s, _) = crate::movers_replay::replay(&c, 140);
    let body = s.original_player.as_ref().unwrap().mo.unwrap();
    let x = s.actor(body).unwrap().position()[0];
    assert!(
        x > Fixed::from_map_units(16) && x < Fixed::from_map_units(48),
        "{x:?}"
    );
    s.checked_demon_operation(|h| -> Result<(), SimError> {
        let plan = combat::DamagePlan::current_original(
            h,
            DamageRequest {
                target: body,
                inflictor: None,
                source: None,
                amount: 200,
            },
        )?;
        lifecycle::ActorAccess::reserve(&mut h.access, plan.spawn_count())?;
        h.apply_damage(plan);
        Ok(())
    })
    .unwrap()
    .unwrap();
    let mut gibbed_at = None;
    for op in 141..=700 {
        s.advance([TicCommand::default(); 4])
            .unwrap_or_else(|e| panic!("op{op}: {e}"));
        assert_v_prime(&s, "dead player", op);
        let a = s.actor(body).unwrap();
        if planes::is_gibs(a) {
            gibbed_at.get_or_insert(op);
        } else {
            assert!(gibbed_at.is_none(), "op{op}: left S_GIBS");
        }
    }
    assert!(gibbed_at.is_some());
    let a = s.actor(body).unwrap();
    let l = a.life().unwrap();
    assert_eq!(
        (a.height(), a.radius(), l.flags & original_player::MF_SOLID),
        (Fixed::ZERO, Fixed::ZERO, 0)
    );
    assert_eq!(
        s.original_player.as_ref().unwrap().state,
        PlayerBodyState::Dead
    );
    // The identity read admits the giblets' state outside the player's own graph.
    assert_eq!(
        s.original_actor_identity(body).unwrap().1,
        Some(identity::OriginalState::S_GIBS)
    );
    // The door closed over the giblets: its ceiling is back at its floor.
    assert_eq!(s.sectors[2].ceiling, s.sectors[2].floor);
}

/// States outside V′ (or outside the item witness) are refused before the tic with
/// every byte and the RNG preserved; the boundary values on the inside are
/// admitted. Rust-only corruption of `door_gibs_drop_bonus` after the corpse was
/// crunched and the bonus stuck: the player sunk by exactly `Δ` is case (c) and by
/// one fixed-point unit more is refused; the giblets at `R_min − H_max` are case
/// (e) and one unit lower are refused; a shootable thing below its floor by more
/// than `Δ` is refused (the corpse envelope is not shootable); the item off its
/// floor is refused by the item witness although it is inside (a).
#[test]
fn states_outside_v_prime_are_refused_with_the_state_preserved() {
    let c = case("door_gibs_drop_bonus");
    let (s, _) = crate::movers_replay::replay(&c, 250);
    let admission = s.movers.clone().unwrap();
    assert_eq!((admission.reach, admission.delta), ([0, 128], 2));
    let body = s.original_player.as_ref().unwrap().mo.unwrap();
    let find = |f: &dyn Fn(&Actor) -> bool| s.actor_order().find(|&id| f(s.actor(id).unwrap()));
    let gibs = find(&|a| planes::is_gibs(a)).unwrap();
    let bonus = find(&|a| {
        a.life()
            .is_some_and(|l| world_pickups::kind_for(s.player_policy(), l.actor_type).is_some())
    })
    .unwrap();
    // The bonus is stuck under the closed door (b).
    assert_eq!(
        player_map_scale::v_prime_case(s.actor(bonus).unwrap(), admission.reach, admission.delta),
        Some('b')
    );
    let units = Fixed::from_map_units;
    let raw = Fixed::from_bits(1);
    let floor_of = |id: ActorId| s.actor(id).unwrap().floor;
    let refused = |id: ActorId, edit: &dyn Fn(&mut Actor), expected: SimError| {
        let mut corrupt = s.clone();
        edit(corrupt.actors.get_mut(id).unwrap());
        let before = corrupt.clone();
        assert_eq!(corrupt.advance(command(&c, 251)), Err(expected));
        assert_eq!(corrupt.snapshot(), before.snapshot());
        assert_eq!(corrupt.rng(), before.rng());
        assert_eq!(corrupt.events().len(), before.events().len());
    };
    let admitted = |id: ActorId, edit: &dyn Fn(&mut Actor), case: char| {
        let mut ok = s.clone();
        edit(ok.actors.get_mut(id).unwrap());
        let a = ok.actor(id).unwrap();
        assert_eq!(
            player_map_scale::v_prime_case(a, admission.reach, admission.delta),
            Some(case)
        );
        ok.advance(command(&c, 251)).unwrap();
    };
    let vertical = SimError::MapScale(MapScaleError::Vertical);
    let player_floor = floor_of(body);
    admitted(body, &|a| a.z = player_floor - units(2), 'c');
    refused(body, &|a| a.z = player_floor - units(2) - raw, vertical);
    let gibs_floor = floor_of(gibs);
    assert!(gibs_floor - units(56) > units(-56) - raw);
    admitted(gibs, &|a| a.z = units(-56), 'e');
    refused(gibs, &|a| a.z = units(-56) - raw, vertical);
    // A shootable thing is never in (e): the live player 3 units down is refused.
    refused(body, &|a| a.z = player_floor - units(3), vertical);
    // A z above the giblets' ceiling (`z + 0 > ceilingz`) is in no case.
    let gibs_ceiling = s.actor(gibs).unwrap().ceiling;
    refused(gibs, &|a| a.z = gibs_ceiling + raw, vertical);
    // The relaxed item witness keeps `z == floorz` although (a) would admit it (the
    // bonus lifted one unit under a ceiling raised to 64).
    let bonus_floor = floor_of(bonus);
    refused(
        bonus,
        &|a| {
            a.z = bonus_floor + units(1);
            a.ceiling = bonus_floor + units(64);
        },
        SimError::PlayerBody(PlayerBodyError::Invariant),
    );
    // Older policies keep the M3h4 invariant: the player sunk by one unit is refused
    // under the specials policy (see the heights tests for the full rollback).
    assert!(
        player_map_scale::vertical_admitted(&{
            let mut a = *s.actor(body).unwrap();
            a.z = player_floor - raw;
            a
        })
        .is_err()
    );
}
