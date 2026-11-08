//! M4a Episode 1 policy, plan step 7: the command domain of decision H5
//! (`forwardmove`/`sidemove` −127..127, −128 refused for both (D9), buttons 0x40
//! and `BT_SPECIAL` 0x80 refused, `BT_CHANGE` with weapon indices 0–7, players
//! 2–4 empty). Rust-only policy tests: the expected values come from the source
//! reading (p_user.c, p_mobj.c, m_fixed.c, tables.c), never from the C engine or
//! from Rust output.
use crate::diagonal_room::Lumps;
use crate::*;

const EPISODE1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const LEVEL: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersLevelV1;

/// The doorway room (west room x −512..0, y −384..384) with the player facing
/// east (angle 0) at `(x, 0)`.
fn lumps(x: i16) -> Lumps {
    let mut l = crate::player_movers_tests::room(0);
    l.add_thing([x, 0, 0, 1, 7]);
    l
}
fn sim(l: &Lumps, policy: OriginalExecutionProfile) -> Simulation {
    let options = MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    };
    let mut s = if policy == EPISODE1 {
        Simulation::original_player_episode1_map(l.map(), options)
    } else {
        Simulation::original_player_level_map(l.map(), options)
    }
    .unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn command(forward: i8, side: i8, buttons: u8) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward,
        side,
        angle_turn: 0,
        buttons,
    };
    c
}
fn runtime(s: &Simulation) -> &original_player::PlayerRuntime {
    s.original_player.as_ref().unwrap()
}
fn body(s: &Simulation) -> Actor {
    *s.actor(runtime(s).mo.unwrap()).unwrap()
}
fn pack(s: &Simulation) -> PlayerWeaponState {
    runtime(s).weapon.unwrap().pack().unwrap()
}
fn decode(s: &Simulation) {
    let capture = s.snapshot().canonical_bytes();
    Diagnostic::decode_episode1(&capture, DiagnosticLimits::default()).unwrap();
}
/// `advance(commands)` fails with `error`, the canonical state is unchanged and
/// a valid tic still advances from it.
fn refused(s: &mut Simulation, commands: [TicCommand; 4], error: SimError) {
    let before = s.snapshot().canonical_bytes();
    assert_eq!(s.advance(commands), Err(error), "{commands:?}");
    assert_eq!(s.snapshot().canonical_bytes(), before, "{commands:?}");
    let mut probe = s.clone();
    probe.advance(command(0, 0, 0)).unwrap();
}

/// One tic of a grounded player facing east (angle 0) at rest, as C runs it:
/// `P_MovePlayer` (p_user.c:154–164) calls `P_Thrust(angle, move*2048)` for the
/// forward move at `angle` and the side move at `angle − ANG90`; `P_Thrust`
/// (p_user.c:59–68) adds `FixedMul(move, finecosine[a])` to `momx` and
/// `FixedMul(move, finesine[a])` to `momy`, `a = angle >> 19`; `FixedMul` is
/// `((long long)a*b) >> 16` (m_fixed.c:44–50, arithmetic shift: floor).
/// tables.c: `finesine[0] = 25`, `finecosine[0] = finesine[2048] = 65535`,
/// `finesine[6144] = −65535`, `finecosine[6144] = finesine[8192] = 25`.
/// Then `P_XYMovement` (p_mobj.c:120–241) moves the body by the momentum (one
/// step: |momx|, |momy| ≤ `MAXMOVE/2`) and applies `FRICTION` 0xe800 with a
/// nonzero command: `momx = FixedMul(momx, 0xe800)`.
///
/// | move | thrust (momx, momy) | after friction |
/// | --- | --- | --- |
/// | forward 100 (204,800) | (204,800·65,535>>16 = 204,796, 204,800·25>>16 = 78) | (185,596, 70) |
/// | forward 127 (260,096) | (260,092, 99) | (235,708, 89) |
/// | forward −127 (−260,096) | (−260,093, −100) | (−235,710, −91) |
/// | side 127 (heading 6144) | (260,096·25>>16 = 99, 260,096·−65,535>>16 = −260,093) | (89, −235,710) |
/// | side −127 | (−100, 260,092) | (−91, 235,708) |
///
/// The largest product, 260,096·65,535 ≈ 1.7·10^10, is formed in 64 bits by
/// `FixedMul` and its result (≤ 260,096) is far inside `i32`.
#[test]
fn full_range_thrust_matches_p_thrust() {
    let x0 = -256 * 65536;
    for (forward, side, thrust, momentum) in [
        (100, 0, [204_796, 78], [185_596, 70]),
        (127, 0, [260_092, 99], [235_708, 89]),
        (-127, 0, [-260_093, -100], [-235_710, -91]),
        (0, 127, [99, -260_093], [89, -235_710]),
        (0, -127, [-100, 260_092], [-91, 235_708]),
    ] {
        let mut s = sim(&lumps(-256), EPISODE1);
        assert_eq!(body(&s).position().map(Fixed::bits), [x0, 0]);
        s.advance(command(forward, side, 0)).unwrap();
        decode(&s);
        let b = body(&s);
        assert_eq!(
            b.position().map(Fixed::bits),
            [x0 + thrust[0], thrust[1]],
            "forward {forward} side {side}"
        );
        assert_eq!(
            b.momentum().map(Fixed::bits),
            momentum,
            "forward {forward} side {side}"
        );
        assert_eq!(s.commands[0].forward, forward);
        assert_eq!(s.commands[0].side, side);
    }
}

/// H5/D9 boundaries under wire 23: ±127 admitted on both axes, −128 refused on
/// both (`PlayerBodyError::Unsupported`), buttons 0x40 and 0x80 refused
/// (`UnsupportedButtons`, p_user.c:280–281 and g_game.c:700–720 are M4c),
/// weapon bits without `BT_CHANGE` refused, players 2–4 must be empty; every
/// refusal keeps the complete canonical state. Wire 22 keeps ±50.
#[test]
fn command_domain_boundaries() {
    let l = lumps(-256);
    for (forward, side) in [(127, 0), (-127, 0), (0, 127), (0, -127), (127, -127)] {
        let mut s = sim(&l, EPISODE1);
        s.advance(command(forward, side, 0)).unwrap();
    }
    let mut s = sim(&l, EPISODE1);
    let unsupported = SimError::from(PlayerBodyError::Unsupported);
    refused(&mut s, command(-128, 0, 0), unsupported);
    refused(&mut s, command(0, -128, 0), unsupported);
    refused(&mut s, command(-128, -128, 0), unsupported);
    for buttons in [
        0x40,
        0x80,
        0xc0,
        0x40 | BT_ATTACK,
        0x80 | BT_USE,
        0x80 | 2 << 3,
    ] {
        refused(&mut s, command(0, 0, buttons), SimError::UnsupportedButtons);
    }
    refused(&mut s, command(0, 0, 3 << 3), SimError::UnsupportedButtons);
    for slot in 1..4 {
        let mut c = command(0, 0, 0);
        c[slot].forward = 1;
        refused(&mut s, c, SimError::from(PlayerBodyError::Slot));
    }
    // Wire 22 is unchanged: ±50 only.
    let mut w22 = sim(&l, LEVEL);
    for (forward, side) in [(51, 0), (-51, 0), (0, 51), (0, -51), (127, 0), (0, -127)] {
        refused(&mut w22, command(forward, side, 0), unsupported);
    }
    w22.advance(command(50, -50, 0)).unwrap();
}

/// p_user.c:283–312 with the default loadout (fist and pistol owned, pistol
/// ready): index 0 selects the fist (no chainsaw), 1 is the ready weapon and
/// 2–7 are not owned, so all are ignored; owning every Episode 1 weapon, 0 and 7
/// select the chainsaw, 2–4 select theirs and 5/6 (plasma, BFG: never owned
/// here) stay ignored. Every index is an admitted command under wire 23.
#[test]
fn weapon_indices_0_to_7() {
    let l = lumps(-256);
    for index in 0..8u8 {
        let mut s = sim(&l, EPISODE1);
        let before = pack(&s).pending;
        s.advance(command(0, 0, BT_CHANGE | index << 3)).unwrap();
        decode(&s);
        let expected = if index == 0 {
            Some(Weapon::Fist)
        } else {
            before
        };
        assert_eq!(pack(&s).pending, expected, "index {index}");
    }
    for (index, expected) in [
        (0, Some(Weapon::Chainsaw)),
        (2, Some(Weapon::Shotgun)),
        (3, Some(Weapon::Chaingun)),
        (4, Some(Weapon::Rocket)),
        (5, None),
        (6, None),
        (7, Some(Weapon::Chainsaw)),
    ] {
        let mut s = sim(&l, EPISODE1);
        arm_all(&mut s);
        s.advance(command(0, 0, BT_CHANGE | index << 3)).unwrap();
        decode(&s);
        assert_eq!(pack(&s).pending, expected, "index {index}");
    }
}

/// The pistol raised and ready (its ready state with one tic left), owning
/// fist, pistol, shotgun, chaingun, launcher and chainsaw, with ammo for each.
fn arm_all(s: &mut Simulation) {
    let p = s.original_player.as_mut().unwrap();
    let Some(player_weapons::PlayerWeaponRuntime::Pack(w)) = p.weapon.as_mut() else {
        panic!("pack");
    };
    w.ready = Weapon::Pistol;
    w.pending = None;
    w.owned = 0b1001_1111;
    w.shells = 8;
    w.rockets = 2;
    w.psprites[0].state = Some(Weapon::Pistol.states()[2]);
    w.psprites[0].tics = 1;
    w.psprites[0].position = [Fixed::ONE, Fixed::from_map_units(32)];
}

/// A dead player's command is admitted inside the wire-23 domain as under wire
/// 22 (`P_DeathThink` reads only `BT_USE`, p_user.c:227), the domain itself
/// still applies: −128 and 0x80 are refused with the state kept.
#[test]
fn dead_player_commands() {
    // ROCKDIE in miniature: the rocket explodes at the touching west wall.
    let mut l = crate::player_movers_tests::room(0);
    l.add_thing([-496, 0, 180, 1, 7]);
    let mut s = sim(&l, EPISODE1);
    {
        let p = s.original_player.as_mut().unwrap();
        let Some(player_weapons::PlayerWeaponRuntime::Pack(w)) = p.weapon.as_mut() else {
            panic!("pack");
        };
        w.ready = Weapon::Rocket;
        w.pending = None;
        w.owned = 3 | 1 << 4;
        w.rockets = 2;
        w.psprites[0].state = Some(Weapon::Rocket.states()[2]);
        w.psprites[0].tics = 1;
        w.psprites[0].position = [Fixed::ONE, Fixed::from_map_units(32)];
    }
    s.advance(command(0, 0, 0)).unwrap();
    for _ in 0..12 {
        s.advance(command(0, 0, BT_ATTACK)).unwrap();
        if runtime(&s).state == PlayerBodyState::Dead {
            break;
        }
    }
    assert_eq!(runtime(&s).state, PlayerBodyState::Dead);
    refused(
        &mut s,
        command(-128, 0, 0),
        PlayerBodyError::Unsupported.into(),
    );
    refused(&mut s, command(0, 0, 0x80), SimError::UnsupportedButtons);
    s.advance(command(127, -127, BT_ATTACK | BT_CHANGE | 7 << 3))
        .unwrap();
    decode(&s);
    assert_eq!(runtime(&s).state, PlayerBodyState::Dead);
}

/// Forward 127 held for 14 tics from rest (x −480, facing east): each tic adds
/// the thrust (260,092, 99) of the table above, `P_XYMovement` clamps each axis
/// to `MAXMOVE` 30·65,536 (p_mobj.c:137–145), moves in two halves when a
/// component exceeds `MAXMOVE/2` (p_mobj.c:150–160: `x + xmove/2` then
/// `xmove >>= 1` again, so an odd component loses one raw unit) and applies
/// friction. Iterating that C arithmetic by hand gives the momenta below (the
/// clamp first binds at tic 13: `FixedMul(1,966,080, 0xe800) = 1,781,760`) and
/// the final position (−12,821,580, 7,106).
#[test]
fn sustained_forward_127_reaches_maxmove() {
    let expected = [
        [235_708, 89],
        [449_318, 170],
        [642_902, 243],
        [818_338, 309],
        [977_327, 369],
        [1_121_410, 424],
        [1_251_986, 473],
        [1_370_320, 518],
        [1_477_560, 559],
        [1_574_747, 596],
        [1_662_822, 629],
        [1_742_640, 659],
        [1_781_760, 686],
        [1_781_760, 711],
    ];
    let mut s = sim(&lumps(-480), EPISODE1);
    for (tic, momentum) in expected.iter().enumerate() {
        s.advance(command(127, 0, 0)).unwrap();
        assert_eq!(
            body(&s).momentum().map(Fixed::bits),
            *momentum,
            "tic {}",
            tic + 1
        );
    }
    decode(&s);
    assert_eq!(body(&s).position().map(Fixed::bits), [-12_821_580, 7_106]);
}

/// `G_Ticker`'s turbo message (g_game.c:671–679), read from the source:
/// before `P_Ticker`, `players[consoleplayer].message` becomes
/// `"%s is turbo!"` when the read command's `forwardmove > TURBOTHRESHOLD`
/// (0x32) and `!(gametic & 31) && ((gametic >> 5) & 3) == 0` for player 0, so
/// on gametics that are multiples of 128 only; gametic is the completed-tic
/// count (`TryRunTics` increments it after `G_Ticker`, d_net.c:742–746). The
/// message is kept until another assignment (DSIM39 `pickups.message` code 30);
/// forward 50, negative forward and other gametics assign nothing.
#[test]
fn turbo_message_on_every_128th_gametic() {
    let l = lumps(-256);
    let mut s = sim(&l, EPISODE1);
    let message = |s: &Simulation| runtime(s).pickups.unwrap().message;
    // (gametic, forward) of every nonzero command; the others are zero.
    let schedule = [
        (0, 50),
        (1, -50),
        (64, 127),
        (65, -127),
        (128, -127),
        (129, 127),
        (256, 51),
        (257, -51),
        (384, 127),
        (385, -127),
    ];
    let mut expected = PickupMessage::None;
    for gametic in 0..400u64 {
        assert_eq!(s.tic(), gametic);
        let forward = schedule
            .iter()
            .find(|(g, _)| *g == gametic)
            .map_or(0, |(_, f)| *f);
        if forward > 50 && gametic % 128 == 0 {
            expected = PickupMessage::Turbo;
        }
        s.advance(command(forward, 0, 0)).unwrap();
        assert_eq!(message(&s), expected, "after gametic {gametic}");
        if gametic == 255 {
            assert_eq!(
                expected,
                PickupMessage::None,
                "0, 64 and 128 assign nothing"
            );
        }
    }
    assert_eq!(message(&s), PickupMessage::Turbo);
    let capture = s.snapshot().canonical_bytes();
    let snap = s.snapshot();
    let value = snap
        .fields()
        .find(|f| f.path() == "original_player.0.pickups.message")
        .unwrap()
        .value()
        .to_vec();
    assert_eq!(value, [30]);
    Diagnostic::decode_episode1(&capture, DiagnosticLimits::default()).unwrap();
    // A refused tic on a turbo gametic (forward 127, button 0x40) assigns nothing.
    let mut t = sim(&l, EPISODE1);
    refused(&mut t, command(127, 0, 0x40), SimError::UnsupportedButtons);
    assert_eq!(message(&t), PickupMessage::None);
    // Wire 22 never reaches it: forward 50 is its maximum.
    let mut w = sim(&l, LEVEL);
    w.advance(command(50, 0, 0)).unwrap();
    assert_eq!(message(&w), PickupMessage::None);
}

/// p_user.c:157–164: `onground = (player->mo->z <= player->mo->floorz)`, and the side
/// move thrusts only `if (cmd->sidemove && onground)`; an airborne body has no friction
/// either (p_mobj.c:205), so a side move changes nothing of its motion. The player
/// walks east off a 64-unit ledge; at the first tic boundary with `z > floorz` a tic
/// with side move 50 and a tic with no move leave the same position, angle and
/// momentum. On the ground before the walk, the same side move does change the
/// momentum (the test can fail).
#[test]
fn an_airborne_body_ignores_its_side_move() {
    use crate::heights_room::HeightsRoom;
    let room = HeightsRoom::new(&[-512, 0, 512], &[[64, 256], [0, 256]])
        .with_things(&[[-40, 0, 0, 1, 7]]);
    let mut s = sim(&Lumps::parse(&room.wad()), EPISODE1);
    let motion = |s: &Simulation| {
        let b = body(s);
        (b.position(), b.spec.angle, b.spec.momentum, b.z, b.momz)
    };
    let (mut side, mut still) = (s.clone(), s.clone());
    side.advance(command(0, 50, 0)).unwrap();
    still.advance(command(0, 0, 0)).unwrap();
    assert_ne!(motion(&side).2, motion(&still).2, "grounded side move");
    let mut tics = 0;
    while body(&s).z <= body(&s).floor {
        s.advance(command(50, 0, 0)).unwrap();
        tics += 1;
        assert!(tics < 60, "the body never left the ledge");
    }
    let (mut side, mut still) = (s.clone(), s.clone());
    side.advance(command(0, 50, 0)).unwrap();
    still.advance(command(0, 0, 0)).unwrap();
    assert_eq!(motion(&side), motion(&still));
}
