//! M4a Episode 1 policy, plan step 5: the keycards, the power-ups, the backpack
//! and rockets (`P_TouchSpecialThing`, `P_GiveCard`, `P_GivePower`, `P_GiveAmmo`),
//! the power countdowns of `P_PlayerThink`, the radiation suit and sector special
//! 11 in `P_PlayerInSpecialSector` with the `P_DamageMobj` cap, the candle and
//! firestick decorations, and the squashed non-shootable ruling. Rust-only policy
//! tests: the expected values come from the source reading (p_inter.c, p_user.c,
//! p_spec.c, p_map.c, info.c), never from the C engine; the C-side checks are the
//! DSIM39 goldens of the M4a synthetic WADs (ITEMS1, TIMERS, SUIT1, SUITDIE,
//! INVIS1, SECT11, DOORS1–3).
use crate::diagonal_room::Lumps;
use crate::world_pickups::PickupEvent;
use crate::*;

const EPISODE1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const LEVEL: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersLevelV1;

fn room() -> Lumps {
    crate::player_movers_tests::room(0)
}
fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn set_sector_special(l: &mut Lumps, sector: usize, special: i16) {
    lump(l, "SECTORS")[26 * sector + 22..26 * sector + 24].copy_from_slice(&special.to_le_bytes());
}
fn thing(l: &mut Lumps, x: i16, y: i16, kind: i16) {
    l.add_thing([x, y, 0, kind, 7]);
}
fn options() -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    }
}
fn load(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let result = if policy == EPISODE1 {
        Simulation::original_player_episode1_map(l.map(), options())
    } else {
        Simulation::original_player_level_map(l.map(), options())
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn try_sim(l: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let mut s = load(l, policy)?;
    s.spawn_map_things()?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())?;
    s.spawn_level_specials()?;
    Ok(s)
}
fn sim(l: &Lumps) -> Simulation {
    try_sim(l, EPISODE1).unwrap()
}
/// The doorway room (0) with the player at (−32, 0) facing east and `items`
/// overlapping it from the west (8 units away, inside the 36-unit touch box).
fn with_items(items: &[i16]) -> Lumps {
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    for &kind in items {
        thing(&mut l, -40, 0, kind);
    }
    l
}
fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn step() -> [TicCommand; 4] {
    let mut c = zero();
    c[0].forward = 1;
    c
}
fn runtime(s: &Simulation) -> &original_player::PlayerRuntime {
    s.original_player.as_ref().unwrap()
}
fn runtime_mut(s: &mut Simulation) -> &mut original_player::PlayerRuntime {
    s.original_player.as_mut().unwrap()
}
fn powers(s: &Simulation) -> PlayerPowers {
    s.original_player_powers(PlayerSlot::One).unwrap()
}
fn pickups(s: &Simulation) -> world_pickups::PlayerPickupState {
    s.original_player_pickups(PlayerSlot::One).unwrap()
}
fn pack(s: &Simulation) -> PlayerWeaponState {
    runtime(s).weapon.unwrap().pack().unwrap()
}
fn body(s: &Simulation) -> Actor {
    *s.actor(runtime(s).mo.unwrap()).unwrap()
}
fn decode(s: &Simulation) {
    let capture = s.snapshot().canonical_bytes();
    Diagnostic::decode_episode1(&capture, DiagnosticLimits::default()).unwrap();
}
fn pickup_events(s: &Simulation) -> Vec<PickupEvent> {
    s.events
        .iter()
        .filter_map(|e| match e {
            Event::Pickup(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}
/// Live (not pending) actors of `kind`'s DoomEd type.
fn live_of(s: &Simulation, ty: identity::OriginalActorType) -> usize {
    s.actors.live_of_type(ActorTypeId(ty.ordinal()))
}
fn units(v: i16) -> Fixed {
    Fixed::from_map_units(v)
}

/// Every new pickup (the chainsaw and the launcher since the weapons step) and
/// both new decorations load under wire 23 only.
#[test]
fn episode1_items_and_statics_load_only_under_wire_23() {
    for kind in [
        5, 6, 13, 2013, 2024, 2025, 2026, 2045, 8, 2010, 2046, 2005, 2003, 34, 46,
    ] {
        let mut l = room();
        thing(&mut l, -32, 0, 1);
        thing(&mut l, -128, 0, kind);
        let s = try_sim(&l, EPISODE1).unwrap_or_else(|e| panic!("{kind}: {e:?}"));
        decode(&s);
        assert_eq!(
            try_sim(&l, LEVEL).err(),
            Some(SimError::MapSpawn(MapSpawnError::UnsupportedType(kind))),
            "{kind}"
        );
    }
    // MF_COUNTITEM: soulsphere, invisibility, map and visor count; the keys, the
    // suit, the backpack and the rockets do not (info.c).
    let s = sim(&with_items(&[
        5, 2013, 2024, 2025, 2026, 2045, 8, 2010, 2046,
    ]));
    assert_eq!(s.statistics.unwrap().total_items, 4);
}

/// A keycard sets its card and assigns `bonuscount = BONUSADD` before the tail
/// adds another 6; a second card of the same colour assigns neither the message
/// nor the bonus (p_inter.c:274–283, 421–443) but is still removed.
#[test]
fn keycards_assign_the_card_message_and_bonus_once() {
    let mut s = sim(&with_items(&[5, 5]));
    s.advance(step()).unwrap();
    assert_eq!(powers(&s).cards, [true, false, false, false, false, false]);
    let p = pickups(&s);
    assert_eq!(p.message, PickupMessage::BlueCard);
    // 6 (P_GiveCard) + 6 (tail) + 6 (second tail). The body moves (and
    // touches) in its mobj thinker, after `P_PlayerThink`'s countdowns.
    assert_eq!(p.bonus_count, 18);
    assert_eq!(p.item_count, 0);
    assert_eq!(live_of(&s, identity::OriginalActorType::MT_MISC4), 0);
    let cards: Vec<_> = pickup_events(&s)
        .into_iter()
        .filter(|e| matches!(e, PickupEvent::Card(..)))
        .collect();
    assert_eq!(
        cards,
        [PickupEvent::Card(0, false), PickupEvent::Card(0, true)]
    );
    let messages = pickup_events(&s)
        .into_iter()
        .filter(|e| matches!(e, PickupEvent::Message(_)))
        .count();
    assert_eq!(messages, 1);
    decode(&s);
    // Yellow and red map to card_t 1 and 2.
    let mut s = sim(&with_items(&[6, 13]));
    s.advance(step()).unwrap();
    assert_eq!(powers(&s).cards, [false, true, true, false, false, false]);
    // Touched newest first: the red card, then the yellow one.
    assert_eq!(pickups(&s).message, PickupMessage::YellowCard);
}

/// The soulsphere adds 100 up to 200; the powers set their durations (the body
/// touches in its mobj thinker, after `P_PlayerThink`'s countdowns, so the first
/// countdown is the next tic's); invisibility sets `MF_SHADOW` on the body; a
/// second computer map is refused and stays (p_inter.c:290–330, 400–407,
/// 504–530; p_user.c:339–353). Things overlapping one spot are touched newest
/// first (each spawn is linked at the head of its block list).
#[test]
fn soulsphere_and_powers() {
    let mut s = sim(&with_items(&[2013, 2024, 2025, 2026, 2045, 2026]));
    s.advance(step()).unwrap();
    assert_eq!(runtime(&s).health, 200);
    assert_eq!(body(&s).life.unwrap().health, 200);
    assert_eq!(powers(&s).powers, [0, 0, 2100, 2100, 1, 4200]);
    assert_ne!(body(&s).life.unwrap().flags & original_player::MF_SHADOW, 0);
    let p = pickups(&s);
    assert_eq!(p.message, PickupMessage::Soulsphere);
    assert_eq!(p.item_count, 4);
    assert_eq!(s.statistics.unwrap().total_items, 5);
    assert_eq!(live_of(&s, identity::OriginalActorType::MT_MISC15), 1);
    assert!(pickup_events(&s).contains(&PickupEvent::Power(4, false, 1, 1)));
    assert!(pickup_events(&s).contains(&PickupEvent::Soulsphere(100, 200, 200)));
    decode(&s);
    // A soulsphere caps at 200.
    let mut s = sim(&with_items(&[2013, 2013]));
    s.advance(step()).unwrap();
    assert_eq!(runtime(&s).health, 200);
}

/// `pw_invisibility` counts down and clears `MF_SHADOW` only when it reaches 0
/// (p_user.c:345–347); a dead player's counters do not move (258–262).
#[test]
fn invisibility_runs_out_and_clears_the_shadow() {
    let mut s = sim(&with_items(&[2024]));
    s.advance(step()).unwrap();
    assert_eq!(powers(&s).powers[2], 2100);
    s.advance(zero()).unwrap();
    assert_eq!(powers(&s).powers[2], 2099);
    runtime_mut(&mut s).powers.as_mut().unwrap().powers[2] = 2;
    s.advance(zero()).unwrap();
    assert_eq!(powers(&s).powers[2], 1);
    assert_ne!(body(&s).life.unwrap().flags & original_player::MF_SHADOW, 0);
    decode(&s);
    s.advance(zero()).unwrap();
    assert_eq!(powers(&s).powers[2], 0);
    assert_eq!(body(&s).life.unwrap().flags & original_player::MF_SHADOW, 0);
    decode(&s);
    s.advance(zero()).unwrap();
    assert_eq!(powers(&s).powers[2], 0);
}

/// The backpack doubles every maximum once and gives one clip load of each type
/// in order (p_inter.c:589–599); a second one only gives the ammo.
#[test]
fn backpack_doubles_the_maxima_once() {
    let mut s = sim(&with_items(&[8]));
    s.advance(step()).unwrap();
    let w = pack(&s);
    assert!(w.backpack);
    assert_eq!(w.max_ammo(), [400, 100, 600, 100]);
    assert_eq!(w.ammo(), [60, 4, 20, 1]);
    assert_eq!(pickups(&s).message, PickupMessage::Backpack);
    decode(&s);
    let mut s = sim(&with_items(&[8, 8]));
    s.advance(step()).unwrap();
    let w = pack(&s);
    assert_eq!(w.max_ammo(), [400, 100, 600, 100]);
    assert_eq!(w.ammo(), [70, 8, 40, 2]);
    let backpacks: Vec<_> = pickup_events(&s)
        .into_iter()
        .filter(|e| matches!(e, PickupEvent::Backpack(_)))
        .collect();
    assert_eq!(
        backpacks,
        [PickupEvent::Backpack(true), PickupEvent::Backpack(false)]
    );
    decode(&s);
}

/// Rockets: one per rocket, five per box, refused at `maxammo` (the item stays).
#[test]
fn rockets_are_capped_at_the_maximum() {
    let mut s = sim(&with_items(&[2046, 2010]));
    s.advance(step()).unwrap();
    assert_eq!(pack(&s).rockets, 6);
    assert_eq!(pickups(&s).message, PickupMessage::RocketBox);
    decode(&s);
    let mut s = sim(&with_items(&[2010]));
    if let Some(player_weapons::PlayerWeaponRuntime::Pack(w)) = runtime_mut(&mut s).weapon.as_mut()
    {
        w.rockets = 50;
    }
    s.advance(step()).unwrap();
    assert_eq!(pack(&s).rockets, 50);
    assert_eq!(live_of(&s, identity::OriginalActorType::MT_MISC18), 1);
    assert_eq!(pickups(&s).message, PickupMessage::None);
}

/// The player's sector, read after a load.
fn player_sector(l: &Lumps) -> usize {
    let s = sim(l);
    usize::from(body(&s).location.unwrap().sector)
}
fn set_health(s: &mut Simulation, health: i32) {
    runtime_mut(s).source.health = health;
    let id = runtime(s).mo.unwrap();
    s.actors.get_mut(id).unwrap().life.as_mut().unwrap().health = health;
}

/// Sector special 11 (p_spec.c:1054–1063): 20 points on the leveltime gate, the
/// `P_DamageMobj` cap at `health − 1` (p_inter.c:837–842), and `G_ExitLevel`
/// once health is at most 10; the next tic is refused. Wire 22 refuses the
/// special at load.
#[test]
fn sector_special_11_damages_caps_and_exits() {
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    let sector = player_sector(&l);
    set_sector_special(&mut l, sector, 11);
    assert_eq!(
        load(&l, LEVEL).err(),
        Some(SimError::Pursuit(PursuitError::Specials))
    );
    let mut s = sim(&l);
    // Tic 0 is on the gate: 20 points.
    s.advance(zero()).unwrap();
    assert_eq!(runtime(&s).health, 80);
    assert!(!s.level_exit);
    for _ in 1..32 {
        s.advance(zero()).unwrap();
    }
    assert_eq!(runtime(&s).health, 80);
    // The cap: 20 ≥ 15 becomes 14 points, leaving health 1, at most 10: exit.
    set_health(&mut s, 15);
    s.advance(zero()).unwrap();
    assert_eq!(runtime(&s).health, 1);
    assert!(s.level_exit && !s.level_secret_exit);
    assert!(
        s.events
            .iter()
            .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { secret: false })))
    );
    decode(&s);
    assert_eq!(s.advance(zero()), Err(SimError::LevelExited));
    // Uncapped (20 < 40) and above 10 afterwards: no exit.
    let mut s = sim(&l);
    s.advance(zero()).unwrap();
    for _ in 1..32 {
        s.advance(zero()).unwrap();
    }
    set_health(&mut s, 40);
    s.advance(zero()).unwrap();
    assert_eq!(runtime(&s).health, 20);
    assert!(!s.level_exit);
    // The exit test also runs off the gate: health already at 10.
    let mut s = sim(&l);
    s.advance(zero()).unwrap();
    set_health(&mut s, 10);
    s.advance(zero()).unwrap();
    assert_eq!(runtime(&s).health, 10);
    assert!(s.level_exit);
}

/// The suit: specials 5 and 7 do nothing; on 16 (and 4) the source draws
/// `P_Random() < 5` every tic on the floor, whatever the gate, and damages only
/// on the gate (p_spec.c:1020–1047).
#[test]
fn radiation_suit_on_the_damaging_floors() {
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    let sector = player_sector(&l);
    for special in [5, 7] {
        let mut l = l.clone();
        set_sector_special(&mut l, sector, special);
        let mut s = sim(&l);
        runtime_mut(&mut s).powers.as_mut().unwrap().powers[3] = 100;
        s.advance(zero()).unwrap();
        assert_eq!(runtime(&s).health, 100, "{special}");
        let mut s = sim(&l);
        s.advance(zero()).unwrap();
        assert!(runtime(&s).health < 100, "{special}");
    }
    let mut l16 = l.clone();
    set_sector_special(&mut l16, sector, 16);
    let mut bare = sim(&l16);
    let mut suited = sim(&l16);
    runtime_mut(&mut suited).powers.as_mut().unwrap().powers[3] = 100;
    let start = suited.rng.indices()[0];
    for _ in 0..40 {
        bare.advance(zero()).unwrap();
        suited.advance(zero()).unwrap();
        decode(&suited);
    }
    // The bare player is hit on the gates (tics 0 and 32) and draws only its two
    // pain chances; the suited one draws once every tic and, on this seed, is
    // never below 5 on a gate tic, so it is never hurt and draws nothing else.
    assert_eq!(runtime(&bare).health, 60);
    assert_eq!(runtime(&suited).health, 100);
    assert_eq!(bare.rng.indices()[0].wrapping_sub(start), 2);
    assert_eq!(suited.rng.indices()[0].wrapping_sub(start), 40);
    assert_eq!(runtime(&suited).powers.unwrap().powers[3], 60);
}

/// The firestick cycles its four `S_REDTORCH` states (4 tics each, no action);
/// the candle stays in its spawn state; both are refused under wire 22.
#[test]
fn firestick_cycles_and_candle_stays() {
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    thing(&mut l, -160, 0, 46);
    thing(&mut l, -160, 48, 34);
    let mut s = sim(&l);
    let find = |s: &Simulation, ty: identity::OriginalActorType| {
        s.actors
            .order()
            .find(|id| {
                s.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| l.actor_type.0 == ty.ordinal())
            })
            .unwrap()
    };
    let torch = find(&s, identity::OriginalActorType::MT_MISC43);
    let candle = find(&s, identity::OriginalActorType::MT_MISC49);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..20 {
        s.advance(zero()).unwrap();
        let l = s.actor(torch).unwrap().life.unwrap();
        seen.insert(l.state.unwrap().0);
        assert!((1..=4).contains(&l.tics));
        let c = s.actor(candle).unwrap().life.unwrap();
        assert_eq!(c.tics, -1);
        decode(&s);
    }
    assert_eq!(seen.len(), 4);
    assert_eq!(s.actor(torch).unwrap().solidity, Solidity::Solid);
}

/// The step-5 ruling: under wire 23 a non-shootable thing whose clipped
/// `floorz > ceilingz` (a mover closed over it, p_map.c:1290–1294) stays a legal
/// state; a shootable one keeps the refusal, and wire 22 refuses both.
#[test]
fn squashed_non_shootable_things_are_admitted_under_wire_23() {
    let squash = |s: &mut Simulation, ty: identity::OriginalActorType| {
        let id = s
            .actors
            .order()
            .find(|id| {
                s.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| l.actor_type.0 == ty.ordinal())
            })
            .unwrap();
        let a = s.actors.get_mut(id).unwrap();
        a.ceiling = a.floor - units(8);
    };
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    thing(&mut l, -160, 0, 2011);
    let mut s = sim(&l);
    squash(&mut s, identity::OriginalActorType::MT_MISC10);
    s.advance(zero()).unwrap();
    let mut s = try_sim(&l, LEVEL).unwrap();
    squash(&mut s, identity::OriginalActorType::MT_MISC10);
    assert_eq!(
        s.advance(zero()),
        Err(SimError::MapSpawn(MapSpawnError::Ownership))
    );
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    thing(&mut l, -160, 0, 2035);
    let mut s = sim(&l);
    squash(&mut s, identity::OriginalActorType::MT_BARREL);
    assert_eq!(
        s.advance(zero()),
        Err(SimError::MapSpawn(MapSpawnError::Ownership))
    );
}

// ---- M4a mutation-testing step (run 1, shard 1) ---------------------------

fn field(s: &Simulation, path: &str) -> Vec<u8> {
    s.snapshot()
        .fields()
        .find(|f| f.path() == path)
        .unwrap_or_else(|| panic!("no field {path}"))
        .value()
        .to_vec()
}
fn health(s: &Simulation) -> i32 {
    i32::from_le_bytes(field(s, "original_player.0.health").try_into().unwrap())
}
fn weapons(s: &Simulation) -> PlayerWeaponState {
    s.original_player_weapons(PlayerSlot::One).unwrap().unwrap()
}
fn loaded(l: &Lumps, gameplay: u8, inventory: WeaponInventory) -> Simulation {
    let options = MapContextOptions {
        initial_rng: DoomRng::from_indices(gameplay, 0),
        respawn_monsters: false,
    };
    let mut s = Simulation::original_player_episode1_map(l.map(), options).unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, inventory)
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}

/// `Episode1Bounds::countitems` counts the placed pickups whose info.c
/// definition carries `MF_COUNTITEM`: the soulsphere (info.c:2560,
/// `MF_SPECIAL|MF_COUNTITEM`) does; the blue keycard (2352,
/// `MF_SPECIAL|MF_NOTDMATCH`), the backpack (2976) and the radiation suit (2664,
/// `MF_SPECIAL` alone) do not.
#[test]
fn episode1_bounds_count_only_mf_countitem_pickups() {
    let record = |kind: i16| {
        let mut r = [0u8; 10];
        r[6..8].copy_from_slice(&kind.to_le_bytes());
        r[8..10].copy_from_slice(&7u16.to_le_bytes());
        r
    };
    let lump: Vec<u8> = [2013, 5, 8, 2025, 2013]
        .into_iter()
        .flat_map(record)
        .collect();
    let b = Episode1Bounds::from_thing_records(&lump).unwrap();
    assert_eq!((b.pickups, b.keycards, b.countitems), (5, 1, 2));
}

/// The suit does not stop the special-16 damage when the gate tic's draw is
/// below 5: `if (!player->powers[pw_ironfeet] || (P_Random()<5))`
/// (p_spec.c:1040–1045). Derivation of the draw from the initial gameplay index
/// 31: the load draws one `lastlook` per spawned thing (`P_SpawnMobj`,
/// p_mobj.c:524), the player and the suit (33). Tic 0 is a gate tic before the
/// touch (the body touches in its mobj thinker, after `P_PlayerThink`): the
/// bare player takes 20 and draws its pain chance (p_inter.c:888–893; 34).
/// From tic 1 the suited player on the floor draws once a tic, so gate tic 32
/// draws index 66, `rndtable[66]` = 4 (m_random.c), below 5: 20 more points
/// and a pain chance (67).
#[test]
fn suit_still_hurts_on_a_gate_draw_below_5() {
    let mut l = with_items(&[2025]);
    let sector = player_sector(&l);
    set_sector_special(&mut l, sector, 16);
    let mut s = loaded(&l, 31, WeaponInventory::default());
    assert_eq!(s.rng().indices()[0], 33);
    s.advance(step()).unwrap();
    assert_eq!(health(&s), 80);
    assert_eq!(powers(&s).powers[3], 2100);
    assert_eq!(s.rng().indices()[0], 34);
    for _ in 1..32 {
        s.advance(zero()).unwrap();
    }
    assert_eq!((health(&s), s.rng().indices()[0]), (80, 65));
    s.advance(zero()).unwrap();
    assert_eq!((health(&s), s.rng().indices()[0]), (60, 67));
    decode(&s);
}

/// `P_GiveAmmo` switches to the launcher on a rocket pickup only when the
/// rockets were 0 (p_inter.c:104–114), the fist is ready and the launcher is
/// owned (149–154): with the pistol ready instead, nothing is pending. The
/// launcher's two rockets are fired at the far wall first; then the weapon is
/// selected (key 1, no chainsaw: the fist; key 2: the pistol) and the player
/// walks onto a rocket.
#[test]
fn rocket_pickup_switches_from_the_fist_only() {
    for (key, ready, pending) in [
        (0, Weapon::Fist, Some(Weapon::Rocket)),
        (1, Weapon::Pistol, None),
    ] {
        let mut l = room();
        l.add_thing([-32, 0, 180, 1, 7]);
        thing(&mut l, -40, 0, 2003);
        thing(&mut l, -200, 0, 2010);
        let inventory = WeaponInventory {
            ready: Weapon::Fist,
            owned: 3,
            ammo: [0; 4],
        };
        let mut s = loaded(&l, 6, inventory);
        let fire = |attack: bool| {
            let mut c = zero();
            c[0].buttons = if attack { weapons::BT_ATTACK } else { 0 };
            c
        };
        s.advance(step()).unwrap();
        assert_eq!((weapons(&s).owned, weapons(&s).rockets), (3 | 1 << 4, 2));
        for tic in 0..300 {
            if weapons(&s).rockets == 0 {
                break;
            }
            s.advance(fire(tic % 2 == 0)).unwrap();
        }
        assert_eq!(weapons(&s).rockets, 0);
        let mut change = zero();
        change[0].buttons = weapons::BT_CHANGE | key << 3;
        for _ in 0..100 {
            let w = weapons(&s);
            if w.ready == ready && w.pending.is_none() {
                break;
            }
            let c = if w.pending.is_none() && w.ready != ready {
                change
            } else {
                zero()
            };
            s.advance(c).unwrap();
        }
        assert_eq!(weapons(&s).ready, ready);
        let mut walked = false;
        let mut walk = zero();
        walk[0].forward = 25;
        for _ in 0..200 {
            s.advance(walk).unwrap();
            if weapons(&s).rockets == 1 {
                walked = true;
                break;
            }
        }
        assert!(walked);
        assert_eq!(weapons(&s).pending, pending, "{ready:?}");
        assert_eq!(health(&s), 100);
        decode(&s);
    }
}

/// The wire-23 `bonuscount` bound is the map's own `6 * (P + R + C)`
/// (docs/COMPATIBILITY.md, wire 23 "Bounds"), not wire 22's 384: 65 health
/// bonuses overlapping the player are all touched in one step, each adding
/// `BONUSADD` 6 (p_inter.c:51, 658), so `bonuscount` is 390, exactly the
/// map's bound, and the next tic still runs.
#[test]
fn bonuscount_may_reach_the_maps_own_bound() {
    let mut s = sim(&with_items(&[2014; 65]));
    assert_eq!(s.episode1_bounds().unwrap().max_bonus(), 390);
    s.advance(step()).unwrap();
    assert_eq!(pickups(&s).bonus_count, 390);
    assert_eq!(health(&s), 165);
    s.advance(zero()).unwrap();
    assert_eq!(pickups(&s).bonus_count, 389);
    decode(&s);
}

// ---- M4a mutation-testing step (run 1, shard 1): the tic-preflight witnesses
//
// Rust-only safety tests, in the manner of the E1M1 decoration witness test
// (player_e1m1_tests.rs): each corrupts one field of an otherwise valid wire-23
// state from inside the crate (no public operation produces these values) and
// checks that the witness refuses it. The domains are the written ones cited on
// each test; a witness that accepted these values would let a corrupt state run.

fn of_doomed(kind: i32) -> ActorTypeId {
    ActorTypeId(
        identity::OriginalActorType::all()
            .find(|t| t.descriptor().doomed_number == kind)
            .unwrap()
            .ordinal(),
    )
}
fn first_of(s: &Simulation, ty: ActorTypeId) -> ActorId {
    s.actor_order()
        .find(|id| {
            s.actor(*id)
                .and_then(Actor::life)
                .is_some_and(|l| l.actor_type == ty)
        })
        .unwrap()
}

/// The power counters' Episode 1 domains (`validate_powers`' contract, from the
/// source durations, doomdef.h:234–238 and M4A_REFERENCE_INVESTIGATION.md
/// lines 988–1029): no invulnerability or berserk in Episode 1 single player
/// (`powers[0]`, `powers[1]` = 0), invisibility and the suit at most 60·35,
/// `pw_allmap` 0 or 1, infrared at most 120·35. Each counter one past its
/// domain is refused alone; the domain bounds themselves are accepted.
#[test]
fn power_counters_outside_their_domains_are_refused_one_at_a_time() {
    let s = sim(&with_items(&[]));
    let valid = powers(&s);
    assert_eq!(episode1::validate_powers(&valid), Ok(()));
    let mut top = valid;
    top.powers = [0, 0, 60 * 35, 60 * 35, 1, 120 * 35];
    assert_eq!(episode1::validate_powers(&top), Ok(()));
    for (i, bad) in [
        (0, 1),
        (1, 1),
        (2, 60 * 35 + 1),
        (3, 60 * 35 + 1),
        (4, 2),
        (5, 120 * 35 + 1),
    ] {
        let mut p = valid;
        p.powers[i] = bad;
        assert_eq!(
            episode1::validate_powers(&p),
            Err(SimError::PlayerBody(PlayerBodyError::Invariant)),
            "powers[{i}] = {bad}"
        );
    }
}

/// STATE_FORMAT.md (v27, the bonus slice the later wires keep): `item_count` is
/// `0..total` and a nonzero `level.total_items` equals the placed bonuses; the
/// live COUNTITEM bonuses are the ones not yet collected. With two health
/// bonuses and a clip placed (so the map's pickup bound is 3) and none taken,
/// each corruption is refused by the tic preflight: `item_count` above the
/// total, one collected while both still stand, and a total of 3, inside the
/// pickup bound but not the placed bonus count.
#[test]
fn item_statistics_outside_the_written_domain_are_refused() {
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    thing(&mut l, 200, 64, 2014);
    thing(&mut l, 200, -64, 2014);
    thing(&mut l, 200, 0, 2007);
    let s = sim(&l);
    assert_eq!(s.statistics.unwrap().total_items, 2);
    assert_eq!(s.preflight_world_pickups(), Ok(()));
    let invariant = Err(SimError::PlayerBody(PlayerBodyError::Invariant));
    for collected in [3, 1] {
        let mut c = s.clone();
        runtime_mut(&mut c).pickups.as_mut().unwrap().item_count = collected;
        assert_eq!(
            c.preflight_world_pickups(),
            invariant,
            "item_count {collected}"
        );
    }
    let mut c = s.clone();
    c.statistics.as_mut().unwrap().total_items = 3;
    assert_eq!(c.preflight_world_pickups(), invariant, "total 3");
}

/// `bonuscount` is never negative: `P_PlayerThink` decrements it only while it
/// is nonzero (p_user.c:358–359) and pickups add `BONUSADD` (p_inter.c:658). A
/// negative count is refused by the pickup-state witness of the tic preflight.
#[test]
fn a_negative_bonus_count_is_refused() {
    let mut s = sim(&with_items(&[]));
    assert_eq!(s.preflight_world_pickups(), Ok(()));
    runtime_mut(&mut s).pickups.as_mut().unwrap().bonus_count = -1;
    assert_eq!(
        s.preflight_world_pickups(),
        Err(SimError::PlayerBody(PlayerBodyError::Invariant))
    );
}

/// `GOTMEDINEED` is never assigned: `P_GiveBody (player, 25)` succeeds only
/// below 100 health and then leaves at least 26, so `player->health < 25` is
/// never true after it (p_inter.c:476–483). A stored `MedikitNeeded` message is
/// refused by the pickup-state witness of the tic preflight.
#[test]
fn a_stored_medikit_needed_message_is_refused() {
    let mut s = sim(&with_items(&[]));
    runtime_mut(&mut s).pickups.as_mut().unwrap().message =
        world_pickups::PickupMessage::MedikitNeeded;
    assert_eq!(
        s.preflight_world_pickups(),
        Err(SimError::PlayerBody(PlayerBodyError::Invariant))
    );
}

/// The item witness (STATE_FORMAT.md v27: an item record validates its current
/// state within its own loop): a cycling item (the blue keycard, info.c:964–965 `S_BKEY`
/// / `S_BKEY2`) accepts its two loop states and refuses a state outside it.
#[test]
fn a_cycling_item_accepts_only_its_own_loop_states() {
    use identity::OriginalState;
    let card = of_doomed(5);
    assert!(world_pickups::state_allowed(card, OriginalState::S_BKEY));
    assert!(world_pickups::state_allowed(card, OriginalState::S_BKEY2));
    for other in [
        OriginalState::S_PLAY,
        OriginalState::S_YKEY,
        OriginalState::S_NULL,
    ] {
        assert!(!world_pickups::state_allowed(card, other), "{other:?}");
    }
}

/// The decoration witness (the E1M1 decoration witness test's rule: sprite and
/// frame are written together with the state by state entry from info.c): a
/// candle (DoomEd 34, `S_CANDLESTIK`) whose sprite alone, or frame alone,
/// disagrees with its state is refused.
#[test]
fn a_decoration_whose_sprite_or_frame_disagrees_with_its_state_is_refused() {
    let mut l = with_items(&[]);
    thing(&mut l, 200, 0, 34);
    let s = sim(&l);
    let candle = *s.actor(first_of(&s, of_doomed(34))).unwrap();
    assert_eq!(e1m1_rules::validate_decoration(None, candle), Ok(()));
    let mut sprite = candle;
    sprite.life.as_mut().unwrap().sprite ^= 1;
    let mut frame = candle;
    frame.life.as_mut().unwrap().frame ^= 1;
    for (name, bad) in [("sprite", sprite), ("frame", frame)] {
        assert_eq!(
            e1m1_rules::validate_decoration(None, bad),
            Err(SimError::PlayerBody(PlayerBodyError::Invariant)),
            "{name}"
        );
    }
}

/// `MF_JUSTATTACKED` is set only by `A_Saw` (p_pspr.c:542) and cleared by the
/// next `P_PlayerThink` (p_user.c:249–254), so at a tic boundary under wire 23
/// it is admitted only with the chainsaw ready (the body preflight's written
/// exception); with the pistol ready it is refused. The flag with the chainsaw
/// ready is admitted.
#[test]
fn just_attacked_at_a_tic_boundary_needs_the_chainsaw_ready() {
    let mut s = sim(&with_items(&[]));
    assert_eq!(s.preflight_player_body(false), Ok(()));
    let mo = runtime(&s).mo.unwrap();
    s.actors.get_mut(mo).unwrap().life.as_mut().unwrap().flags |= player_commands::MF_JUSTATTACKED;
    assert_eq!(
        s.preflight_player_body(false),
        Err(SimError::PlayerBody(PlayerBodyError::Invariant))
    );
    let Some(player_weapons::PlayerWeaponRuntime::Pack(w)) = runtime_mut(&mut s).weapon.as_mut()
    else {
        panic!("pack");
    };
    w.ready = Weapon::Chainsaw;
    w.owned |= 1 << 7;
    assert_eq!(s.preflight_player_body(false), Ok(()));
}

/// The body preflight's airborne-mover rule (original_player.rs: a flying
/// missile is the one admitted non-monster, non-barrel mover with momentum at a
/// tic boundary): a health bonus given momentum is refused as `Unsupported`
/// under wire 23, which also admits imps.
#[test]
fn a_moving_item_at_a_tic_boundary_is_refused() {
    let mut l = with_items(&[]);
    thing(&mut l, 200, 0, 2014);
    let mut s = sim(&l);
    assert_eq!(s.preflight_player_body(false), Ok(()));
    let bonus = first_of(&s, of_doomed(2014));
    s.actors.get_mut(bonus).unwrap().spec.momentum[0] = units(1);
    assert_eq!(
        s.preflight_player_body(false),
        Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
    );
}

/// STATE_FORMAT.md (v27): "zero also describes a constructed world before its
/// THING batch". A wire-23 world with one placed health bonus, constructed but
/// not yet spawned, has `level.total_items` 0 and passes the item-statistics
/// witness; after the batch the total is the placed count, 1.
#[test]
fn a_zero_item_total_before_the_thing_batch_is_accepted() {
    let mut l = room();
    thing(&mut l, -32, 0, 1);
    thing(&mut l, 200, 64, 2014);
    let s = load(&l, EPISODE1).unwrap();
    assert_eq!(s.statistics.unwrap().total_items, 0);
    assert_eq!(s.preflight_world_pickups(), Ok(()));
    let s = sim(&l);
    assert_eq!(s.statistics.unwrap().total_items, 1);
    assert_eq!(s.preflight_world_pickups(), Ok(()));
}
