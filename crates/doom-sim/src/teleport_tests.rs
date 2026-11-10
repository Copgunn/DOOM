//! Teleportation under wire 24 (M4b step 4). Expected values come from the C
//! source (cited), from the genuine engine's runs recorded in
//! docs/validation/M4B.md ("C-observed"), or from written derivations; never from
//! Rust output. The tic-for-tic comparison of the M4b scripts against the genuine
//! goldens is `tests/episode1v2_mechanism_goldens.rs`.
use super::*;
use crate::PursuitError;
use crate::diagonal_room::Lumps;
use crate::identity::OriginalState;
use crate::{
    MapContextOptions, OriginalExecutionProfile as P, OriginalRegistryError, PlayerSlot,
    Simulation, Skill, TicCommand, WeaponInventory, demo::parse_demo_episode1_v2,
};

const V1: P = P::PlayerMonstersEpisode1V1;
const V2: P = P::PlayerMonstersEpisode1V2;

fn reference(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name);
    std::fs::read(path).expect("reference file")
}
fn wad(name: &str) -> Lumps {
    // The generator writes IWAD-headed files; the lump editor reads the same layout.
    let mut bytes = reference(&format!("m4b_wads/{name}.wad"));
    bytes[0] = b'P';
    Lumps::parse(&bytes)
}
fn map_of(l: &Lumps) -> doom_map::Map {
    let bytes = l.wad();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap()
}
fn lines(l: &mut Lumps) -> &mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == "LINEDEFS").unwrap().1
}
fn things(l: &mut Lumps) -> &mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1
}
fn load(l: &Lumps, policy: P) -> Result<Simulation, SimError> {
    let result = if policy == V2 {
        Simulation::original_player_episode1_v2_map(
            map_of(l),
            1,
            Skill::Medium,
            MapContextOptions::default(),
        )
    } else {
        Simulation::original_player_episode1_map(map_of(l), MapContextOptions::default())
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn setup(l: &Lumps) -> Simulation {
    let mut s = load(l, V2).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn demo(script: &str) -> Vec<TicCommand> {
    parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), 1)
        .unwrap()
        .1
        .commands
}
fn step(s: &mut Simulation, command: TicCommand) -> Result<(), SimError> {
    s.advance([
        command,
        TicCommand::default(),
        TicCommand::default(),
        TicCommand::default(),
    ])
}
/// Runs `commands[..tics]` (DSIM tics 1..=tics).
fn run_to(s: &mut Simulation, commands: &[TicCommand], tics: usize) {
    for (i, c) in commands[..tics].iter().enumerate() {
        step(s, *c).unwrap_or_else(|e| panic!("tic {}: {e:?}", i + 1));
    }
}
fn body(s: &Simulation) -> crate::Actor {
    *s.actor(s.original_player.unwrap().mo.unwrap()).unwrap()
}
fn of_type(s: &Simulation, ty: OriginalActorType) -> Vec<(ActorId, crate::Actor)> {
    s.actors
        .order()
        .filter(|id| s.actors.is_pending(*id) == Some(false))
        .map(|id| (id, *s.actor(id).unwrap()))
        .filter(|(_, a)| a.life.is_some_and(|l| l.actor_type.0 == ty.ordinal()))
        .collect()
}
fn teleport_events(s: &Simulation) -> Vec<MoverEvent> {
    s.events()
        .iter()
        .filter_map(|e| match e {
            Event::Mover(
                m @ (MoverEvent::Teleported { .. } | MoverEvent::TeleportNotTaken { .. }),
            ) => Some(*m),
            _ => None,
        })
        .collect()
}

/// p_inter.c:817 with damage 10,000 (decision I3): the product 8,192,000,000 wraps
/// to −397,934,592 and the signed division truncates toward zero. The values are
/// the M4b note's T8.5 (observed in the strict and strict32 builds).
#[test]
fn telefrag_thrust_is_the_wrapped_c_value() {
    use crate::combat::telefrag_thrust;
    assert_eq!(telefrag_thrust(10_000, 100).unwrap(), -3_979_345);
    assert_eq!(telefrag_thrust(10_000, 400).unwrap(), -994_836);
    assert_eq!(telefrag_thrust(10_000, 1000).unwrap(), -397_934);
    // Only the stomp's amount is computed this way.
    assert!(telefrag_thrust(9_999, 100).is_err());
    assert!(telefrag_thrust(1_000, 100).is_err());
    assert!(telefrag_thrust(10_000, 0).is_err());
}

/// Fog 2's offset, p_telept.c:113–115, against the M4b note's T2.9 (observed from
/// tables.c): angle 0 gives (+1,310,700, +500), 45° (+926,460, +927,160), 90°
/// (−500, +1,310,700); 180° and 270° follow by the table's symmetry.
#[test]
fn fog_offset_is_the_table_value() {
    let at = |octant: u32| fog_offset(octant.wrapping_mul(0x2000_0000)).map(Fixed::bits);
    assert_eq!(at(0), [1_310_700, 500]);
    assert_eq!(at(1), [926_460, 927_160]);
    assert_eq!(at(2), [-500, 1_310_700]);
    assert_eq!(at(4), [-1_310_700, -500]);
    assert_eq!(at(6), [500, -1_310_700]);
}

/// The crossing loop ends exactly when `P_TeleportMove` was called (p_map.c:152).
#[test]
fn only_a_reached_teleport_move_ends_the_crossing_loop() {
    assert!(!Outcome::BackSide.ends_crossings());
    assert!(!Outcome::NoLanding.ends_crossings());
    assert!(Outcome::Blocked.ends_crossings());
    assert!(Outcome::Teleported.ends_crossings());
}

/// Wire 24 admits line special 97 and the landing; wire 23 keeps refusing both
/// with its errors (`PursuitError::Specials` at load, `UnsupportedType(14)` at the
/// THING batch, state unchanged); 39, 125 and 126 stay refused under both wires
/// (decision I1). The landing spawns in state 0 (`S_NULL`) with tics −1, in
/// `spatial.link_order` (p_mobj.c:517) but in no sector list (NOSECTOR).
#[test]
fn wire_24_admits_97_and_the_landing() {
    let tele = wad("tele");
    let mut s = setup(&tele);
    let landings = of_type(&s, OriginalActorType::MT_TELEPORTMAN);
    assert!(!landings.is_empty());
    for (id, a) in &landings {
        let l = a.life.unwrap();
        assert_eq!((l.state, l.tics), (Some(crate::StateId(0)), -1));
        assert!(s.spatial.order.contains(id));
        assert!(!crate::lifecycle::in_sector_list(a));
        assert!(!crate::lifecycle::blocks_cell_index(a));
    }
    step(&mut s, TicCommand::default()).unwrap();
    assert_eq!(
        of_type(&s, OriginalActorType::MT_TELEPORTMAN).len(),
        landings.len()
    );
    // Wire 23.
    assert_eq!(
        load(&tele, V1).err(),
        Some(SimError::Pursuit(PursuitError::Specials))
    );
    // 39, 125 and 126 instead of 97, under both wires.
    for special in [39_i16, 125, 126] {
        let mut l = tele.clone();
        let v = lines(&mut l);
        for line in 0..v.len() / 14 {
            if i16::from_le_bytes([v[14 * line + 6], v[14 * line + 7]]) == LINE_TELEPORT {
                v[14 * line + 6..14 * line + 8].copy_from_slice(&special.to_le_bytes());
            }
        }
        for policy in [V1, V2] {
            assert_eq!(
                load(&l, policy).err(),
                Some(SimError::Pursuit(PursuitError::Specials)),
                "{special} {policy:?}"
            );
        }
    }
    // The landing alone (no 97 line) under wire 23.
    let mut l = tele.clone();
    let v = lines(&mut l);
    for line in 0..v.len() / 14 {
        if i16::from_le_bytes([v[14 * line + 6], v[14 * line + 7]]) == LINE_TELEPORT {
            v[14 * line + 6..14 * line + 8].copy_from_slice(&0_i16.to_le_bytes());
        }
    }
    let mut s = load(&l, V1).unwrap();
    let before = s.snapshot().canonical_bytes();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::MapSpawn(crate::MapSpawnError::UnsupportedType(
            14
        )))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
}

/// TELE1 (C-observed, validation/M4B.md): the back-side crossing at dump tic 34
/// does nothing (DSIM tic 35: sub-tag 16 reason 0); the front-side crossing at dump
/// tic 88 teleports (DSIM tic 89): `reactiontime` 18, momentum 0, two fogs and two
/// draws (one `lastlook` per fog, p_mobj.c:506). The next 18 `P_PlayerThink` calls
/// (DSIM tics 90–107) count it down instead of `P_MovePlayer` (p_user.c:267–270):
/// forward and turn commands are ignored and the body stays put; it moves again at
/// DSIM tic 108 (dump tic 107).
#[test]
fn player_teleport_and_the_freeze_boundary() {
    let commands = demo("TELE1");
    let mut s = setup(&wad("tele"));
    run_to(&mut s, &commands, 34);
    step(&mut s, commands[34]).unwrap();
    assert!(matches!(
        teleport_events(&s)[..],
        [MoverEvent::TeleportNotTaken { reason: 0, .. }]
    ));
    run_to(&mut s, &commands[35..], 88 - 35);
    let rng = s.rng.indices()[0];
    let fogs = of_type(&s, OriginalActorType::MT_TFOG).len();
    step(&mut s, commands[88]).unwrap();
    assert!(matches!(
        teleport_events(&s)[..],
        [MoverEvent::Teleported { .. }]
    ));
    let b = body(&s);
    assert_eq!(b.life.unwrap().reaction_time, PLAYER_TELEPORT_FREEZE);
    assert_eq!(b.momentum(), [Fixed::ZERO; 2]);
    assert_eq!(b.z, b.floor);
    assert_eq!(of_type(&s, OriginalActorType::MT_TFOG).len(), fogs + 2);
    assert_eq!(s.rng.indices()[0].wrapping_sub(rng), 2);
    let landed = (b.position(), b.angle());
    for k in 1..=18 {
        step(&mut s, commands[88 + k]).unwrap();
        let b = body(&s);
        assert_eq!(
            b.life.unwrap().reaction_time,
            18 - k as i32,
            "tic {}",
            89 + k
        );
        assert_eq!((b.position(), b.angle()), landed, "tic {}", 89 + k);
    }
    assert!(
        commands[89..=106]
            .iter()
            .any(|c| c.forward != 0 || c.angle_turn != 0),
        "the frozen tics receive movement commands"
    );
    step(&mut s, commands[107]).unwrap();
    assert_ne!((body(&s).position(), body(&s).angle()), landed, "tic 108");
}

/// MTELE1 (C-observed): the east Demon crosses its 97 line at dump tic 117 with a
/// live barrel on the landing. `PIT_StompThing` refuses a non-player's move on maps
/// 1–9 (p_map.c:102–103): no teleport, no fog, no draw from the teleport, and the
/// Demon stays where its walk took it (sub-tag 16 reason 2).
#[test]
fn blocked_monster_landing_changes_nothing() {
    let commands = demo("MTELE1");
    let mut s = setup(&wad("montele"));
    run_to(&mut s, &commands, 117);
    let fogs = of_type(&s, OriginalActorType::MT_TFOG);
    step(&mut s, commands[117]).unwrap();
    let events = teleport_events(&s);
    let [
        MoverEvent::TeleportNotTaken {
            actor, reason: 2, ..
        },
    ] = events[..]
    else {
        panic!("{events:?}");
    };
    let demon = *s.actor(actor).unwrap();
    assert_eq!(of_type(&s, OriginalActorType::MT_TFOG).len(), fogs.len());
    assert!(
        !s.events().iter().any(|e| matches!(e, Event::Spawned(_))),
        "a refused teleport spawns nothing"
    );
    // Not on any landing.
    for (_, landing) in of_type(&s, OriginalActorType::MT_TELEPORTMAN) {
        assert_ne!(landing.position(), demon.position());
    }
}

/// The store-full witness (decision I7): TELE1's teleport needs two free slots for
/// its fogs (`P_SpawnMobj` twice, p_telept.c:111–117). With one free slot the tic is
/// refused with `StoreError::Capacity` before any mutation of the teleport, the
/// canonical state is unchanged, and a retry refuses again; with two the tic runs.
#[test]
fn store_full_at_the_fogs_refuses_the_tic() {
    let commands = demo("TELE1");
    let mut base = setup(&wad("tele"));
    run_to(&mut base, &commands, 88);
    let fill = |s: &mut Simulation, free: usize| {
        let registry = s.registry.clone().unwrap();
        let fog = ActorTypeId(OriginalActorType::MT_TFOG.ordinal());
        let position = body(s).position();
        let live = s.actors.order().count();
        for _ in 0..(crate::player_map_scale::EPISODE1_ACTOR_SLOTS as usize - live - free) {
            let actor = crate::lifecycle::make_actor(
                &s.world,
                crate::staging::Sectors::committed(&s.sectors),
                &registry,
                s.actor_options,
                fog,
                position,
                SpawnHeight::Floor,
            );
            let id = s.actors.spawn(actor).unwrap();
            s.spatial.link(id);
        }
    };
    let mut full = base.clone();
    fill(&mut full, 1);
    let before = full.snapshot().canonical_bytes();
    assert_eq!(
        step(&mut full, commands[88]),
        Err(StoreError::Capacity.into())
    );
    assert_eq!(full.snapshot().canonical_bytes(), before);
    assert_eq!(
        step(&mut full, commands[88]),
        Err(StoreError::Capacity.into())
    );
    assert_eq!(full.snapshot().canonical_bytes(), before);
    let mut room = base.clone();
    fill(&mut room, 2);
    step(&mut room, commands[88]).unwrap();
    assert!(matches!(
        teleport_events(&room)[..],
        [MoverEvent::Teleported { .. }]
    ));
}

/// The transient allowance of wire 24: `slots − (1 + K + R + P + B + D + L)` from
/// the THINGS lump; the load refuses a map that leaves fewer than two slots.
#[test]
fn transient_allowance_formula() {
    let b = crate::Episode1Bounds {
        monsters: 10,
        droppers: 4,
        pickups: 20,
        keycards: 1,
        barrels: 3,
        statics: 5,
        countitems: 2,
    };
    assert_eq!(
        b.transient_allowance(2, 1024),
        Some(1024 - (1 + 10 + 4 + 20 + 3 + 5 + 2))
    );
    assert_eq!(b.transient_allowance(2, 45), Some(0));
    assert_eq!(b.transient_allowance(2, 44), None);
    let tele = wad("tele");
    let map = map_of(&tele);
    // Counted independently from the lump: doomednum 14 with the medium bit (2)
    // and without MTF_NOTSINGLE (16), p_mobj.c:740–751.
    let mut l = tele.clone();
    let expected = things(&mut l)
        .chunks_exact(10)
        .filter(|r| {
            let options = u16::from_le_bytes([r[8], r[9]]);
            i16::from_le_bytes([r[6], r[7]]) == 14 && options & 16 == 0 && options & 2 != 0
        })
        .count();
    assert!(expected > 0);
    assert_eq!(crate::Episode1Bounds::landings(&map), expected);
}

/// TFRAG (C-observed, validation/M4B.md) without its Baron (step 5): the
/// zombieman telefragged at dump tic 63 (DSIM 64) has health −9,980, its xdeath
/// state and momentum (−3,979,285, −1,518), the wrapped thrust applied through
/// `FixedMul` toward the player's pre-teleport position; the Demon at dump tic 139
/// (DSIM 140) has health −9,850 and momentum (−994,821, −380). Neither value reads
/// the RNG, so removing the Baron's THING changes neither. With the chainsaw ready
/// no thrust applies (p_inter.c:806–810): the zombieman keeps momentum 0.
#[test]
fn telefrag_without_the_baron() {
    let mut l = wad("tfrag");
    let t = things(&mut l);
    let kept: Vec<u8> = t
        .chunks_exact(10)
        .filter(|r| i16::from_le_bytes([r[6], r[7]]) != 3003)
        .flatten()
        .copied()
        .collect();
    assert!(kept.len() < t.len(), "TFRAG places a Baron");
    *t = kept;
    let commands = demo("TFRAG1");
    let victim = |s: &Simulation, ty| of_type(s, ty)[0].1;
    let victim_id = |s: &Simulation, ty| of_type(s, ty)[0].0;
    let mut s = setup(&l);
    run_to(&mut s, &commands, 63);
    let before_stomp = s.clone();
    step(&mut s, commands[63]).unwrap();
    let z = victim(&s, OriginalActorType::MT_POSSESSED);
    let zl = z.life.unwrap();
    assert_eq!(zl.health, -9_980);
    assert_eq!(
        zl.state,
        Some(crate::StateId(OriginalState::S_POSS_XDIE1.ordinal()))
    );
    assert_eq!(z.momentum().map(Fixed::bits), [-3_979_285, -1_518]);
    // The DSIM40 reader admits the capture: the damage record of 10,000 with
    // inflictor and source the player, the knockback and the health −9,980.
    let bytes = s.snapshot().canonical_bytes();
    crate::Diagnostic::decode_episode1_v2(&bytes, crate::DiagnosticLimits::default())
        .expect("telefrag capture");
    // The health domain's boundary: −9,999 is admitted, −10,000 is not.
    let slot = of_type(&s, OriginalActorType::MT_POSSESSED)[0].0.slot();
    let snap = s.snapshot();
    let with_health = |health: i32| -> Vec<u8> {
        let path = format!("actor.{slot}.health");
        let fields: Vec<(String, Vec<u8>)> = snap
            .fields()
            .map(|f| {
                let value = if f.path() == path {
                    health.to_le_bytes().to_vec()
                } else {
                    f.value().to_vec()
                };
                (f.path().to_string(), value)
            })
            .collect();
        let mut out = b"DSIM".to_vec();
        out.extend(snap.version().to_le_bytes());
        out.extend((fields.len() as u64).to_le_bytes());
        for (p, v) in fields {
            out.extend((p.len() as u32).to_le_bytes());
            out.extend(p.as_bytes());
            out.extend((v.len() as u64).to_le_bytes());
            out.extend(v);
        }
        out
    };
    let decode = |bytes: &[u8]| {
        crate::Diagnostic::decode_episode1_v2(bytes, crate::DiagnosticLimits::default()).map(|_| ())
    };
    assert_eq!(decode(&with_health(-9_999)), Ok(()));
    assert_eq!(
        decode(&with_health(-10_000)),
        Err(crate::DiagnosticError::Identity)
    );
    run_to(&mut s, &commands[64..], 139 - 64);
    step(&mut s, commands[139]).unwrap();
    let d = victim(&s, OriginalActorType::MT_SERGEANT);
    assert_eq!(d.life.unwrap().health, -9_850);
    assert_eq!(d.momentum().map(Fixed::bits), [-994_821, -380]);
    // The chainsaw ready: the same damage, planned and applied in the state before
    // the stomp with only the player's ready weapon changed (the plan reads nothing
    // else of the weapon), gives no knockback.
    let after = |chainsaw: bool| {
        let mut t = before_stomp.clone();
        let target = victim_id(&t, OriginalActorType::MT_POSSESSED);
        let mo = t.original_player.unwrap().mo.unwrap();
        t.checked_demon_operation(|host| {
            if chainsaw {
                if let Some(crate::player_weapons::PlayerWeaponRuntime::Pack(w)) =
                    host.player.as_deref_mut().and_then(|p| p.weapon.as_mut())
                {
                    w.ready = crate::Weapon::Chainsaw;
                }
            }
            let request = crate::DamageRequest {
                target,
                inflictor: Some(mo),
                source: Some(mo),
                amount: TELEFRAG_DAMAGE,
            };
            let plan = crate::combat::DamagePlan::telefrag(host, request).unwrap();
            host.apply_damage(plan);
        })
        .unwrap();
        t
    };
    let saw = after(true);
    assert_ne!(
        victim(&after(false), OriginalActorType::MT_POSSESSED).momentum(),
        [Fixed::ZERO; 2]
    );
    let z = victim(&saw, OriginalActorType::MT_POSSESSED);
    assert_eq!(z.life.unwrap().health, -9_980);
    assert_eq!(z.momentum(), [Fixed::ZERO; 2]);
}

/// Damage above 1,000 stays refused outside the stomp (decision I3), under wire
/// 24 and under wire 23: `damage_actor` with 10,000 is `CombatError::Damage`, and
/// the state is unchanged.
#[test]
fn damage_above_1000_stays_refused_outside_the_stomp() {
    let mut room = crate::player_movers_tests::room(13);
    room.add_thing([1000, 64, 180, 1, 7]);
    let mut wire23 =
        Simulation::original_player_episode1_map(room.map(), MapContextOptions::default()).unwrap();
    wire23.spawn_map_things().unwrap();
    wire23
        .initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    wire23.spawn_level_specials().unwrap();
    for mut s in [setup(&wad("tele")), wire23] {
        let mo = s.original_player.unwrap().mo.unwrap();
        let before = s.snapshot().canonical_bytes();
        for amount in [1_001, TELEFRAG_DAMAGE] {
            assert_eq!(
                s.damage_actor(crate::DamageRequest {
                    target: mo,
                    inflictor: None,
                    source: None,
                    amount,
                }),
                Err(crate::CombatError::Damage.into())
            );
            assert_eq!(s.snapshot().canonical_bytes(), before);
        }
    }
}
