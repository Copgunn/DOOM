//! The Baron of Hell and `A_BossDeath` under wire 24 (M4b step 5). Expected values
//! come from the C source (cited), from the genuine engine's runs recorded in
//! docs/validation/M4B.md ("C-observed", dump tic N is DSIM tic N + 1), or from
//! written derivations; never from Rust output. The tic-for-tic comparison of the
//! Baron scripts against the genuine goldens is
//! `tests/episode1v2_mechanism_goldens.rs`; these tests pin the individual
//! mechanisms on those runs and on edited copies of their WADs.
use super::*;
use crate::diagonal_room::Lumps;
use crate::identity::OriginalState;
use crate::{
    ActorTypeId, CombatEvent, DamageRequest, Fixed, MapContextOptions, MapSpawnError, MonsterEvent,
    OriginalExecutionProfile as P, OriginalRegistryError, PlayerSlot, ProjectileEvent, Simulation,
    Skill, TicCommand, WeaponInventory, demo::parse_demo_episode1_v2, episode1::Episode1Bounds,
    original_registry,
};

const V1: P = P::PlayerMonstersEpisode1V1;
const V2: P = P::PlayerMonstersEpisode1V2;
const BARON: OriginalActorType = OriginalActorType::MT_BRUISER;
const BALL: OriginalActorType = OriginalActorType::MT_BRUISERSHOT;

fn reference(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name);
    std::fs::read(path).expect("reference file")
}
fn wad(name: &str) -> Lumps {
    let mut bytes = reference(&format!("m4b_wads/{name}.wad"));
    bytes[0] = b'P';
    Lumps::parse(&bytes)
}
fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn map_of(l: &Lumps, slot: &str) -> doom_map::Map {
    let bytes = l.wad();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, slot, doom_map::MapLimits::default()).unwrap()
}
fn setup(l: &Lumps, slot: &str, map_number: u8) -> Simulation {
    let mut s = Simulation::original_player_episode1_v2_map(
        map_of(l, slot),
        map_number,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn demo(script: &str, map_number: u8) -> Vec<TicCommand> {
    parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), map_number)
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
fn of_type(s: &Simulation, ty: OriginalActorType) -> Vec<(ActorId, crate::Actor)> {
    s.actors
        .order()
        .filter(|id| s.actors.is_pending(*id) == Some(false))
        .map(|id| (id, *s.actor(id).unwrap()))
        .filter(|(_, a)| a.life.is_some_and(|l| l.actor_type.0 == ty.ordinal()))
        .collect()
}
fn is_type(s: &Simulation, id: ActorId, ty: OriginalActorType) -> bool {
    s.actor(id)
        .and_then(|a| a.life)
        .is_some_and(|l| l.actor_type.0 == ty.ordinal())
}
fn boss_events(s: &Simulation) -> Vec<(ActorId, u8)> {
    s.events()
        .iter()
        .filter_map(|e| match e {
            Event::Mover(MoverEvent::BossDeath { actor, outcome }) => Some((*actor, *outcome)),
            _ => None,
        })
        .collect()
}
/// Every `A_BossDeath` of a script: (DSIM tic, outcome), with the tic's floor calls.
fn boss_run(
    l: &Lumps,
    slot: &str,
    map_number: u8,
    script: &str,
) -> Vec<(usize, u8, Vec<MoverEvent>)> {
    let commands = demo(script, map_number);
    let mut s = setup(l, slot, map_number);
    let mut out = Vec::new();
    for (i, c) in commands.iter().enumerate() {
        match step(&mut s, *c) {
            Ok(()) => {}
            Err(SimError::LevelExited) => break,
            Err(e) => panic!("{script} tic {}: {e:?}", i + 1),
        }
        for (actor, outcome) in boss_events(&s) {
            assert!(is_type(&s, actor, BARON));
            let calls = s
                .events()
                .iter()
                .filter_map(|e| match e {
                    Event::Mover(m @ (MoverEvent::Call { .. } | MoverEvent::Started { .. })) => {
                        Some(*m)
                    }
                    _ => None,
                })
                .collect();
            out.push((i + 1, outcome, calls));
        }
    }
    out
}

/// `snap`'s canonical bytes with one field's value replaced.
fn with_field(snap: &crate::Snapshot, path: &str, value: &[u8]) -> Vec<u8> {
    let mut out = b"DSIM".to_vec();
    out.extend(snap.version().to_le_bytes());
    out.extend((snap.fields().count() as u64).to_le_bytes());
    for f in snap.fields() {
        let v = if f.path() == path { value } else { f.value() };
        out.extend((f.path().len() as u32).to_le_bytes());
        out.extend(f.path().as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    }
    out
}

/// Wire 24 admits doomednum 3003 with its closure, raise states excluded; wire 23
/// keeps refusing it with the unchanged error and state (the THING batch changes
/// nothing when it refuses). `K` counts it under wire 24 only.
#[test]
fn wire_24_admits_the_baron_wire_23_refuses_it() {
    let l = wad("baron");
    let map = map_of(&l, "E1M1");
    let v1 = Episode1Bounds::from_map(&map);
    let v2 = Episode1Bounds::from_map_v2(&map, Skill::Medium);
    let placed = map.things().iter().filter(|t| t.kind == 3003).count();
    assert_eq!(placed, 1);
    assert_eq!(v2.monsters, v1.monsters + 1);
    assert_eq!(v2.droppers, v1.droppers);
    let mut s =
        Simulation::original_player_episode1_map(map_of(&l, "E1M1"), MapContextOptions::default())
            .unwrap();
    let before = s.snapshot().canonical_bytes();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(3003)))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
    let s = setup(&l, "E1M1", 1);
    assert_eq!(of_type(&s, BARON).len(), 1);
    assert_eq!(s.episode1_bounds(), Some(v2));
    let baron = ActorTypeId(BARON.ordinal());
    let ball = ActorTypeId(BALL.ordinal());
    for state in OriginalState::all() {
        let id = crate::StateId(state.ordinal());
        let n = state.ordinal();
        // `S_BOSS_STND` (527) … `S_BOSS_DIE7` (548); `S_BOSS_RAISE1`–`7` are
        // 549–555; `S_BRBALL1` (522) … `S_BRBALLX3` (526) (info.h).
        let own = (527..=548).contains(&n);
        assert_eq!(
            original_registry::state_role_for(V2, baron, id).is_ok(),
            own,
            "{n}"
        );
        assert_eq!(
            original_registry::state_role_for(V2, ball, id).is_ok(),
            (522..=526).contains(&n),
            "{n}"
        );
        assert!(original_registry::state_role_for(V1, baron, id).is_err());
        assert!(original_registry::state_role_for(V1, ball, id).is_err());
    }
    assert!(original_registry::is_policy_monster(V2, baron));
    assert!(!original_registry::is_policy_monster(V1, baron));
}

/// The DSIM40 reader refuses a Baron in a raise state (`S_BOSS_RAISE1`, info.c:685;
/// reached only through the arch-vile) and admits its live states.
#[test]
fn reader_refuses_baron_raise_states() {
    let l = wad("baron");
    let s = setup(&l, "E1M1", 1);
    let slot = of_type(&s, BARON)[0].0.slot();
    let snap = s.snapshot();
    let bytes = snap.canonical_bytes();
    crate::Diagnostic::decode_episode1_v2(&bytes, crate::DiagnosticLimits::default())
        .expect("load capture");
    let path = format!("actor.{slot}.state");
    for (state, ok) in [
        (OriginalState::S_BOSS_RUN1, true),
        (OriginalState::S_BOSS_DIE7, true),
        (OriginalState::S_BOSS_RAISE1, false),
        (OriginalState::S_BOSS_RAISE7, false),
        (OriginalState::S_BOS2_RUN1, false),
    ] {
        let edited = with_field(&snap, &path, &state.ordinal().to_le_bytes());
        assert_eq!(
            crate::Diagnostic::decode_episode1_v2(&edited, crate::DiagnosticLimits::default())
                .is_ok(),
            ok,
            "{state:?}"
        );
    }
}

/// The Baron fireball's record (info.c:1524–1548): speed 15·FRACUNIT, damage 8,
/// radius 6, height 8; the Imp's is speed 10, damage 3 (info.c:1914–1938). No
/// fireball step splits: `P_XYMovement` splits only above `MAXMOVE/2`
/// (p_mobj.c:152; 30·FRACUNIT/2 = 983,040), and the largest component is
/// `FixedMul(15·FRACUNIT, 65535)` = 983,025 (the largest `finesine` entry,
/// tables.c), strictly below.
#[test]
fn baron_fireball_record_and_no_split() {
    let d = BALL.descriptor();
    assert_eq!(d.speed, 15 * 65536);
    assert_eq!(d.damage, 8);
    assert_eq!(d.radius, 6 * 65536);
    assert_eq!(d.height, 8 * 65536);
    assert_eq!(
        OriginalActorType::MT_TROOPSHOT.descriptor().speed,
        10 * 65536
    );
    assert_eq!(OriginalActorType::MT_TROOPSHOT.descriptor().damage, 3);
    let largest = crate::FINE_SINE.iter().map(|v| v.abs()).max().unwrap();
    assert_eq!(largest, 65535);
    let step = (Fixed::from_bits(15 * 65536) * Fixed::from_bits(largest)).bits();
    assert_eq!(step, 983_025);
    assert!(step < 30 * 65536 / 2);
}

/// BARON1, C-observed: a fireball at dump tic 64 (DSIM 65). `A_BruisAttack` at
/// `S_BOSS_ATK3` does not face (no `A_FaceTarget` record in that call,
/// p_enemy.c:979–996); `P_SpawnMissile` launches `MT_BRUISERSHOT` with the Baron as
/// its target, momentum `FixedMul(15·FRACUNIT, finecosine/finesine[angle >> 19])`
/// (p_mobj.c:912–915), and its only draws besides `lastlook` are the spawn tics
/// (the player is not `MF_SHADOW`: no pair, p_mobj.c:909–910).
#[test]
fn baron_fireball_launch() {
    let commands = demo("BARON1", 1);
    let mut s = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut s, &commands, 64);
    let baron = of_type(&s, BARON)[0].0;
    step(&mut s, commands[64]).unwrap();
    let mut launched = None;
    let mut draws = Vec::new();
    let mut after_launch = false;
    for e in s.events() {
        match e {
            Event::Projectile(ProjectileEvent::Launch(r)) => {
                assert_eq!(r.source, baron);
                assert_eq!(r.actor_type, ActorTypeId(BALL.ordinal()));
                launched = Some(r.source);
                after_launch = true;
            }
            Event::Monster(MonsterEvent::Face { actor, .. }) if after_launch => {
                assert_ne!(*actor, baron, "A_BruisAttack faced");
            }
            Event::Projectile(ProjectileEvent::Random { kind, .. }) if after_launch => {
                draws.push(*kind)
            }
            Event::Projectile(ProjectileEvent::Velocity {
                angle, momentum, ..
            }) if after_launch => {
                let phase = (*angle >> 19) as usize;
                let speed = Fixed::from_bits(15 * 65536);
                assert_eq!(
                    momentum[0],
                    speed * Fixed::from_bits(crate::FINE_SINE[phase + 2048])
                );
                assert_eq!(
                    momentum[1],
                    speed * Fixed::from_bits(crate::FINE_SINE[phase])
                );
            }
            _ => {}
        }
    }
    assert_eq!(launched, Some(baron));
    assert_eq!(draws.first(), Some(&0), "spawn tics draw");
    assert!(!draws.contains(&1) && !draws.contains(&2), "no shadow pair");
    let ball = of_type(&s, BALL);
    assert_eq!(ball.len(), 1);
    assert_eq!(ball[0].1.life.unwrap().target, Some(baron));
}

/// BARON1, C-observed: the claw at dump tic 505 (DSIM 506), no fireball spawned.
/// `S_StartSound (actor, sfx_claw)`, one draw, `P_DamageMobj (target, actor, actor,
/// (P_Random()%8+1)*10)` (p_enemy.c:986–991).
#[test]
fn baron_claw() {
    let commands = demo("BARON1", 1);
    let mut s = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut s, &commands, 505);
    let baron = of_type(&s, BARON)[0].0;
    let player = s.original_player.unwrap().mo.unwrap();
    let balls_before = of_type(&s, BALL).len();
    step(&mut s, commands[505]).unwrap();
    let claw = crate::lifecycle::source_sound("sfx_claw").unwrap();
    let events = s.events();
    let sound = events
        .iter()
        .position(|e| {
            matches!(e, Event::ActorState(crate::ActorStateEvent::Sound { actor, sound })
                if *actor == Some(baron) && *sound == claw)
        })
        .expect("claw sound");
    let value = events[sound..]
        .iter()
        .find_map(|e| match e {
            Event::Monster(MonsterEvent::Random { actor, value, .. }) if *actor == baron => {
                Some(*value)
            }
            _ => None,
        })
        .expect("damage draw");
    let damage = events[sound..]
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Damage(r)) => Some(*r),
            _ => None,
        })
        .expect("damage");
    assert_eq!(
        damage,
        DamageRequest {
            target: player,
            inflictor: Some(baron),
            source: Some(baron),
            amount: (i32::from(value) % 8 + 1) * 10,
        }
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
    );
    assert!(of_type(&s, BALL).len() <= balls_before);
}

/// BARON1, C-observed: the fireball hits the player at dump tic 119 (DSIM 120):
/// contact damage `((P_Random()%8)+1)*8` (p_map.c:323) with inflictor the ball and
/// source the Baron, then the explosion tics draw (p_mobj.c:96).
#[test]
fn baron_fireball_contact_damage() {
    let commands = demo("BARON1", 1);
    let mut s = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut s, &commands, 119);
    let baron = of_type(&s, BARON)[0].0;
    let player = s.original_player.unwrap().mo.unwrap();
    step(&mut s, commands[119]).unwrap();
    let events = s.events();
    let contact = events
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::Projectile(ProjectileEvent::Random { kind: 3, .. })
            )
        })
        .expect("contact draw");
    let Event::Projectile(ProjectileEvent::Random { value, .. }) = events[contact] else {
        unreachable!()
    };
    let damage = events[contact..]
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Damage(r)) => Some(*r),
            _ => None,
        })
        .expect("damage");
    assert_eq!(damage.target, player);
    assert_eq!(damage.source, Some(baron));
    assert!(is_type(&s, damage.inflictor.unwrap(), BALL));
    assert_eq!(damage.amount, (i32::from(value) % 8 + 1) * 8);
    assert!(events[contact..].iter().any(|e| matches!(
        e,
        Event::Projectile(ProjectileEvent::Random { kind: 4, .. })
    )));
}

/// BVB1, C-observed: a Baron fireball bursts on the other Baron at dump tic 322
/// (DSIM 323) without damage (p_map.c:299–313, "Explode, but do no damage"): an
/// explosion, no contact draw and no damage record in that tic, the ledge
/// Baron's health unchanged.
#[test]
fn baron_fireball_never_hurts_a_baron() {
    let commands = demo("BVB1", 1);
    let mut s = setup(&wad("bvb"), "E1M1", 1);
    run_to(&mut s, &commands, 322);
    let health = |s: &Simulation| {
        of_type(s, BARON)
            .iter()
            .map(|(_, a)| a.life.unwrap().health)
            .collect::<Vec<_>>()
    };
    let before = health(&s);
    step(&mut s, commands[322]).unwrap();
    assert_eq!(health(&s), before);
    let exploded: Vec<ActorId> = s
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Projectile(ProjectileEvent::Exploded(id)) => Some(*id),
            _ => None,
        })
        .collect();
    assert!(exploded.iter().any(|id| is_type(&s, *id, BALL)));
    assert!(!s.events().iter().any(|e| matches!(
        e,
        Event::Projectile(ProjectileEvent::Random { kind: 3, .. })
            | Event::Combat(CombatEvent::Damage(_))
    )));
}

/// BINF1, C-observed: a fireball aimed at the player hits the Spectre at dump tic
/// 81 (DSIM 82), which then targets the Baron (infighting: `P_DamageMobj` retargets
/// a surviving victim with threshold 0 to the source, p_inter.c:904–915).
#[test]
fn baron_fireball_hits_a_spectre_which_retargets() {
    let commands = demo("BINF1", 1);
    let mut s = setup(&wad("infight"), "E1M1", 1);
    run_to(&mut s, &commands, 81);
    let baron = of_type(&s, BARON)[0].0;
    let spectre = of_type(&s, OriginalActorType::MT_SHADOWS)[0].0;
    step(&mut s, commands[81]).unwrap();
    let damage = s
        .events()
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Damage(r)) => Some(*r),
            _ => None,
        })
        .expect("damage");
    assert_eq!(damage.target, spectre);
    assert_eq!(damage.source, Some(baron));
    let life = s.actor(spectre).unwrap().life.unwrap();
    assert_eq!(life.target, Some(baron));
    // `threshold = BASETHRESHOLD` (100, p_local.h:61); the Spectre's own A_Chase
    // later in the tic may count it down once (p_enemy.c:682–690).
    assert!((99..=100).contains(&life.threshold), "{}", life.threshold);
}

/// `A_Chase` (p_enemy.c:725–749): the melee branch needs no `movecount == 0` and
/// sets no `MF_JUSTATTACKED`; the missile branch needs `movecount == 0` and sets
/// it. Over the three Baron scripts every Baron selection is checked against the
/// Baron's `movecount` at the end of the previous tic (A_Chase does not change it
/// before the branches) and the flag record of the same call.
#[test]
fn chase_melee_and_missile_branches_differ() {
    let mut kinds = [0; 2];
    for (script, name) in [("BARON1", "baron"), ("BINF1", "infight"), ("BVB1", "bvb")] {
        let commands = demo(script, 1);
        let mut s = setup(&wad(name), "E1M1", 1);
        for c in commands {
            let before: Vec<(ActorId, i32)> = of_type(&s, BARON)
                .iter()
                .map(|(id, a)| (*id, a.life.unwrap().movecount))
                .collect();
            step(&mut s, c).unwrap();
            let events = s.events();
            for (i, e) in events.iter().enumerate() {
                let Event::Monster(MonsterEvent::Selection { actor, kind, .. }) = e else {
                    continue;
                };
                let Some(&(_, movecount)) = before.iter().find(|(id, _)| id == actor) else {
                    continue;
                };
                let flagged = events[i + 1..].iter().any(|e| {
                    matches!(e,
                        Event::Monster(MonsterEvent::JustAttacked { actor: a, .. }) if a == actor)
                });
                kinds[usize::from(*kind)] += 1;
                if *kind == 1 {
                    assert_eq!(movecount, 0, "{script}");
                    assert!(flagged, "{script}");
                } else {
                    assert!(!flagged, "{script}");
                }
            }
        }
    }
    assert!(kinds[0] > 0 && kinds[1] > 0, "{kinds:?}");
}

/// `A_BossDeath`'s checks in order (p_enemy.c:1629–1705): the map first, then the
/// live player, then the thinker walk, which runs only when both pass.
#[test]
fn boss_death_checks_in_order() {
    let never = || -> Result<bool, SimError> { panic!("walk ran") };
    assert_eq!(decide(1, 100, never).unwrap(), Outcome::NotBossMap);
    assert_eq!(decide(1, 0, never).unwrap(), Outcome::NotBossMap);
    assert_eq!(decide(9, 100, never).unwrap(), Outcome::NotBossMap);
    assert_eq!(decide(8, 0, never).unwrap(), Outcome::NoLivePlayer);
    assert_eq!(decide(8, -5, never).unwrap(), Outcome::NoLivePlayer);
    assert_eq!(decide(8, 1, || Ok(true)).unwrap(), Outcome::OtherBossAlive);
    assert_eq!(decide(8, 1, || Ok(false)).unwrap(), Outcome::FloorCalled);
    assert!(decide(8, 1, || Err(StoreError::UnsupportedThinker.into())).is_err());
}

/// BOSS1 and BOSSM1, C-observed: both Barons enter DIE7 at dump tics 112 and 223
/// (DSIM 113 and 224; 48 tics after the telefrags at 64 and 175, §7 finding 6).
/// On E1M8 the first finds the other alive and the second starts the tag-666
/// `lowerFloorToLowest`: one floor (kind 7), the call record naming no line
/// (65535), function 4, argument 1, result 1. In the E1M1 slot nothing happens.
#[test]
fn boss_death_on_e1m8_and_elsewhere() {
    let boss = boss_run(&wad("boss8"), "E1M8", 8, "BOSS1");
    assert_eq!(
        boss.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(113, 2), (224, 3)]
    );
    assert!(boss[0].2.is_empty());
    let calls = &boss[1].2;
    assert!(matches!(
        calls[..],
        [
            MoverEvent::Started { kind: 7, .. },
            MoverEvent::Call {
                function: crate::planes::MoverFunction::DoFloor,
                line: u16::MAX,
                argument: 1,
                result: 1,
                created: 1,
            }
        ]
    ));
    let other = boss_run(&wad("bossm1"), "E1M1", 1, "BOSSM1");
    assert_eq!(
        other.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(113, 0), (224, 0)]
    );
    assert!(other.iter().all(|(_, _, c)| c.is_empty()));
    // BARON1 (death at dump tic 688, DIE7 at 736) and TFRAG1 (DIE7 at 271), E1M1.
    let baron = boss_run(&wad("baron"), "E1M1", 1, "BARON1");
    assert_eq!(
        baron.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(737, 0)]
    );
    let tfrag = boss_run(&wad("tfrag"), "E1M1", 1, "TFRAG1");
    assert_eq!(
        tfrag.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(272, 0)]
    );
}

/// BOSS2, C-observed: the W1 raiseFloor on tag 666 runs when the second Baron's
/// DIE7 comes, so `EV_DoFloor` skips the busy sector (p_floor.c:277–278): the call
/// is made and creates nothing (result 0).
#[test]
fn boss_death_skips_a_busy_sector() {
    let boss = boss_run(&wad("boss8"), "E1M8", 8, "BOSS2");
    assert_eq!(
        boss.iter().map(|(t, o, _)| (*t, *o)).collect::<Vec<_>>(),
        [(113, 2), (224, 3)]
    );
    assert!(matches!(
        boss[1].2[..],
        [MoverEvent::Call {
            line: u16::MAX,
            result: 0,
            created: 0,
            ..
        }]
    ));
}

/// BOSS1 with the tag-666 sector's tag cleared: no sector carries tag 666, so
/// `P_FindSectorFromLineTag` finds none and the call creates nothing
/// (p_spec.c:434–445, p_floor.c:270–283). Rust-only edit of the C-run WAD.
#[test]
fn boss_death_without_a_tagged_sector() {
    let mut l = wad("boss8");
    let sectors = lump(&mut l, "SECTORS");
    let mut cleared = 0;
    for r in sectors.chunks_exact_mut(26) {
        if i16::from_le_bytes([r[24], r[25]]) == 666 {
            r[24..26].copy_from_slice(&0_i16.to_le_bytes());
            cleared += 1;
        }
    }
    assert_eq!(cleared, 1);
    let boss = boss_run(&l, "E1M8", 8, "BOSS1");
    assert_eq!(boss.last().map(|(t, o, _)| (*t, *o)), Some((224, 3)));
    assert!(matches!(
        boss.last().unwrap().2[..],
        [MoverEvent::Call {
            result: 0,
            created: 0,
            ..
        }]
    ));
}

/// BOSS1, C-observed: the exit at dump tic 544 completes (DSIM 545) through sector
/// special 11 and every later tic is refused with `LevelExited`, state preserved
/// (as in M4a).
#[test]
fn e1m8_ends_through_sector_11() {
    let commands = demo("BOSS1", 8);
    let mut s = setup(&wad("boss8"), "E1M8", 8);
    run_to(&mut s, &commands, 545);
    let before = s.snapshot().canonical_bytes();
    assert_eq!(step(&mut s, commands[545]), Err(SimError::LevelExited));
    assert_eq!(s.snapshot().canonical_bytes(), before);
}

/// The store-full witness on the Baron's fireball: BARON1's launch tic (DSIM 65)
/// with no free slot is refused with `StoreError::Capacity` before the launch
/// changes anything, the canonical state is unchanged, and a retry refuses again;
/// with one free slot the tic runs.
#[test]
fn store_full_at_the_fireball_refuses_the_tic() {
    let commands = demo("BARON1", 1);
    let mut base = setup(&wad("baron"), "E1M1", 1);
    run_to(&mut base, &commands, 64);
    let fill = |s: &mut Simulation, free: usize| {
        let registry = s.registry.clone().unwrap();
        let fog = ActorTypeId(OriginalActorType::MT_TFOG.ordinal());
        let position = s
            .actor(s.original_player.unwrap().mo.unwrap())
            .unwrap()
            .position();
        let live = s.actors.order().count();
        for _ in 0..(crate::player_map_scale::EPISODE1_ACTOR_SLOTS as usize - live - free) {
            let actor = crate::lifecycle::make_actor(
                &s.world,
                crate::staging::Sectors::committed(&s.sectors),
                &registry,
                s.actor_options,
                fog,
                position,
                crate::SpawnHeight::Floor,
            );
            let id = s.actors.spawn(actor).unwrap();
            s.spatial.link(id);
        }
    };
    let mut full = base.clone();
    fill(&mut full, 0);
    let before = full.snapshot().canonical_bytes();
    for _ in 0..2 {
        assert_eq!(
            step(&mut full, commands[64]),
            Err(StoreError::Capacity.into())
        );
        assert_eq!(full.snapshot().canonical_bytes(), before);
    }
    let mut room = base.clone();
    fill(&mut room, 1);
    step(&mut room, commands[64]).unwrap();
    assert_eq!(of_type(&room, BALL).len(), 1);
}

/// `launch_missile` refuses any type but the Imp's and, under wire 24, the
/// Baron's, and `A_BruisAttack`'s callback is wire 24's alone.
#[test]
fn baron_paths_are_wire_24_only() {
    assert!(V2.has_baron() && V2.has_boss_death());
    for p in [V1, P::PlayerMonstersLevelV1, P::PlayerMonstersImpV1] {
        assert!(!p.has_baron() && !p.has_boss_death(), "{p:?}");
        assert!(original_registry::require_role_for(p, ActorTypeId(BARON.ordinal())).is_err());
        assert!(original_registry::require_role_for(p, ActorTypeId(BALL.ordinal())).is_err());
    }
    let _ = OriginalRegistryError::Graph;
}
/// A contact during `P_CheckMissileSpawn`'s half step (p_mobj.c:868–881; M4b note
/// §3 fact 8, correction 2): the contact draws the damage (p_map.c:323), runs
/// `P_DamageMobj` (with its pain draw for a surviving victim, p_inter.c:894) and
/// fails the move, so `P_ExplodeMissile` draws as well (p_mobj.c:96), in that order,
/// all in the launch tic. Rust-only setup: BINF1 just before the Baron's first
/// fireball, with the Spectre moved 30 units in front of the Baron on the aim line
/// (the player stays out of melee range, so `A_BruisAttack` fires).
#[test]
fn spawn_step_hit_draws_damage_pain_and_explosion() {
    let commands = demo("BINF1", 1);
    let mut s = setup(&wad("infight"), "E1M1", 1);
    let mut tic = 0;
    loop {
        let mut probe = s.clone();
        step(&mut probe, commands[tic]).unwrap();
        if probe
            .events()
            .iter()
            .any(|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(_))))
        {
            break;
        }
        s = probe;
        tic += 1;
    }
    let (baron, b) = of_type(&s, BARON)[0];
    let (spectre, _) = of_type(&s, OriginalActorType::MT_SHADOWS)[0];
    let target = s.actor(b.life.unwrap().target.unwrap()).unwrap().position();
    let angle =
        crate::movement::point_angle([target[0] - b.position()[0], target[1] - b.position()[1]]);
    let phase = (angle >> 19) as usize;
    let thirty = Fixed::from_bits(30 * 65536);
    let position = [
        b.position()[0] + thirty * Fixed::from_bits(crate::FINE_SINE[phase + 2048]),
        b.position()[1] + thirty * Fixed::from_bits(crate::FINE_SINE[phase]),
    ];
    {
        let a = s.actors.get_mut(spectre).unwrap();
        a.spec.position = position;
        a.z = b.z;
    }
    s.spatial.link(spectre);
    step(&mut s, commands[tic]).unwrap();
    let events = s.events();
    let at = |f: &dyn Fn(&Event) -> bool| events.iter().position(f);
    let launch =
        at(&|e| matches!(e, Event::Projectile(ProjectileEvent::Launch(r)) if r.source == baron))
            .expect("launch");
    let contact = at(&|e| {
        matches!(
            e,
            Event::Projectile(ProjectileEvent::Random { kind: 3, .. })
        )
    })
    .expect("contact draw");
    let damage = at(&|e| {
        matches!(e, Event::Combat(CombatEvent::Damage(r))
        if r.target == spectre && r.source == Some(baron))
    })
    .expect("damage");
    let pain = at(&|e| matches!(e, Event::Combat(CombatEvent::Random { kind: 1, .. })))
        .expect("pain draw");
    let explode = at(&|e| {
        matches!(
            e,
            Event::Projectile(ProjectileEvent::Random { kind: 4, .. })
        )
    })
    .expect("explosion draw");
    assert!(launch < contact && contact < damage && damage < pain && pain < explode);
}
