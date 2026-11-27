//! The slide trace's checked flag, `checked_arithmetic && limit.is_some()` in
//! `Mover::slide` (certificate audit 1, item 5: the mutation survivor "replace &&
//! with ||" of `docs/validation/WIRE27.md` section 9). At WRAP's tic 52
//! (`tests/overshoot_walk.rs`) the genuine engine's notch-edge intercepts narrow a
//! `FixedMul` product (m_fixed.c:44-49, p_maputl.c:246-248), come out negative and
//! are dropped (p_maputl.c:591-592): no `L` rows. The plain wrapping intercept gives
//! that; the checked one refuses. No configuration builds a checked world with a slide
//! access without an intercept limit (`checked_arithmetic` comes with the Episode 1
//! bounds alone, whose players all have one; non-players never slide), so the second
//! test builds it by hand: `||` would take the checked path and reach the plain
//! access's `intercept_arithmetic`, which panics.
use crate::intercept_vector::InterceptCaller;
use crate::*;

fn wrap_at_tic_52() -> Simulation {
    let bytes = include_bytes!("../../../tests/reference/overshoot_wads/overshoot.wad");
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "E1M2", doom_map::MapLimits::default()).unwrap();
    let start = LevelStart::NewGame;
    let options = MapContextOptions::default();
    let mut s =
        Simulation::original_player_episode1_v5_level(map, 2, Skill::Medium, options, start)
            .unwrap();
    let lmp = include_bytes!("../../../tests/reference/overshoot_scripts/WRAP.lmp");
    let commands = demo::parse_demo_episode1_v2(lmp, lmp[3])
        .unwrap()
        .1
        .commands;
    let none = TicCommand::default();
    for c in &commands[..52] {
        s.advance([*c, none, none, none]).unwrap();
    }
    assert_eq!(
        s.advance([commands[52], none, none, none]),
        Err(MapScaleError::Arithmetic.into())
    );
    s
}

#[test]
fn the_plain_intercept_gives_the_genuine_none_and_the_checked_one_refuses() {
    let text = include_str!("../../../tests/reference/overshoot_state.txt");
    let row = text.lines().find(|l| l.starts_with("O,WRAP,52,")).unwrap();
    let o: Vec<i32> = row.split(',').skip(3).map(|v| v.parse().unwrap()).collect();
    assert!(
        !text.lines().any(|l| l.starts_with("L,WRAP,")),
        "the genuine run met no line"
    );
    let s = wrap_at_tic_52();
    let f = Fixed::from_bits;
    let (start, end) = ([f(o[0]), f(o[1])], [f(o[0] + o[2]), f(o[1] + o[3])]);
    let caller = InterceptCaller::Slide;
    assert_eq!(
        movement::path(&s.world, start, end, Some(128), false, caller),
        Ok(vec![])
    );
    let refused = Err(movement::PathError::Arithmetic);
    assert_eq!(
        movement::path(&s.world, start, end, Some(128), true, caller),
        refused
    );
}

#[test]
fn a_slide_access_without_a_limit_never_takes_the_checked_intercept() {
    let s = wrap_at_tic_52();
    assert!(s.world.checked_arithmetic && s.world.wrapping_intercept);
    let id = s.original_player.as_ref().unwrap().mo.unwrap();
    let lmp = include_bytes!("../../../tests/reference/overshoot_scripts/WRAP.lmp");
    let command = demo::parse_demo_episode1_v2(lmp, lmp[3])
        .unwrap()
        .1
        .commands[52];
    // P_MovePlayer's thrust of that command (fine angle 0; P_Thrust,
    // p_user.c:59-68): the momentum P_XYMovement starts from.
    let mut actor = *s.actor(id).unwrap();
    let thrust = Fixed::from_bits(i32::from(command.forward) * 2048);
    let [cos, sin] = [crate::FINE_SINE[2048], crate::FINE_SINE[0]].map(Fixed::from_bits);
    let [mx, my] = actor.spec.momentum;
    actor.spec.momentum = [mx + thrust * cos, my + thrust * sin];
    assert_eq!(
        actor.spec.momentum[1].bits(),
        9,
        "the y momentum of WRAP's tic 52"
    );
    let mut moved = Vec::new();
    for checked in [true, false] {
        let mut world = s.world.clone();
        world.checked_arithmetic = checked;
        let (mut a, mut spatial) = (actor, s.spatial.clone());
        let get = |i: ActorId| s.actor(i).copied();
        let events = movement::move_xy(&world, id, &mut a, &mut spatial, get, Some(command));
        moved.push((a.spec.position, a.spec.momentum, events.len()));
    }
    assert_eq!(moved[0], moved[1]);
    assert_ne!(moved[0].0, actor.spec.position, "the player moved");
}
