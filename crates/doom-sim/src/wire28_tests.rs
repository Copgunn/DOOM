//! Wire 28 (`PlayerMonstersEpisode1V6`, DSIM44): Rust-only safety tests of the
//! easy skills (ruling U12 (iii)) and of spawn rule S″ with the monster escape
//! witness (§1.17 option 1). Expected values are computed by hand from the C text
//! cited at each test (p_mobj.c:743-751, p_inter.c:73-160 and 775-916,
//! p_mobj.c:137-189), or are the same quantity computed under wire 27 or at skill
//! 2; never from the function under test alone. No genuine C trace is compared
//! here: that evidence is a later step.
use crate::barrels_replay::{cases, room};
use crate::combat::DamagePlan;
use crate::diagonal_room::Lumps;
use crate::identity::{OriginalActorType as T, OriginalState};
use crate::lifecycle::ActorAccess as _;
use crate::movement::MotionAccess as _;
use crate::player_weapons::PlayerWeaponRuntime;
use crate::*;

const V5: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V5;
const V6: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V6;

fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
/// The wire-26 unit-test room of `wire27_tests` (a 1,024 × 256 rectangle, the
/// player at (-256, 0) facing east), with `extra` THING records appended.
fn base(extra: &[[i16; 5]]) -> Lumps {
    let c = &cases()[0];
    let mut l = room(c.map);
    let t = &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1;
    t.splice(0..0, record(c.position[0], c.position[1], 0, 1, 7));
    for e in extra {
        t.extend(record(e[0], e[1], e[2], e[3], e[4]));
    }
    l
}
/// The M3h9b corridor hall (map 6): the hall `x ∈ [0, 512]`, `y ∈ [−320, 320]`,
/// entered at its west wall `x = 0` by the corridor `x ∈ [−512, 0]`,
/// `y ∈ [−32, 32]`; the void `x < 0`, `|y| > 32` lies inside the vertex
/// rectangle and outside the boundary. The player starts at `player`.
fn hall(player: [i16; 2], extra: &[[i16; 5]]) -> Lumps {
    let mut l = room(6);
    l.clear_things();
    l.add_thing([player[0], player[1], 0, 1, 7]);
    for e in extra {
        l.add_thing(*e);
    }
    l
}
type Built = Result<Simulation, LevelStartError>;
fn load(policy: OriginalExecutionProfile, l: &Lumps, skill: Skill) -> Built {
    let build = match policy {
        V5 => Simulation::original_player_episode1_v5_level,
        V6 => Simulation::original_player_episode1_v6_level,
        _ => Simulation::original_player_episode1_v4_level,
    };
    build(l.map(), 1, skill, Default::default(), LevelStart::NewGame)
}
fn idle() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn of_type(s: &Simulation, ty: T) -> Vec<ActorId> {
    s.actor_order()
        .filter(|id| {
            s.actor(*id)
                .and_then(Actor::life)
                .is_some_and(|l| l.actor_type.0 == ty.ordinal())
        })
        .collect()
}
fn body(s: &Simulation) -> ActorId {
    s.original_player.as_ref().unwrap().mo.unwrap()
}
fn differing(a: &[u8], b: &[u8]) -> Vec<usize> {
    assert_eq!(a.len(), b.len());
    (0..a.len()).filter(|i| a[*i] != b[*i]).collect()
}
fn weapons(s: &Simulation) -> player_weapons::PlayerWeaponState {
    let p = s.original_player.unwrap();
    p.weapon.and_then(PlayerWeaponRuntime::pack).unwrap()
}
const ESCAPE: Result<(), SimError> = Err(SimError::MapScale(MapScaleError::Escape));

// ---- Identity, admission and DSIM44 -----------------------------------------

/// Wire 28's identity: policy 28, its own context of wire 27's length, the
/// level constructors only, and DSIM44 captures. U12: it admits skills 0 and 1
/// (and still 2 and 3); wires 26 and 27 refuse 0 and 1 with the master error,
/// `Registry(PlayerBodyError::Unsupported)` from the admission gate; nightmare
/// stays refused.
#[test]
fn wire_28_identity_and_skills_0_and_1() {
    assert_eq!(V6.wire_id(), 28);
    assert_eq!(V6.context_schema(), b"PLAYER-MONSTERS-EPISODE1-6-v1");
    assert_eq!(
        V6.context_schema().len(),
        PLAYER_EPISODE1_V5_CONTEXT_SCHEMA.len()
    );
    assert!(V6.has_easy_skills() && V6.has_monster_escape_witness());
    assert!(!V5.has_easy_skills() && !V5.has_monster_escape_witness());
    assert_eq!(
        Simulation::with_original_registry(
            base(&[]).map(),
            player_map_scale::actor_slots(V6),
            OriginalExecutionRegistry::new(V6).unwrap(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .err(),
        Some(PlayerBodyError::Unsupported.into())
    );
    let l = base(&[]);
    let s = load(V6, &l, Skill::Medium).unwrap();
    assert_eq!(s.snapshot().version(), 44);
    assert!(s.world.wrapping_intercept && s.world.escape_witness.is_some());
    let refused = Err(LevelStartError::Registry(
        SimError::from(PlayerBodyError::Unsupported).into(),
    ));
    for skill in [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard] {
        let s = load(V6, &l, skill).unwrap();
        assert_eq!(s.actor_options.skill, skill);
        assert_eq!(s.original_player.unwrap().baby, skill == Skill::Baby);
        for old in [V5, OriginalExecutionProfile::PlayerMonstersEpisode1V4] {
            let got = load(old, &l, skill).map(|_| ());
            if matches!(skill, Skill::Baby | Skill::Easy) {
                assert_eq!(got, refused, "{old:?} {skill:?}");
            } else {
                assert_eq!(got, Ok(()));
            }
        }
    }
    for p in [V6, V5] {
        assert_eq!(load(p, &l, Skill::Nightmare).map(|_| ()), refused);
    }
}

/// p_mobj.c:743-751: skills 0 and 1 spawn a THING with option bit 1 and not one
/// with bit 2 or 4 only; skill 2 the reverse.
#[test]
fn skills_0_and_1_spawn_with_thing_bit_1() {
    let l = base(&[
        [0, 64, 0, 3001, 1],
        [0, -64, 0, 3001, 2],
        [64, 0, 0, 3001, 4],
    ]);
    let imps = |skill| of_type(&load(V6, &l, skill).unwrap(), T::MT_TROOP).len();
    assert_eq!([imps(Skill::Baby), imps(Skill::Easy)], [1, 1]);
    assert_eq!([imps(Skill::Medium), imps(Skill::Hard)], [1, 1]);
    let s = load(V6, &l, Skill::Easy).unwrap();
    let at = s.actor(of_type(&s, T::MT_TROOP)[0]).unwrap().position();
    assert_eq!(at, [0, 64].map(Fixed::from_map_units));
}

/// The wire-28 demo reader and writer admit skills 0 to 3 and refuse 4; the
/// wire-24 ones keep refusing 0 and 1.
#[test]
fn the_wire_28_demo_header_admits_skills_0_to_3() {
    use crate::demo::*;
    let c = [TicCommand::default()];
    for skill in 0..=3 {
        let bytes = write_demo_episode1_v3(4, skill, &c).unwrap();
        assert_eq!(
            parse_demo_episode1_v3(&bytes, 4).unwrap().0,
            DemoHeaderV2 { map: 4, skill }
        );
        if skill >= 2 {
            assert_eq!(bytes, write_demo_episode1_v2(4, skill, &c).unwrap());
        }
    }
    let header = |value| {
        Err(DemoError::Header {
            index: DEMO_SKILL_INDEX,
            value,
        })
    };
    let mut bytes = write_demo_episode1_v3(4, 0, &c).unwrap();
    for skill in [0, 1] {
        bytes[DEMO_SKILL_INDEX] = skill;
        assert_eq!(parse_demo_episode1_v2(&bytes, 4).map(|_| ()), header(skill));
        assert_eq!(
            write_demo_episode1_v2(4, skill, &c).map(|_| ()),
            header(skill)
        );
    }
    bytes[DEMO_SKILL_INDEX] = 4;
    assert_eq!(parse_demo_episode1_v3(&bytes, 4).map(|_| ()), header(4));
    assert_eq!(write_demo_episode1_v3(4, 4, &c).map(|_| ()), header(4));
}

/// Decision D1: DSIM44 is DSIM43 field for field. One state run under wire 27
/// and wire 28 at skill 2 (moving, turning, using, firing) differs after every
/// tic in exactly the version (offset 4: 43, 44), the `profile` value (offset
/// 82: 27, 28) and the context's part digit (offset 181: '5', '6'); events are
/// equal. Each reader refuses the other's version, and a capture whose version
/// names one wire and whose profile or context names the other is refused.
#[test]
fn dsim44_is_dsim43_but_for_the_three_values_naming_the_wire() {
    let l = base(&[]);
    let (mut v5, mut v6) = (
        load(V5, &l, Skill::Medium).unwrap(),
        load(V6, &l, Skill::Medium).unwrap(),
    );
    let command = |t: u32| {
        let mut c = idle();
        c[0] = TicCommand {
            forward: 50,
            side: if t % 3 == 0 { 24 } else { 0 },
            angle_turn: if t < 40 { 256 } else { -512 },
            buttons: [weapons::BT_ATTACK, weapons::BT_USE, 0][(t % 3) as usize],
        };
        c
    };
    for t in 0..=70 {
        if t > 0 {
            v5.advance(command(t)).unwrap();
            v6.advance(command(t)).unwrap();
            assert_eq!(v5.events(), v6.events(), "tic {t}");
        }
        let (b5, b6) = (
            v5.snapshot().canonical_bytes(),
            v6.snapshot().canonical_bytes(),
        );
        assert_eq!(differing(&b5, &b6), [4, 82, 181], "tic {t}");
        assert_eq!(
            (&b5[4..8], &b6[4..8]),
            (&[43, 0, 0, 0][..], &[44, 0, 0, 0][..])
        );
        assert_eq!(&b6[63..82], b"\x07\0\0\0profile\x04\0\0\0\0\0\0\0");
        assert_eq!(
            (&b5[82..86], &b6[82..86]),
            (&[27, 0, 0, 0][..], &[28, 0, 0, 0][..])
        );
        assert_eq!(&b6[130..156], b"\x0e\0\0\0context.schema\x1d\0\0\0\0\0\0\0");
        assert_eq!(&b6[156..185], PLAYER_EPISODE1_V6_CONTEXT_SCHEMA);
    }
    let (b5, b6) = (
        v5.snapshot().canonical_bytes(),
        v6.snapshot().canonical_bytes(),
    );
    let limits = DiagnosticLimits::default();
    let v6 = |b: &[u8]| Diagnostic::decode_episode1_v6(b, limits).map(|d| d.version());
    let v5 = |b: &[u8]| Diagnostic::decode_episode1_v5(b, limits).map(|d| d.version());
    let v4 = |b: &[u8]| Diagnostic::decode_episode1_v4(b, limits).map(|d| d.version());
    let version = |v| Err(DiagnosticError::Version(v));
    assert_eq!(
        [v6(&b6), v5(&b6), v4(&b6), v6(&b5)],
        [Ok(44), version(44), version(44), version(43)]
    );
    for (mut bytes, at, value, v6_reader) in [
        (b6.clone(), 82, 27, true),
        (b5.clone(), 82, 28, false),
        (b6.clone(), 181, b'5', true),
        (b5.clone(), 181, b'6', false),
        (b5.clone(), 4, 44, true),
        (b6.clone(), 4, 43, false),
    ] {
        bytes[at] = value;
        let decoded = if v6_reader { v6(&bytes) } else { v5(&bytes) };
        assert_eq!(decoded, Err(DiagnosticError::Schema), "{at}");
    }
}

/// DSIM44 round trip at every admitted skill: the capture decodes with the
/// skill in `actors.options`, and the same bytes relabelled DSIM43 (version,
/// profile and context) are refused for skills 0 and 1 by the wire-27 reader's
/// options check (`DiagnosticError::Identity`) and admitted for 2 and 3.
#[test]
fn dsim44_round_trips_every_admitted_skill() {
    let l = base(&[[0, 64, 0, 3001, 7]]);
    let limits = DiagnosticLimits::default();
    for skill in [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard] {
        let mut s = load(V6, &l, skill).unwrap();
        for _ in 0..10 {
            s.advance(idle()).unwrap();
        }
        let bytes = s.snapshot().canonical_bytes();
        assert_eq!(
            Diagnostic::decode_episode1_v6(&bytes, limits)
                .unwrap()
                .version(),
            44
        );
        let mut as43 = bytes.clone();
        as43[4] = 43;
        as43[82] = 27;
        as43[181] = b'5';
        let got = Diagnostic::decode_episode1_v5(&as43, limits).map(|d| d.version());
        if matches!(skill, Skill::Baby | Skill::Easy) {
            assert_eq!(got, Err(DiagnosticError::Identity), "{skill:?}");
        } else {
            assert_eq!(got, Ok(43), "{skill:?}");
        }
    }
}

// ---- sk_baby: P_GiveAmmo ------------------------------------------------------

/// The level's starting weapon state (50 bullets, the pistol ready).
fn pack() -> player_weapons::PlayerWeaponState {
    weapons(&load(V6, &base(&[]), Skill::Baby).unwrap())
}
/// `P_GiveAmmo` (p_inter.c:73-160) with `num` clip loads (0: half a clip) and
/// `double` as `gameskill == sk_baby`; returns (accepted, after, pending).
fn give(
    mut p: player_weapons::PlayerWeaponState,
    ammo: usize,
    before: u16,
    loads: u16,
    double: bool,
) -> (bool, u16, Option<Weapon>) {
    match ammo {
        0 => p.bullets = before,
        1 => p.shells = before,
        2 => p.cells = before,
        _ => p.rockets = before,
    }
    let accepted = world_pickups::ammo(&mut p, ammo, loads, double, &mut Vec::new()).unwrap();
    let after = [p.bullets, p.shells, p.cells, p.rockets][ammo];
    (accepted, after, p.pending)
}

/// p_inter.c:89-101: `num` is the clip loads times `clipammo` (10, 4, 20, 1), or
/// `clipammo/2` for 0 (a dropped clip, 536), and doubled at `sk_baby` after that,
/// for every caller: a clip (541), a dropped clip, a box (548, 560, 572, 584), a
/// single (554, 566, 578), the backpack's one load (597) and a weapon's two loads,
/// one if dropped (203-205).
#[test]
fn baby_doubles_every_ammo_amount_after_the_loads() {
    let p = pack();
    for (ammo, loads, plain) in [
        (0, 1, 10), // clip
        (0, 0, 5),  // dropped clip: clipammo/2 before the doubling
        (0, 5, 50), // box of bullets
        (1, 1, 4),  // shells
        (1, 5, 20), // box of shells
        (1, 2, 8),  // a found shotgun
        (2, 1, 20), // the backpack's cells
        (3, 1, 1),  // rocket
        (3, 5, 5),  // box of rockets
    ] {
        let got = [false, true].map(|double| give(p, ammo, 1, loads, double));
        assert_eq!(got, [(true, 1 + plain, None), (true, 1 + 2 * plain, None)]);
    }
}

/// p_inter.c:87-88 and 106-107: refused at `maxammo` before any doubling, and the
/// doubled amount is capped at `maxammo` (200 bullets, 50 shells; doubled by the
/// backpack, 589-595). p_inter.c:112-153: the switch from zero reads `oldammo`,
/// so the doubling changes nothing in it: from 0 bullets with the fist ready the
/// pending weapon is the pistol, from 1 nothing.
#[test]
fn baby_ammo_is_capped_after_the_doubling_and_keeps_the_switch() {
    let mut p = pack();
    assert_eq!(give(p, 0, 200, 1, true), (false, 200, None));
    assert_eq!(give(p, 0, 185, 1, true), (true, 200, None));
    assert_eq!(give(p, 0, 180, 1, true), (true, 200, None));
    assert_eq!(give(p, 0, 179, 1, true), (true, 199, None));
    assert_eq!(give(p, 1, 45, 1, true), (true, 50, None));
    assert_eq!(give(p, 1, 41, 1, true), (true, 49, None));
    p.backpack = true;
    assert_eq!(give(p, 0, 390, 5, true), (true, 400, None));
    assert_eq!(give(p, 1, 0, 5, true).1, 40);
    p.ready = Weapon::Fist;
    p.pending = None;
    assert_eq!(give(p, 0, 0, 0, true), (true, 10, Some(Weapon::Pistol)));
    assert_eq!(give(p, 0, 1, 0, true), (true, 11, None));
}

/// The pickup path end to end: the player walks onto a clip (2007) and a box of
/// shells (2049); wire 28 at skill 0 gains 20 bullets and 40 shells, wire 28 at
/// skill 1 and 2 and wire 27 at skill 2 gain 10 and 20 (the starting 50 bullets
/// are `G_PlayerReborn`'s, not `P_GiveAmmo`'s, g_game.c, so never doubled).
#[test]
fn the_pickups_double_at_skill_0_only() {
    let l = base(&[[-200, 0, 0, 2007, 7], [-150, 0, 0, 2049, 7]]);
    let walk = |policy, skill| {
        let mut s = load(policy, &l, skill).unwrap();
        assert_eq!((weapons(&s).bullets, weapons(&s).shells), (50, 0));
        let mut c = idle();
        c[0].forward = 25;
        for _ in 0..30 {
            s.advance(c).unwrap();
        }
        (weapons(&s).bullets, weapons(&s).shells)
    };
    assert_eq!(walk(V6, Skill::Baby), (70, 40));
    assert_eq!(walk(V6, Skill::Easy), (60, 20));
    assert_eq!(walk(V6, Skill::Medium), (60, 20));
    assert_eq!(walk(V5, Skill::Medium), (60, 20));
}

// ---- sk_baby: P_DamageMobj ----------------------------------------------------

/// `P_DamageMobj (target, inflictor, inflictor, amount)` through the shared plan
/// (the player by default), with `special` the target's sector special.
fn hurt(s: &mut Simulation, amount: i32, inflictor: Option<ActorId>, special: i16) {
    hurt_at(s, body(s), amount, inflictor, special);
}
fn hurt_at(s: &mut Simulation, target: ActorId, amount: i32, by: Option<ActorId>, special: i16) {
    let inflictor = by;
    s.checked_demon_operation(|h| -> Result<(), SimError> {
        let request = DamageRequest {
            target,
            inflictor,
            source: inflictor,
            amount,
        };
        let plan = DamagePlan::current_original_in(h, request, Some(special))?;
        h.access.reserve(plan.spawn_count())?;
        h.apply_damage(plan);
        Ok(())
    })
    .and_then(|r| r)
    .unwrap();
}
/// The halving itself (p_inter.c:799-800): `damage >>= 1` on the player at
/// `sk_baby` only, flooring odd values, and never on another target.
#[test]
fn baby_halves_the_players_damage_with_a_shift() {
    let s = load(V6, &base(&[]), Skill::Baby).unwrap();
    let p = s.original_player.unwrap();
    let request = |target, amount| DamageRequest {
        target,
        inflictor: None,
        source: None,
        amount,
    };
    let amounts = [0, 1, 2, 7, 40, 999, 1000];
    let halved = amounts.map(|a| p.player_damage(request(body(&s), a)).amount);
    assert_eq!(halved, [0, 0, 1, 3, 20, 499, 500]);
    let other = s.actor_order().find(|id| *id != body(&s)).unwrap();
    assert_eq!(p.player_damage(request(other, 7)).amount, 7);
    for skill in [Skill::Easy, Skill::Medium, Skill::Hard] {
        let s = load(V6, &base(&[]), skill).unwrap();
        let p = s.original_player.unwrap();
        assert_eq!(p.player_damage(request(body(&s), 7)).amount, 7);
    }
}

/// p_inter.c:799-832: the halving precedes the thrust, so at skill 0 a blow of
/// `2k + 1` (and `2k`) leaves exactly the state a blow of `k` leaves at skill 2:
/// health, `damagecount`, the knockback momentum (from the halved damage), the
/// RNG and every other byte but the skill. An Imp 64 units east is the inflictor.
#[test]
fn a_baby_blow_equals_the_halved_blow_at_skill_2() {
    let l = base(&[[-192, 0, 0, 3001, 7]]);
    let bytes = |s: &Simulation| s.snapshot().canonical_bytes();
    for k in [0, 1, 3, 10, 19, 20, 49] {
        for amount in [2 * k, 2 * k + 1] {
            let mut baby = load(V6, &l, Skill::Baby).unwrap();
            let mut medium = load(V6, &l, Skill::Medium).unwrap();
            let imp = of_type(&baby, T::MT_TROOP)[0];
            hurt(&mut baby, amount, Some(imp), 0);
            hurt(&mut medium, k, Some(imp), 0);
            let at = |s: &Simulation| {
                let (p, a) = (s.original_player.unwrap(), s.actor(body(s)).unwrap());
                (p.health, p.damage_count, p.attacker, a.momentum(), s.rng())
            };
            assert_eq!(at(&baby), at(&medium), "{amount}");
            assert_eq!(at(&baby).0, 100 - k, "{amount}");
            assert_eq!(baby.events(), medium.events(), "{amount}");
            // Only `actors.options[0]` (the skill byte) differs.
            assert_eq!(differing(&bytes(&baby), &bytes(&medium)).len(), 1);
        }
    }
}

/// p_inter.c:799-916 for a blow of 1 at skill 0: the damage becomes 0, and the C
/// still runs everything after the halving: a zero thrust, no fall-forward draw
/// (`damage > target->health` fails), `attacker` set, `damagecount` += 0, the
/// health unchanged, then the pain roll (`P_Random() < painchance`, 255 for the
/// player: one draw, the pain state entered).
#[test]
fn a_baby_blow_of_1_is_a_zero_damage_hit() {
    let l = base(&[[-192, 0, 0, 3001, 7]]);
    let mut s = load(V6, &l, Skill::Baby).unwrap();
    let imp = of_type(&s, T::MT_TROOP)[0];
    let (rng, momentum) = (s.rng(), s.actor(body(&s)).unwrap().momentum());
    hurt(&mut s, 1, Some(imp), 0);
    let p = s.original_player.unwrap();
    assert_eq!((p.health, p.damage_count, p.attacker), (100, 0, Some(imp)));
    let a = s.actor(body(&s)).unwrap();
    assert_eq!(a.momentum(), momentum);
    let pain = Some(StateId(OriginalState::S_PLAY_PAIN.ordinal()));
    assert_eq!(
        (a.life().unwrap().health, a.life().unwrap().state),
        (100, pain)
    );
    let mut next = rng;
    next.p_random();
    assert_eq!(s.rng(), next);
}

/// p_inter.c:837-842 after 799-800: at skill 0 with health 100 in a special-11
/// sector a blow of 201 is halved to 100, which reaches the health and is capped
/// to 99 (health 1). Capping first would give 99 halved, 49 (health 51).
#[test]
fn baby_halves_before_the_sector_11_cap() {
    let l = base(&[]);
    for (amount, health) in [(201, 1), (200, 1), (199, 1), (198, 1), (150, 25)] {
        let mut s = load(V6, &l, Skill::Baby).unwrap();
        hurt(&mut s, amount, None, 11);
        assert_eq!(s.original_player.unwrap().health, health, "{amount}");
    }
    let mut s = load(V6, &l, Skill::Medium).unwrap();
    hurt(&mut s, 201, None, 11);
    assert_eq!(s.original_player.unwrap().health, 1);
}

/// p_inter.c:855-869 after 799-800: armour saves a third (green) or a half (blue)
/// of the halved damage. Green 100 against 61 at skill 0: 30, saved 10, health
/// 80, armour 90 (armour before the halving would save 20: armour 80).
#[test]
fn baby_armour_reads_the_halved_damage() {
    let l = base(&[]);
    for (kind, amount, health, points) in [
        (ArmorType::Green, 61, 80, 90),
        (ArmorType::Blue, 61, 85, 85),
        (ArmorType::Green, 1, 100, 100),
    ] {
        let mut s = load(V6, &l, Skill::Baby).unwrap();
        s.original_player.as_mut().unwrap().armor = Some(PlayerArmorState { kind, points: 100 });
        hurt(&mut s, amount, None, 0);
        let p = s.original_player.unwrap();
        assert_eq!(
            (p.health, p.armor.unwrap().points),
            (health, points),
            "{amount}"
        );
    }
}

// ---- S″ -------------------------------------------------------------------------

/// S′ (decision F3) needs an Imp (radius 20) whose open box meets the hall's west
/// wall `x = 0` to stand at least 31 − 20 = 11 units in front of it; S″ drops
/// that for monsters under wire 28. The boundary value 11 loads under both
/// wires; 10 and 1 load under wire 28 only, wire 27 refusing them with the master
/// error `Setup(Monster(Arithmetic))`.
#[test]
fn s_double_prime_admits_an_embedded_monster_under_wire_28_alone() {
    let refused = Err(LevelStartError::Setup(MonsterError::Arithmetic.into()));
    for (x, v5_loads) in [(11, true), (10, false), (5, false), (1, false)] {
        let l = hall([-300, 0], &[[x, 100, 0, 3001, 7]]);
        let s = load(V6, &l, Skill::Hard).unwrap();
        let imp = s.actor(of_type(&s, T::MT_TROOP)[0]).unwrap();
        assert_eq!(imp.position()[0], Fixed::from_map_units(x));
        let got = load(V5, &l, Skill::Hard).map(|_| ());
        assert_eq!(got, if v5_loads { Ok(()) } else { refused }, "{x}");
    }
}

/// Barrels and the player keep S′'s margin under wire 28: a barrel (radius 10,
/// margin 21) whose box only touches the wall (centre 10) loads and one embedded
/// by a unit (9) is refused; the player (radius 16, margin 15) loads at 15 and
/// is refused at 14 (`PlayerBodyError::Binding`), under both wires.
#[test]
fn barrels_and_the_player_keep_the_margin_under_wire_28() {
    let barrel = Err(LevelStartError::Setup(MonsterError::Arithmetic.into()));
    let player = Err(LevelStartError::Setup(PlayerBodyError::Binding.into()));
    for policy in [V5, V6] {
        let at = |l: Lumps| load(policy, &l, Skill::Medium).map(|_| ());
        assert_eq!(at(hall([-300, 0], &[[10, 100, 0, 2035, 7]])), Ok(()));
        assert_eq!(at(hall([-300, 0], &[[9, 100, 0, 2035, 7]])), barrel);
        assert_eq!(at(hall([15, 100], &[])), Ok(()));
        assert_eq!(at(hall([14, 100], &[])), player);
    }
}

// ---- The monster escape witness -------------------------------------------------

/// An Imp embedded 5 units into the hall's west wall (centre (5, 100), admitted
/// by S″ only), with the player in the corridor.
fn embedded() -> (Simulation, ActorId) {
    let s = load(V6, &hall([-300, 0], &[[5, 100, 0, 3001, 7]]), Skill::Hard).unwrap();
    let imp = of_type(&s, T::MT_TROOP)[0];
    (s, imp)
}
fn push(s: &mut Simulation, id: ActorId, momentum: [i16; 2]) {
    s.actors.get_mut(id).unwrap().spec.momentum = momentum.map(Fixed::from_map_units);
}

/// The witness is the barrel's parity test (`BarrelMotion::after_move`), exact on
/// both sides of the wall line `x = 0`: one raw unit (or one map unit) inside
/// passes, one raw unit (or one map unit) outside refuses with `Escape`. (A
/// centre on the line itself cannot follow an accepted move: the box would cross
/// the wall.)
#[test]
fn the_witness_is_exact_on_both_sides_of_the_wall() {
    let (s, imp) = embedded();
    let boundary = s.world.escape_witness.clone().unwrap();
    assert_eq!(&*boundary, s.boundary.as_deref().unwrap());
    let mut witness = barrels::BarrelMotion {
        get: |_| None,
        boundary: &boundary,
    };
    let mut a = *s.actor(imp).unwrap();
    for (x, ok) in [(1, true), (65536, true), (-1, false), (-65536, false)] {
        a.spec.position = [Fixed::from_bits(x), Fixed::from_map_units(100)];
        assert_eq!(
            witness.after_move(&a),
            if ok { Ok(()) } else { ESCAPE },
            "{x}"
        );
    }
}

/// p_mobj.c:150-164 halve only components above `MAXMOVE/2`, so a westward
/// momentum of 26 is one `P_TryMove` to (-21, 100), whose box `[-41, -1]` clears
/// the wall `x = 0`: the C accepts it and the Imp stands in the void. Wire 28
/// refuses the tic with `Escape`, every canonical byte and the RNG kept, through
/// `advance` and `step`. A momentum of 15 leaves the box across the wall: the
/// move is blocked, the Imp stops in place and the tic is admitted.
#[test]
fn a_thrust_out_of_the_wall_refuses_the_tic_with_the_state_kept() {
    for step in [false, true] {
        let (mut s, imp) = embedded();
        push(&mut s, imp, [-26, 0]);
        let (before, rng) = (s.snapshot(), s.rng());
        let got = if step {
            s.step(idle()).map(|_| ())
        } else {
            s.advance(idle())
        };
        assert_eq!(got, ESCAPE);
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.rng(), rng);
    }
    let (mut s, imp) = embedded();
    push(&mut s, imp, [-15, 0]);
    s.advance(idle()).unwrap();
    let a = s.actor(imp).unwrap();
    assert_eq!(a.position(), [5, 100].map(Fixed::from_map_units));
    assert_eq!(a.momentum(), [Fixed::ZERO; 2]);
}

/// Planted fault: with the witness removed from the same wire-28 world, the tic
/// it refused is admitted and the Imp's centre ends outside the boundary at
/// (-21, 100), so the witness is what refuses. A corpse is a mover too: the
/// same Imp killed in place and thrust likewise is refused.
#[test]
fn without_the_witness_the_imp_leaves_the_room() {
    let (mut s, imp) = embedded();
    s.world.escape_witness = None;
    push(&mut s, imp, [-26, 0]);
    s.advance(idle()).unwrap();
    let at = s.actor(imp).unwrap().position();
    assert_eq!(at, [-21, 100].map(Fixed::from_map_units));
    assert!(!s.boundary.as_ref().unwrap().inside(at));

    let (mut s, imp) = embedded();
    hurt_at(&mut s, imp, 1000, None, 0);
    assert!(s.actor(imp).unwrap().life().unwrap().health <= 0);
    push(&mut s, imp, [-26, 0]);
    let before = s.snapshot();
    assert_eq!(s.advance(idle()), ESCAPE);
    assert_eq!(s.snapshot(), before);
}

/// An Imp put by hand into the void at (-100, 200) (outside the boundary,
/// inside the vertex rectangle): with a momentum of one unit, or, with none,
/// chasing the player with `movecount` 5, so that its first `A_Chase` walks east
/// through `P_Move` (p_enemy.c:303-334, `P_TryMove` and its crossings; no missile
/// check while `movecount` is set, 738-742). Wire 27 admits the move, as for
/// every non-barrel mover since wire 21; wire 28 refuses the tic with `Escape`
/// and the state kept. Below the public API: no admitted state holds a monster
/// outside the boundary.
#[test]
fn every_non_player_move_carries_the_witness_under_wire_28_alone() {
    for (policy, walk) in [(V5, false), (V6, false), (V5, true), (V6, true)] {
        let mut s = load(
            policy,
            &hall([-300, 0], &[[200, 100, 0, 3001, 7]]),
            Skill::Hard,
        )
        .unwrap();
        let (imp, player) = (of_type(&s, T::MT_TROOP)[0], body(&s));
        let at = [Fixed::from_map_units(-100), Fixed::from_map_units(200)];
        assert!(!s.boundary.as_ref().unwrap().inside(at));
        let location = s.world.locate_validated(at);
        let a = s.actors.get_mut(imp).unwrap();
        (a.spec.position, a.location) = (at, Some(location));
        let life = a.life.as_mut().unwrap();
        if walk {
            life.state = Some(StateId(OriginalState::S_TROO_RUN1.ordinal()));
            (life.tics, life.target, life.movedir, life.movecount) = (1, Some(player), 0, 5);
        } else {
            a.spec.momentum = [Fixed::from_map_units(1), Fixed::ZERO];
        }
        s.spatial.link(imp);
        let before = s.snapshot();
        let got = s.advance(idle());
        if policy == V6 {
            assert_eq!((got, s.snapshot() == before), (ESCAPE, true));
        } else {
            let x = s.actor(imp).unwrap().position()[0];
            assert_eq!(
                (got, x),
                (Ok(()), Fixed::from_map_units(if walk { -92 } else { -99 }))
            );
        }
    }
}

// ---- The Demon's bite on the player ---------------------------------------------

/// A Demon 46 units east of the player, its target the player, one tic before
/// `S_SARG_ATK3` (`A_SargAttack`, p_enemy.c:935-948: `(P_Random()%10+1)*4`),
/// every sector's special set to `special`, the player's health `health`.
fn bite(policy: OriginalExecutionProfile, skill: Skill, special: i16, health: i32) -> Simulation {
    let mut l = base(&[[-210, 0, 180, 3002, 7]]);
    let sectors = &mut l.lumps.iter_mut().find(|(n, _)| n == "SECTORS").unwrap().1;
    for s in sectors.chunks_exact_mut(26) {
        s[22..24].copy_from_slice(&special.to_le_bytes());
    }
    let mut s = load(policy, &l, skill).unwrap();
    s.advance(idle()).unwrap();
    let (demon, player) = (of_type(&s, T::MT_SERGEANT)[0], body(&s));
    s.original_player.as_mut().unwrap().health = health;
    let life = s.actors.get_mut(player).unwrap().life.as_mut().unwrap();
    life.health = health;
    let life = s.actors.get_mut(demon).unwrap().life.as_mut().unwrap();
    life.state = Some(StateId(OriginalState::S_SARG_ATK2.ordinal()));
    life.tics = 1;
    life.target = Some(player);
    s.advance(idle()).unwrap();
    s
}

/// p_inter.c:799-800 on the bite path: the Demon's bite at skill 0 takes half
/// of what the same bite (same draw) takes at skill 2, and wire 27 at skill 2
/// takes the same as wire 28 at skill 2.
#[test]
fn baby_halves_the_demons_bite() {
    let health = |p, skill| bite(p, skill, 0, 100).original_player.unwrap().health;
    let medium = 100 - health(V6, Skill::Medium);
    assert!((4..=40).contains(&medium) && medium % 4 == 0, "{medium}");
    assert_eq!(100 - health(V5, Skill::Medium), medium);
    assert_eq!(100 - health(V6, Skill::Baby), medium / 2);
}

/// p_inter.c:837-842 for the bite: under wire 28 a bite that reaches the
/// player's health in a special-11 sector is capped at `health − 1`. Wire 27
/// applies no cap to the bite (the frozen behaviour), which gives the drawn
/// damage `d`; wire 28 must leave `max(11 − d, 1)`.
#[test]
fn the_bite_takes_the_sector_11_cap_under_wire_28() {
    let v5 = bite(V5, Skill::Medium, 11, 11);
    // The body's health is not clamped at 0 (p_inter.c:883).
    let d = 11 - v5.actor(body(&v5)).unwrap().life().unwrap().health;
    let v6 = bite(V6, Skill::Medium, 11, 11).original_player.unwrap();
    assert_eq!(v6.health, (11 - d).max(1));
    assert!(
        d >= 11,
        "the draw {d} must reach the health for the cap to show"
    );
}
