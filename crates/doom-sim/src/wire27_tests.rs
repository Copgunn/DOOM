//! Wire 27 (`PlayerMonstersEpisode1V5`, DSIM43), step A: unit tests of the
//! wrapping `P_InterceptVector` (p_maputl.c:231-251) and of DSIM43. Expected
//! values come from the genuine C rows (`tests/reference/m3d3a_fractions.csv`,
//! the source body of `P_InterceptVector2`, p_sight.c:109-128, whose expression
//! `P_InterceptVector` shares), from hand computation, or from
//! [`reference`], an independent transcription of the C text in 128-bit
//! integers; never from the function under test. Rust-only otherwise. Step B:
//! sight's `P_InterceptVector2` (p_sight.c:109-128) refused on an `INT_MIN`
//! `FixedDiv` operand under wire 27 alone (owner's ruling of 2026-11-24), its
//! vectors computed by hand from p_sight.c:117-125.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::intercept_vector::{
    FixedDivRefusal, InterceptCaller, Operand, Wire27Refusal, WrapClass, policy_intercept,
    sight_intercept, wire27_admits, wire27_intercept, wire27_wraps, wrapping_intercept,
};
use crate::*;
use FixedDivRefusal::{AbsMin, Div2Range, Saturation};

type V = [i64; 2];
fn fx(v: V) -> [Fixed; 2] {
    v.map(|b| Fixed::from_bits(i32::try_from(b).unwrap()))
}
fn wrap(v: i128) -> i64 {
    let m = v.rem_euclid(1 << 32);
    (if m >= 1 << 31 { m - (1 << 32) } else { m }) as i64
}
fn fits(v: i128) -> bool {
    (i128::from(i32::MIN)..=i128::from(i32::MAX)).contains(&v)
}
/// `FixedMul` (m_fixed.c:43-49) before narrowing: the product's floor `>> 16`.
fn mul(a: i64, b: i64) -> i128 {
    (i128::from(a) * i128::from(b)).div_euclid(65536)
}
/// p_maputl.c:240-250 with `FixedDiv` (m_fixed.c:57-87) and wire 27's refusals,
/// computed exactly and reduced mod 2^32 where the C stores an `int`. Returns
/// the result and whether any step left `int` (the cases `checked_intercept`
/// refuses).
fn reference(v2s: V, v2d: V, v1s: V, v1d: V) -> (Result<i32, FixedDivRefusal>, bool) {
    let (p1, p2) = (mul(v1d[1] >> 8, v2d[0]), mul(v1d[0] >> 8, v2d[1]));
    let d = i128::from(wrap(p1)) - i128::from(wrap(p2));
    let mut wrapped = !fits(p1) || !fits(p2) || !fits(d);
    let den = wrap(d);
    if den == 0 {
        return (Ok(0), wrapped);
    }
    let x = i128::from(v1s[0]) - i128::from(v2s[0]);
    let y = i128::from(v2s[1]) - i128::from(v1s[1]);
    let (q1, q2) = (mul(wrap(x) >> 8, v1d[1]), mul(wrap(y) >> 8, v1d[0]));
    let n = i128::from(wrap(q1)) + i128::from(wrap(q2));
    wrapped |= !fits(x) || !fits(y) || !fits(q1) || !fits(q2) || !fits(n);
    let num = wrap(n);
    if num == i64::from(i32::MIN) || den == i64::from(i32::MIN) {
        return (Err(AbsMin), wrapped);
    }
    if (num.abs() >> 14) >= den.abs() {
        if wrapped {
            return (Err(Saturation), true);
        }
        return (Ok(if (num ^ den) < 0 { i32::MIN } else { i32::MAX }), false);
    }
    let c = num as f64 / den as f64 * 65536.0;
    if !(-2147483648.0..2147483648.0).contains(&c) {
        return (Err(Div2Range), wrapped);
    }
    (Ok(c as i32), wrapped)
}
fn wrapping(v2s: V, v2d: V, v1s: V, v1d: V) -> Result<i32, FixedDivRefusal> {
    wrapping_intercept(fx(v2s), fx(v2d), fx(v1s), fx(v1d)).map(Fixed::bits)
}
fn checked(v2s: V, v2d: V, v1s: V, v1d: V) -> Option<i32> {
    sight::checked_intercept(fx(v2s), fx(v2d), fx(v1s), fx(v1d)).map(Fixed::bits)
}

/// Where `checked_intercept` is `Some`, wire 27 returns that value (an `INT_MIN`
/// operand aside): over the 131 genuine C rows, every one of which is `Some`
/// (none wraps) and equals the C value, and over a deterministic sweep, which
/// also agrees with [`reference`] everywhere, wrapped or not, and never reaches
/// `Div2Range`.
#[test]
fn equals_checked_where_some_over_the_c_rows_and_a_sweep() {
    let mut rows = 0;
    for line in include_str!("../../../tests/reference/m3d3a_fractions.csv")
        .lines()
        .skip(1)
    {
        let n: Vec<i64> = line.split(',').map(|s| s.parse().unwrap()).collect();
        let a = ([n[0], n[1]], [n[2], n[3]], [n[4], n[5]], [n[6], n[7]]);
        let got = wrapping(a.0, a.1, a.2, a.3);
        assert_eq!(got, reference(a.0, a.1, a.2, a.3).0, "row {rows}");
        assert_eq!(checked(a.0, a.1, a.2, a.3).map(Ok), Some(got), "row {rows}");
        assert_eq!(got, Ok(n[8] as i32), "C row {rows}");
        rows += 1;
    }
    assert_eq!(rows, 131);
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        // A value of 0 to 31 significant bits, either sign.
        (seed as i32 >> (seed >> 59)) as i64
    };
    let mut seen = [0_u32; 5];
    for _ in 0..200_000 {
        let a = (
            [next(), next()],
            [next(), next()],
            [next(), next()],
            [next(), next()],
        );
        let (expected, wrapped) = reference(a.0, a.1, a.2, a.3);
        let got = wrapping(a.0, a.1, a.2, a.3);
        assert_eq!(got, expected, "{a:?}");
        match checked(a.0, a.1, a.2, a.3) {
            Some(v) => {
                assert!(!wrapped);
                assert!(got == Ok(v) || got == Err(AbsMin), "{a:?}");
                seen[0] += 1;
            }
            None => {
                assert!(wrapped);
                seen[match got {
                    Ok(_) => 1,
                    Err(Saturation) => 2,
                    Err(AbsMin) => 3,
                    Err(Div2Range) => 4,
                }] += 1;
            }
        }
    }
    assert!(seen[..3].iter().all(|n| *n > 1000), "{seen:?}");
    assert_eq!(seen[4], 0);
}

/// One hand-computed vector per class (p_maputl.c:240-250, m_fixed.c:43-87),
/// with `big >> 8 = 0x7FFF00 = 8,388,352` written `b` below. Each is refused
/// by `checked_intercept`.
#[test]
fn hand_vectors_of_each_wrap_class() {
    let (o, big, a) = ([0, 0], 0x7fff_0000, 1 << 30);
    // Wraps that cancel: both den products are b·2^14 (minus b) past `int`;
    // den = a exactly, num = 0x7FFF0000, frac = 256.0, the exact value.
    let cancel = (o, [a, a - 65536], [1 << 24, 0], [big, big]);
    // A wrong intercept: one product, b·2^14 = 137,434,759,168, wraps to
    // -4,194,304; the exact frac 1024 (1/64) becomes -511.984375·65536.
    let wrong = (o, [a, 0], [1 << 24, 0], [big, big]);
    // den = FixedMul(2^22, 2^26) = 2^32 wraps to exactly 0: 0 (p_maputl.c:242-243),
    // where the exact den gives frac 1.
    let zero = (o, [1 << 26, 0], [1 << 20, 0], [0, a]);
    // num = 2·2,147,418,112 wraps to -131,072 over den 131,068:
    // -131072/131068·65536 = -65538.00006, truncated to -65538.
    let num = ([0, a], [65536, 0], [a, 0], [33_553_408, 33_553_408]);
    for (v, expected) in [
        (cancel, 256 << 16),
        (wrong, -33_553_408),
        (zero, 0),
        (num, -65538),
    ] {
        assert_eq!(checked(v.0, v.1, v.2, v.3), None);
        assert_eq!(wrapping(v.0, v.1, v.2, v.3), Ok(expected));
        assert_eq!(reference(v.0, v.1, v.2, v.3), (Ok(expected), true));
    }
}

/// The three `FixedDiv` refusals, each named (ruling 7).
#[test]
fn each_fixeddiv_refusal_is_named() {
    let (o, a) = ([0, 0], 1 << 30);
    // Saturation of wrapped operands: den = 2^32 + 64 wraps to 64, num = 2^20,
    // (2^20 >> 14) = 64 >= 64 (m_fixed.c:62-63).
    let v = (o, [(1 << 26) + 1, 0], [1 << 14, 0], [0, a]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), None);
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(Saturation));
    // INT_MIN after a wrap: num = 2^30 + 2^30.
    let v = ([0, a], [65536, 0], [a, 0], [1 << 24, 1 << 24]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), None);
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    // INT_MIN without a wrap, the one non-wrapped difference from wire 26:
    // num = -2^30 - 2^30 fits, and wire 26 keeps its unsigned-magnitude value.
    let v = ([a, -a], [65536, 0], o, [1 << 24, 1 << 24]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), Some(i32::MIN));
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    // den = FixedMul(-2^22, 2^25) = INT_MIN without a wrap; wire 26 gives 128.
    let v = (o, [1 << 25, 0], [65536, 0], [0, -a]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), Some(128));
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    // Div2Range is unreachable without INT_MIN: at the largest |num| the guard
    // fails, |den|·2^14 - 1, FixedDiv2 stays in range (m_fixed.c:84-85).
    for den in [1, 2, 3, 7, 100, 65535, 65536, 1 << 20, i32::MAX] {
        let top = (i64::from(den) << 14) - 1;
        let num = i32::try_from(top.min(i64::from(i32::MAX))).unwrap();
        for (n, d) in [(num, den), (-num, den), (num, -den), (-num, -den)] {
            assert!((n.unsigned_abs() >> 14) < d.unsigned_abs());
            assert!(
                Fixed::from_bits(n)
                    .div2_checked(Fixed::from_bits(d))
                    .is_ok()
            );
        }
    }
}

fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
/// The wire-26 unit-test room: a 1,024 × 256 rectangle, the player at (-256, 0)
/// facing east.
fn base() -> Lumps {
    let c = &cases()[0];
    let mut l = room(c.map);
    let t = &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1;
    t.splice(0..0, record(c.position[0], c.position[1], 0, 1, 7));
    l
}
fn level(l: &Lumps, v5: bool) -> Simulation {
    let build = if v5 {
        Simulation::original_player_episode1_v5_level
    } else {
        Simulation::original_player_episode1_v4_level
    };
    build(
        l.map(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
        LevelStart::NewGame,
    )
    .unwrap()
}
fn command(t: u32) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward: 50,
        side: if t % 3 == 0 { 24 } else { 0 },
        angle_turn: if t < 40 { 256 } else { -512 },
        buttons: [weapons::BT_ATTACK, weapons::BT_USE, 0][(t % 3) as usize],
    };
    c
}
/// The bytes of `bytes` that differ from `other`'s.
fn differing(bytes: &[u8], other: &[u8]) -> Vec<usize> {
    assert_eq!(bytes.len(), other.len());
    (0..bytes.len())
        .filter(|i| bytes[*i] != other[*i])
        .collect()
}

/// Ruling 2: DSIM43 is DSIM42 field for field. One state, captured under wire 26
/// and wire 27 after every tic of a run that moves, turns, uses and fires (no
/// intercept wraps in this room), differs in exactly three bytes: the version
/// (offset 4: 42, 43), the `profile` value (offset 82: 26, 27) and the context
/// string's part digit (offset 181: '4', '5'); the paths and lengths around them
/// are equal, and the events are equal.
#[test]
fn dsim43_is_dsim42_but_for_the_three_values_naming_the_wire() {
    let l = base();
    let (mut v4, mut v5) = (level(&l, false), level(&l, true));
    assert!(!v4.world.wrapping_intercept && v5.world.wrapping_intercept);
    for t in 0..=70 {
        if t > 0 {
            v4.advance(command(t)).unwrap();
            v5.advance(command(t)).unwrap();
            assert_eq!(v4.events(), v5.events(), "tic {t}");
        }
        let (b4, b5) = (
            v4.snapshot().canonical_bytes(),
            v5.snapshot().canonical_bytes(),
        );
        assert_eq!(differing(&b4, &b5), [4, 82, 181], "tic {t}");
        assert_eq!(
            (&b4[..8], &b5[4..8]),
            (&b"DSIM*\0\0\0"[..], &[43, 0, 0, 0][..])
        );
        assert_eq!(&b4[63..82], b"\x07\0\0\0profile\x04\0\0\0\0\0\0\0");
        assert_eq!(
            (&b4[82..86], &b5[82..86]),
            (&[26, 0, 0, 0][..], &[27, 0, 0, 0][..])
        );
        assert_eq!(&b4[130..156], b"\x0e\0\0\0context.schema\x1d\0\0\0\0\0\0\0");
        assert_eq!(&b4[156..185], PLAYER_EPISODE1_V4_CONTEXT_SCHEMA);
        assert_eq!(&b5[156..185], PLAYER_EPISODE1_V5_CONTEXT_SCHEMA);
    }
    let (b4, b5) = (
        v4.snapshot().canonical_bytes(),
        v5.snapshot().canonical_bytes(),
    );
    let limits = DiagnosticLimits::default();
    assert_eq!(
        Diagnostic::decode_episode1_v4(&b4, limits)
            .unwrap()
            .version(),
        42
    );
    assert_eq!(
        Diagnostic::decode_episode1_v5(&b5, limits)
            .unwrap()
            .version(),
        43
    );
    assert_eq!(
        Diagnostic::decode_episode1_v4(&b5, limits).err(),
        Some(DiagnosticError::Version(43))
    );
    assert_eq!(
        Diagnostic::decode_episode1_v5(&b4, limits).err(),
        Some(DiagnosticError::Version(42))
    );
    // A version that names one wire and a profile (or context) that names the
    // other is refused.
    for (mut bytes, at, value, v5_reader) in [
        (b5.clone(), 82, 26, true),
        (b4.clone(), 82, 27, false),
        (b5.clone(), 181, b'4', true),
        (b4.clone(), 181, b'5', false),
        (b4.clone(), 4, 43, true),
        (b5.clone(), 4, 42, false),
    ] {
        bytes[at] = value;
        let decoded = if v5_reader {
            Diagnostic::decode_episode1_v5(&bytes, limits)
        } else {
            Diagnostic::decode_episode1_v4(&bytes, limits)
        };
        assert_eq!(decoded.err(), Some(DiagnosticError::Schema), "{at}");
    }
}

/// Wire 27's identity and constructor rule: policy 27, its own context string
/// of wire 26's length, and only the level-start constructors build it.
#[test]
fn wire_27_identity_and_constructor() {
    let v5 = OriginalExecutionProfile::PlayerMonstersEpisode1V5;
    assert_eq!(v5.wire_id(), 27);
    assert_eq!(v5.context_schema(), b"PLAYER-MONSTERS-EPISODE1-5-v1");
    assert_eq!(
        v5.context_schema().len(),
        PLAYER_EPISODE1_V4_CONTEXT_SCHEMA.len()
    );
    assert_eq!(
        Simulation::with_original_registry(
            base().map(),
            player_map_scale::actor_slots(v5),
            OriginalExecutionRegistry::new(v5).unwrap(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .err(),
        Some(PlayerBodyError::Unsupported.into())
    );
    let s = level(&base(), true);
    assert_eq!(s.snapshot().version(), 43);
    assert_eq!(s.level_start_kind(), Some(StartKind::NewGame));
}

/// Ruling 6 and the owner's corrections of 2026-11-24: the four callers with a
/// genuine wrapping item wrap, and only under wire 27, each in the classes its
/// items take. "Wraps that cancel" above is admitted for all four through a
/// wire-27 world and refused through a wire-26 one; "a wrong intercept" (a wrong
/// den) only for the aim and shoot traces (AIMSHOT): the slide and use traces
/// have no such item and refuse it. The shoot trace against a thing has none
/// (tests/reference/wire27_results.json) and stays checked under wire 27: both
/// vectors are refused there under both wires.
#[test]
fn every_caller_wraps_under_wire_27_alone() {
    let l = base();
    let (v4, v5) = (level(&l, false), level(&l, true));
    let (big, a) = (0x7fff_0000, 1 << 30);
    let cancel = ([0, 0], [a, a - 65536], [1 << 24, 0], [big, big]);
    let wrong = ([0, 0], [a, 0], [1 << 24, 0], [big, big]);
    let at = |w: &StaticWorld, c, v: (V, V, V, V)| {
        policy_intercept(w, c, fx(v.0), fx(v.1), fx(v.2), fx(v.3)).map(Fixed::bits)
    };
    for caller in [
        InterceptCaller::Aim,
        InterceptCaller::ShootLine,
        InterceptCaller::Slide,
        InterceptCaller::Use,
    ] {
        assert!(wire27_wraps(caller));
        let den_wrong = matches!(caller, InterceptCaller::Aim | InterceptCaller::ShootLine);
        for (v, expected) in [
            (cancel, Some(256 << 16)),
            (wrong, den_wrong.then_some(-33_553_408)),
        ] {
            assert_eq!(at(&v4.world, caller, v), None);
            assert_eq!(at(&v5.world, caller, v), expected, "{caller:?}");
        }
    }
    let thing = InterceptCaller::ShootThing;
    assert!(!wire27_wraps(thing));
    for world in [&v4.world, &v5.world] {
        for v in [cancel, wrong] {
            assert_eq!(at(world, thing, v), None);
        }
    }
}

/// The room with its east wall (line 1) stretched to (512, 7872) → (512, -128),
/// 8,000 units long, and a BLOCKMAP of 9 × 63 cells listing every line, so that
/// the Episode 1 load rules (span and line components under 8,192) still admit
/// it. A pistol shot east (range 2,048) crosses the wall: den =
/// FixedMul(-8000·256, ~2^27) ≈ -4,194,304,000 leaves `int`.
fn stretched() -> Lumps {
    let mut l = base();
    let [_, b] = l.line_vertices(0);
    l.set_vertex(b, [512, 7872]);
    let (w, h) = (9_u16, 63_u16);
    let cells = usize::from(w * h);
    let mut words = vec![(-512_i16) as u16, (-128_i16) as u16, w, h];
    words.extend((0..cells).map(|c| (4 + cells + 6 * c) as u16));
    for _ in 0..cells {
        words.extend([0, 0, 1, 2, 3, 0xffff]);
    }
    l.lumps.iter_mut().find(|(n, _)| n == "BLOCKMAP").unwrap().1 =
        words.iter().flat_map(|w| w.to_le_bytes()).collect();
    l
}

/// The E1M8 refusal in a synthetic room: the tic whose pistol shot crosses the
/// stretched wall is refused under wire 26 with `MapScale(Arithmetic)`, the
/// state kept, and completes under wire 27, where the wrapped fraction is
/// negative ("behind source", p_maputl.c:591-592) so the wall is not an
/// intercept: the shot reaches what lies beyond.
#[test]
fn a_wrapping_shot_completes_under_wire_27_where_wire_26_refuses() {
    let l = stretched();
    let (mut v4, mut v5) = (level(&l, false), level(&l, true));
    let mut fire = [TicCommand::default(); 4];
    fire[0].buttons = weapons::BT_ATTACK;
    // The pistol rises for 18 tics (A_Raise, p_pspr.c:429-434) before it fires.
    for t in 0..18 {
        v4.advance(fire).unwrap();
        v5.advance(fire).unwrap();
        assert_eq!(v4.events(), v5.events(), "tic {t}");
    }
    let before = v4.snapshot().canonical_bytes();
    assert_eq!(v4.advance(fire), Err(MapScaleError::Arithmetic.into()));
    assert_eq!(v4.snapshot().canonical_bytes(), before);
    v5.advance(fire).unwrap();
    let trace = v5
        .events()
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Trace { result }) => Some(result.trace.clone()),
            _ => None,
        })
        .unwrap();
    assert!(trace.candidates.contains(&TraceObject::Line(1)));
    assert!(
        trace
            .intercepts
            .iter()
            .all(|i| i.object != TraceObject::Line(1))
    );
    // The same trace against line 1 by the independent transcription: wrapped,
    // and negative.
    let [ox, oy] = trace.origin.map(|v| i64::from(v.bits()));
    let [dx, dy] = trace.delta.map(|v| i64::from(v.bits()));
    let (result, wrapped) = reference(
        [ox, oy],
        [dx, dy],
        [512 << 16, 7872 << 16],
        [0, -8000 << 16],
    );
    assert!(wrapped && result.is_ok_and(|f| f < 0), "{result:?}");
}

fn sight(wire27: bool, v2s: V, v2d: V, v1s: V, v1d: V) -> Option<Result<i32, FixedDivRefusal>> {
    sight_intercept(wire27, fx(v2s), fx(v2d), fx(v1s), fx(v1d)).map(|r| r.map(Fixed::bits))
}

/// The owner's ruling of 2026-11-24: under wire 27 alone sight's
/// `P_InterceptVector2` refuses `FixedDiv(num, den)` (p_sight.c:125) when `num`
/// or `den` is exactly `INT_MIN` (m_fixed.c:62); wire 26 keeps its value. Each
/// vector is computed by hand from p_sight.c:117-124 and none wraps, so
/// `checked_intercept` is `Some` (wire 26's value).
#[test]
fn sight_refuses_an_int_min_operand_under_wire_27_alone() {
    let o = [0, 0];
    // num = INT_MIN: a vertical line of 4,096 units (v1->dy = 2^28) at x = 0 and
    // a one-unit trace from x = 2,048 (v2->x = 2^27, v2->dx = 2^16).
    // den = FixedMul(2^28>>8, 2^16) - FixedMul(0>>8, 0) = 2^36>>16 = 2^20;
    // num = FixedMul((0 - 2^27)>>8, 2^28) + FixedMul((0 - 0)>>8, 0)
    //     = (-2^19 · 2^28)>>16 = -2^31.
    // Wire 26: (2^31>>14) = 2^17 < 2^20, so FixedDiv2: -2^31/2^20·65536 = -2^27.
    let num = ([1 << 27, 0], [65536, 0], o, [0, 1 << 28]);
    // den = INT_MIN: the line at x = 16 (v1->x = 2^20) running 4,096 units south
    // (v1->dy = -2^28), the trace 2,048 units east from the origin (v2->dx = 2^27).
    // den = FixedMul(-2^28>>8, 2^27) - FixedMul(0>>8, 0) = (-2^20 · 2^27)>>16 = -2^31;
    // num = FixedMul((2^20 - 0)>>8, -2^28) + FixedMul((0 - 0)>>8, 0)
    //     = (2^12 · -2^28)>>16 = -2^24.
    // Wire 26: FixedDiv2: -2^24/-2^31·65536 = 512, the fraction 16/2048.
    let den = (o, [1 << 27, 0], [1 << 20, 0], [0, -(1 << 28)]);
    for (v, wire26, operand) in [(num, -(1 << 27), "num"), (den, 512, "den")] {
        assert_eq!(checked(v.0, v.1, v.2, v.3), Some(wire26), "{operand}");
        assert_eq!(
            sight(false, v.0, v.1, v.2, v.3),
            Some(Ok(wire26)),
            "{operand}"
        );
        assert_eq!(
            sight(true, v.0, v.1, v.2, v.3),
            Some(Err(AbsMin)),
            "{operand}"
        );
        // The same expression as P_InterceptVector (p_maputl.c:240-250): the
        // independent transcription agrees that it is AbsMin without a wrap.
        assert_eq!(reference(v.0, v.1, v.2, v.3), (Err(AbsMin), false));
    }
    // The same vectors through the worlds the sight call site reads
    // (`checked_arithmetic`, and `wrapping_intercept` set by wire 27 alone).
    let l = base();
    let (v4, v5) = (level(&l, false), level(&l, true));
    assert!(v4.world.checked_arithmetic && v5.world.checked_arithmetic);
    for (v, wire26) in [(num, -(1 << 27)), (den, 512)] {
        assert_eq!(
            sight(v4.world.wrapping_intercept, v.0, v.1, v.2, v.3),
            Some(Ok(wire26))
        );
        assert_eq!(
            sight(v5.world.wrapping_intercept, v.0, v.1, v.2, v.3),
            Some(Err(AbsMin))
        );
    }
    // den == 0 with the num above (INT_MIN): p_sight.c:119-120 returns 0 before
    // num is computed or FixedDiv is reached, so nothing is refused, under
    // either wire, and P_InterceptVector (p_maputl.c:242-243) agrees.
    // den = FixedMul(2^28>>8, 0) - FixedMul(0>>8, 2^16) = 0.
    let parallel = ([1 << 27, 0], [0, 65536], o, [0, 1 << 28]);
    let v = parallel;
    for wire27 in [false, true] {
        assert_eq!(sight(wire27, v.0, v.1, v.2, v.3), Some(Ok(0)));
    }
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Ok(0));
    assert_eq!(reference(v.0, v.1, v.2, v.3), (Ok(0), false));
    // Overflow is refused as before under both wires (`None`, the existing
    // `Arithmetic` refusal): "a wrong intercept" above, whose den product wraps.
    let wrong = (o, [1 << 30, 0], [1 << 24, 0], [0x7fff_0000; 2]);
    let v = wrong;
    assert_eq!(checked(v.0, v.1, v.2, v.3), None);
    for wire27 in [false, true] {
        assert_eq!(sight(wire27, v.0, v.1, v.2, v.3), None);
    }
}

/// Sight's ordinary values are unchanged under wire 27: the 131 genuine C rows of
/// `P_InterceptVector2` (`tests/reference/m3d3a_fractions.csv`, generated by
/// tools/reference/generate_m3d3a.py from the p_sight.c body) under both wires,
/// and a deterministic sweep in which wire 26 always equals `checked_intercept`
/// and wire 27 differs from it only where [`reference`] finds an `INT_MIN`
/// operand without a wrap.
#[test]
fn ordinary_sight_values_are_unchanged_under_wire_27() {
    let mut rows = 0;
    for line in include_str!("../../../tests/reference/m3d3a_fractions.csv")
        .lines()
        .skip(1)
    {
        let n: Vec<i64> = line.split(',').map(|s| s.parse().unwrap()).collect();
        let a = ([n[0], n[1]], [n[2], n[3]], [n[4], n[5]], [n[6], n[7]]);
        for wire27 in [false, true] {
            assert_eq!(
                sight(wire27, a.0, a.1, a.2, a.3),
                Some(Ok(n[8] as i32)),
                "C row {rows}"
            );
        }
        rows += 1;
    }
    assert_eq!(rows, 131);
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        // A value of 0 to 31 significant bits, either sign.
        (seed as i32 >> (seed >> 59)) as i64
    };
    let mut seen = [0_u32; 2];
    for _ in 0..200_000 {
        let a = (
            [next(), next()],
            [next(), next()],
            [next(), next()],
            [next(), next()],
        );
        let exact = checked(a.0, a.1, a.2, a.3);
        assert_eq!(sight(false, a.0, a.1, a.2, a.3), exact.map(Ok), "{a:?}");
        let expected = match (exact, reference(a.0, a.1, a.2, a.3)) {
            (None, (_, wrapped)) => {
                assert!(wrapped, "{a:?}");
                None
            }
            (Some(_), (Err(AbsMin), false)) => Some(Err(AbsMin)),
            (Some(v), _) => Some(Ok(v)),
        };
        assert_eq!(sight(true, a.0, a.1, a.2, a.3), expected, "{a:?}");
        seen[usize::from(expected.is_some())] += 1;
    }
    assert!(seen.iter().all(|n| *n > 1000), "{seen:?}");
}

/// Ruling 6: a caller that wire 27 keeps checked (the shoot trace against a
/// thing) still refuses an exactly-`INT_MIN` `FixedDiv` operand (m_fixed.c:62)
/// under wire 27, as the wrapping callers do, while wire 26 keeps its value; an
/// ordinary vector (the hand-computed sight vectors of p_sight.c:117-124, the
/// same expression as p_maputl.c:240-250, here through `P_InterceptVector`'s
/// callers) gives wire 26's value under both. `num = INT_MIN` and `den = INT_MIN`
/// are the vectors of [`sight_refuses_an_int_min_operand_under_wire_27_alone`],
/// neither of which wraps.
#[test]
fn a_checked_caller_refuses_an_int_min_operand_under_wire_27() {
    let l = base();
    let (v4, v5) = (level(&l, false), level(&l, true));
    let o = [0, 0];
    let num = ([1 << 27, 0], [65536, 0], o, [0, 1 << 28]);
    let den = (o, [1 << 27, 0], [1 << 20, 0], [0, -(1 << 28)]);
    let ordinary = (o, [1 << 27, 0], [1 << 20, -(1 << 20)], [0, 1 << 21]);
    let (thing, line) = (InterceptCaller::ShootThing, InterceptCaller::ShootLine);
    for (v, wire26) in [(num, -(1 << 27)), (den, 512)] {
        let at = |w: &StaticWorld, c| policy_intercept(w, c, fx(v.0), fx(v.1), fx(v.2), fx(v.3));
        assert_eq!(checked(v.0, v.1, v.2, v.3), Some(wire26));
        assert_eq!(at(&v4.world, thing), Some(Fixed::from_bits(wire26)));
        assert_eq!(at(&v5.world, thing), None);
        assert_eq!(at(&v5.world, line), None);
        assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    }
    let v = ordinary;
    let exact = checked(v.0, v.1, v.2, v.3).unwrap();
    for world in [&v4.world, &v5.world] {
        assert_eq!(
            policy_intercept(world, thing, fx(v.0), fx(v.1), fx(v.2), fx(v.3)),
            Some(Fixed::from_bits(exact))
        );
    }
    assert_eq!(reference(v.0, v.1, v.2, v.3), (Ok(exact), false));
}

/// The owner's corrections of 2026-11-24: [`wire27_admits`] is the goldens'
/// table. Every `T` row of tests/reference/wire27_state.txt (each caller and
/// class, the genuine witness calls over every item and wire 27's decision,
/// tools/reference/wire27_goldens.py) agrees with it, and every wrap class it
/// admits has at least one genuine witnessed call. `INT_MIN` operands and
/// `FixedDiv2`'s range are refused and calls that do not wrap admitted
/// (`each_fixeddiv_refusal_is_named` and the sweeps above).
#[test]
fn wire27_admits_exactly_the_witnessed_classes() {
    let caller = |name: &str| match name {
        "aim" => InterceptCaller::Aim,
        "shoot_line" => InterceptCaller::ShootLine,
        "shoot_thing" => InterceptCaller::ShootThing,
        "slide" => InterceptCaller::Slide,
        "use" => InterceptCaller::Use,
        _ => panic!("{name}"),
    };
    let operand = |part: &str| match part.rsplit('_').next() {
        Some("exact") => Operand::Exact,
        Some("wrong") => Operand::Wrong,
        _ => panic!("{part}"),
    };
    let (mut rows, mut admitted) = (0, 0);
    let text = include_str!("../../../tests/reference/wire27_state.txt");
    for row in text.lines().filter(|l| l.starts_with("T,")) {
        let f: Vec<&str> = row.split(',').collect();
        let (who, parts, calls) = (caller(f[1]), f[2].split('.').collect::<Vec<_>>(), f[3]);
        let class = match parts[..] {
            [_, _, _, "absmin" | "range"] | ["nowrap", ..] => None,
            ["wrap", "den_zero"] => Some(WrapClass::DenZero { exact: false }),
            ["wrap", "den_zero_exact"] => Some(WrapClass::DenZero { exact: true }),
            ["wrap", den, num, "div2"] => Some(WrapClass::Div2 {
                den: operand(den),
                num: operand(num),
            }),
            ["wrap", den, num, branch @ ("max" | "min")] => Some(WrapClass::Saturation {
                den: operand(den),
                num: operand(num),
                max: branch == "max",
            }),
            _ => panic!("{row}"),
        };
        let expected = match class {
            Some(class) => wire27_wraps(who) && wire27_admits(who, class),
            None => !f[2].ends_with(".absmin") && !f[2].ends_with(".range"),
        };
        assert_eq!(f[4] == "admit", expected, "{row}");
        if class.is_some() && expected {
            assert_ne!(
                calls, "0",
                "an admitted wrap class without a genuine call: {row}"
            );
            admitted += 1;
        }
        rows += 1;
    }
    assert_eq!((rows, admitted), (5 * 28, 10));
}

/// The trace and east wall of the genuine shoot items (tests/reference/
/// wire27_results.json): the player's 2,048-unit trace from `start` at fine
/// angle `fine` (P_LineAttack, p_map.c; FINE_SINE is tables.c's) against the
/// wall from (484, 4000) running `dx` units east and 8,059 south.
fn shot(start: V, fine: usize, dx: i64) -> (V, V, V, V) {
    let delta = [
        2048 * i64::from(FINE_SINE[fine + 2048]),
        2048 * i64::from(FINE_SINE[fine]),
    ];
    (
        start,
        delta,
        [484 << 16, 4000 << 16],
        [dx << 16, -8059 << 16],
    )
}

/// The owner's corrections of 2026-11-24 (A and B), on the genuine operands of
/// the witness rows of tests/reference/wire27_results.json: wire 27 reproduces
/// `FixedDiv` (m_fixed.c:57-87) of a wrong num and den for the shoot trace
/// against a line alone (NUMWRAP: FixedDiv2, -6,666,240 / -17,926,360 gives
/// 24,370; SATUR: 1,149,096,960 over 38,768 saturates to MAXINT; SATNEG:
/// 1,147,048,960 over -38,768 to MININT, m_fixed.c:62-63); the aim, slide and
/// use traces refuse these classes (no item), the shoot trace against a thing
/// keeps the checked computation, and wire 26 refuses them all. A saturation
/// whose num is exact ("Saturation of wrapped operands" above) has no item and
/// is refused for every caller.
#[test]
fn a_wrong_num_follows_fixeddiv_for_the_shoot_line_alone() {
    let l = base();
    let (v4, v5) = (level(&l, false), level(&l, true));
    let wrong = |max| WrapClass::Saturation {
        den: Operand::Wrong,
        num: Operand::Wrong,
        max,
    };
    for (v, frac, class) in [
        (
            shot([0, -2150 << 16], 128, 2098),
            24370,
            WrapClass::Div2 {
                den: Operand::Wrong,
                num: Operand::Wrong,
            },
        ),
        (shot([0, 0], 96, 2097), i32::MAX, wrong(true)),
        (shot([0, 0], 96, 2099), i32::MIN, wrong(false)),
    ] {
        let at = |c| wire27_intercept(c, fx(v.0), fx(v.1), fx(v.2), fx(v.3)).map(Fixed::bits);
        assert_eq!(at(InterceptCaller::ShootLine), Ok(frac));
        for c in [
            InterceptCaller::Aim,
            InterceptCaller::Slide,
            InterceptCaller::Use,
        ] {
            let refusal = if frac == 24370 {
                Wire27Refusal::Unevidenced(class)
            } else {
                Wire27Refusal::FixedDiv(Saturation)
            };
            assert_eq!(at(c), Err(refusal), "{c:?}");
        }
        assert!(at(InterceptCaller::ShootThing).is_err());
        assert_eq!(checked(v.0, v.1, v.2, v.3), None);
        let line = InterceptCaller::ShootLine;
        assert_eq!(
            policy_intercept(&v4.world, line, fx(v.0), fx(v.1), fx(v.2), fx(v.3)),
            None
        );
        assert_eq!(
            policy_intercept(&v5.world, line, fx(v.0), fx(v.1), fx(v.2), fx(v.3)),
            Some(Fixed::from_bits(frac))
        );
        // The independent transcription: the same value, or the saturation it names.
        let expected = if frac == 24370 {
            Ok(frac)
        } else {
            Err(Saturation)
        };
        assert_eq!(reference(v.0, v.1, v.2, v.3), (expected, true));
    }
    let v = ([0, 0], [(1 << 26) + 1, 0], [1 << 14, 0], [0, 1 << 30]);
    for c in [
        InterceptCaller::Aim,
        InterceptCaller::ShootLine,
        InterceptCaller::Slide,
    ] {
        let got = wire27_intercept(c, fx(v.0), fx(v.1), fx(v.2), fx(v.3));
        assert_eq!(got, Err(Wire27Refusal::FixedDiv(Saturation)));
    }
}

/// Mutation survivor `intercept_vector.rs:232:24` (`^` to `|` in the saturation
/// branch's sign): `FixedDiv` (m_fixed.c:62-63) returns
/// `(a ^ b) < 0 ? MININT : MAXINT`, so the result is MAXINT when num and den
/// have the same sign and MININT when they differ. Four hand-built wrapped
/// operand sets (found by a transcription of p_maputl.c:240-250 in 64-bit
/// integers; each has `|num| >> 14 >= |den|`, an int `den` and `num` that differ
/// from the exact sums, and neither is `INT_MIN`), one per sign pair. The shoot
/// trace against a line admits only the wrong-den, wrong-num saturation
/// (`wire27_admits`), so `Ok` also pins the class the classifier assigned; the
/// aim trace refuses every one of them. Genuine items cover SATUR (max) and
/// SATNEG (min) only; the sign pairs (num -, den -) and (num -, den +) rest on
/// these unit tests alone. Rust-only.
#[test]
fn a_wrapped_saturation_follows_the_sign_of_num_xor_den() {
    // (operands, num, den, expected): the int num and den are the C values.
    let cases: [([V; 4], i64, i64, i32); 4] = [
        (
            [
                [-571796130, 2877759],
                [-319878240, 442018225],
                [22672059, -153729900],
                [-291317630, -498512367],
            ],
            1091703789,
            38567,
            i32::MAX, // num +, den +
        ),
        (
            [
                [1852928260, -642718635],
                [-2031647816, 28164226],
                [513163277, -880315396],
                [19826816, 460803817],
            ],
            2137441017,
            -16443,
            i32::MIN, // num +, den -
        ),
        (
            [
                [1291374679, 29445886],
                [399572670, -989149075],
                [-483358055, -19847005],
                [165225401, 492665558],
            ],
            -90251811,
            376,
            i32::MIN, // num -, den +
        ),
        (
            [
                [1726930188, 32846029],
                [1696862486, 11252231],
                [78367699, -781942984],
                [-22306394, -339869832],
            ],
            -2046765681,
            -48923,
            i32::MAX, // num -, den -
        ),
    ];
    for (v, num, den, expected) in cases {
        assert!((num.abs() >> 14) >= den.abs(), "{num} {den}");
        let at = |c| wire27_intercept(c, fx(v[0]), fx(v[1]), fx(v[2]), fx(v[3])).map(Fixed::bits);
        assert_eq!(at(InterceptCaller::ShootLine), Ok(expected), "{num} {den}");
        assert_eq!(
            at(InterceptCaller::Aim),
            Err(Wire27Refusal::FixedDiv(Saturation))
        );
        assert_eq!(wrapping(v[0], v[1], v[2], v[3]), Err(Saturation));
        assert_eq!(checked(v[0], v[1], v[2], v[3]), None);
    }
}
