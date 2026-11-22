//! M6a wire 26 (`PlayerMonstersEpisode1V4`, DSIM42), step 4: unit tests of what wire
//! 26 adds, each from a cited C fact (M6a note §1.1, §1.2, §4.3). Rust-only tests;
//! the genuine comparison of carried starts is `tests/wire26_chain.rs`.
use crate::barrels_replay::{cases, room};
use crate::carried_start::{CarriedField, LevelStartError, MAX_CARRIED_FLASH_TICS};
use crate::diagonal_room::Lumps;
use crate::sound::{SoundOrigin, SoundRecord};
use crate::*;

fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn base() -> Lumps {
    let c = &cases()[0];
    let mut l = room(c.map);
    let t = &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1;
    t.splice(0..0, record(c.position[0], c.position[1], 0, 1, 7));
    l
}
fn level(start: LevelStart) -> Result<Simulation, LevelStartError> {
    Simulation::original_player_episode1_v4_level(
        base().map(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
        start,
    )
}
/// A carried player the C can hold: a chainsaw ready, the backpack, blue armour.
fn carried() -> CarriedStart {
    CarriedStart {
        health: 150,
        armor: PlayerArmorState {
            kind: ArmorType::Blue,
            points: 200,
        },
        weapons_owned: 0b1000_1111,
        ready_weapon: Weapon::Chainsaw,
        ammo: [400, 7, 0, 3],
        max_ammo: [400, 100, 600, 100],
        backpack: true,
        cheats: 0,
        attack_down: false,
        use_down: true,
        bob: Fixed::from_bits(0x8000),
        weapon_x: Fixed::from_bits(3 * 65536),
        flash_tics: 4,
        flash_position: [Fixed::from_bits(65536), Fixed::from_bits(40 * 65536)],
        p_random: 77,
    }
}
fn step(s: &mut Simulation) -> Result<(), SimError> {
    s.advance([TicCommand::default(); 4])
}

/// A new game under wire 26 is wire 25's start: the same state fields and records
/// (the projection over whole demos is `tests/wire26_projection.rs`), start kind 0,
/// index 0 and `G_PlayerReborn`'s player (g_game.c:815-831).
#[test]
fn a_new_game_is_wire_25s_start() {
    let s = level(LevelStart::NewGame).unwrap();
    let mut v25 = Simulation::original_player_episode1_v3_map(
        base().map(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .unwrap();
    v25.spawn_map_things().unwrap();
    v25.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    v25.spawn_level_specials().unwrap();
    assert_eq!(s.level_start_kind(), Some(StartKind::NewGame));
    assert_eq!(s.rng(), v25.rng());
    let strip = |s: &Simulation| -> Vec<(String, Vec<u8>)> {
        s.snapshot()
            .fields()
            .filter(|f| {
                !f.path().starts_with("level.start.")
                    && f.path() != "profile"
                    && f.path() != "context.schema"
            })
            .map(|f| (f.path().to_owned(), f.value().to_vec()))
            .collect()
    };
    assert_eq!(strip(&s), strip(&v25));
    assert_eq!(s.pending_events(), v25.pending_events());
    assert_eq!(
        s.level_start_player(),
        Some(carried_start::reborn_player(0))
    );
    assert!(s.load_sound_records().is_empty());
    assert_eq!(s.snapshot().version(), 42);
    let w = s.original_player_weapons(PlayerSlot::One).unwrap().unwrap();
    assert_eq!(
        (w.ready, w.owned, w.bullets, w.attack_down),
        (Weapon::Pistol, 3, 50, true)
    );
}

/// `P_SetupLevel` does not reset `prndindex` (only `G_InitNew`'s `M_ClearRandom`
/// does, g_game.c:1414, m_random.c:69-72): a reborn start draws its spawns from the
/// carried index, so it ends the load at the new game's draws past that index.
#[test]
fn a_reborn_start_keeps_the_index() {
    let fresh = level(LevelStart::NewGame).unwrap();
    let draws = fresh.rng().indices()[0];
    let s = level(LevelStart::Reborn { p_random: 200 }).unwrap();
    assert_eq!(s.level_start_kind(), Some(StartKind::Reborn));
    assert_eq!(s.rng().indices()[0], 200_u8.wrapping_add(draws));
    assert_eq!(
        s.level_start_player(),
        Some(carried_start::reborn_player(200))
    );
    assert_eq!(s.rng().indices()[1], 0);
}

/// The carried player is set up as `P_SpawnPlayer` and `P_SetupPsprites` leave it
/// (p_mobj.c:642-700, p_pspr.c:138-154, 831-842): the mobj's health is the player's
/// (673), keys and powers clear, the pending weapon cancelled, the ready weapon in
/// its up state at `WEAPONBOTTOM - RAISESPEED`, the weapon psprite's x kept, the
/// flash null with its count and coordinates kept, `bob` kept, the latches the
/// carried ones; a carried chainsaw starts `sfx_sawup` at the player's body
/// (p_pspr.c:145-146), unplaced (s_sound.c:304), recorded in the start group and
/// published first with tic 0's records.
#[test]
fn a_carried_start_sets_up_what_the_c_carries() {
    let c = carried();
    let mut s = level(LevelStart::Carried(c)).unwrap();
    let p = s.original_player(PlayerSlot::One).unwrap();
    let mo = p.mo.unwrap();
    assert_eq!(p.health, 150);
    assert_eq!(s.actor(mo).unwrap().life().unwrap().health, 150);
    assert_eq!(s.original_player_armor(PlayerSlot::One).unwrap(), c.armor);
    assert_eq!(
        s.original_player_powers(PlayerSlot::One).unwrap(),
        PlayerPowers::default()
    );
    let w = s.original_player_weapons(PlayerSlot::One).unwrap().unwrap();
    assert_eq!(w.ready, Weapon::Chainsaw);
    assert_eq!(w.pending, None);
    assert_eq!((w.bullets, w.shells, w.cells, w.rockets), (400, 7, 0, 3));
    assert!(w.backpack && !w.attack_down);
    assert_eq!(w.bob, c.bob);
    assert_eq!(
        w.psprites[0].position,
        [c.weapon_x, Fixed::from_map_units(122)]
    );
    assert_eq!(
        w.psprites[0].state.map(|x| x.0),
        Some(identity::OriginalState::S_SAWUP.ordinal())
    );
    assert_eq!(w.psprites[1].state, None);
    assert_eq!(w.psprites[1].tics, 4);
    assert_eq!(w.psprites[1].position, c.flash_position);
    let saw = SoundRecord::Start {
        sfx: lifecycle::source_sound_id("sfx_sawup"),
        origin: SoundOrigin::Actor(mo),
        at: None,
    };
    assert_eq!(s.load_sound_records(), &[saw]);
    assert_eq!(s.pending_sound_records(), &[saw]);
    let finish = s.level_finish().unwrap();
    assert!(finish.carried.unwrap().use_down);
    step(&mut s).unwrap();
    assert_eq!(s.sound_records().first(), Some(&saw));
    assert_eq!(s.load_sound_records(), &[saw]);
    for _ in 0..40 {
        step(&mut s).unwrap();
    }
    // The null flash keeps its count until a flash state sets it (p_pspr.c:851-883).
    let w = s.original_player_weapons(PlayerSlot::One).unwrap().unwrap();
    assert_eq!((w.psprites[1].state, w.psprites[1].tics), (None, 4));
}

/// Values the C can reach are admitted (health and armour at 200, green armour
/// above 100 from bonuses, p_inter.c:385-396; a null flash at the largest flash
/// count); each value it cannot hold, or the simulation does not model, is refused
/// with its field and nothing is constructed.
#[test]
fn carried_values_the_c_cannot_hold_are_refused() {
    let ok = |c: CarriedStart| level(LevelStart::Carried(c)).map(|_| ());
    let mut c = carried();
    c.health = 200;
    c.armor = PlayerArmorState {
        kind: ArmorType::Green,
        points: 200,
    };
    c.flash_tics = MAX_CARRIED_FLASH_TICS;
    assert_eq!(ok(c), Ok(()));
    type Change = fn(&mut CarriedStart);
    let refused: [(Change, CarriedField); 16] = [
        (|c| c.health = 0, CarriedField::Health),
        (|c| c.health = 201, CarriedField::Health),
        (|c| c.armor.kind = ArmorType::None, CarriedField::Armor),
        (|c| c.armor.points = 0, CarriedField::Armor),
        (|c| c.armor.points = 201, CarriedField::Armor),
        (|c| c.weapons_owned = 0b1000_1101, CarriedField::Weapons),
        (|c| c.ready_weapon = Weapon::Plasma, CarriedField::Weapons),
        (|c| c.weapons_owned = 0b0000_1111, CarriedField::Weapons),
        (|c| c.ammo[1] = 101, CarriedField::Ammo),
        (|c| c.ammo[0] = -1, CarriedField::Ammo),
        (|c| c.max_ammo = [200, 50, 300, 50], CarriedField::MaxAmmo),
        (|c| c.cheats = 2, CarriedField::Cheats),
        (|c| c.bob = Fixed::from_bits(0x10_0001), CarriedField::Bob),
        (|c| c.flash_tics = 8, CarriedField::Psprites),
        (
            |c| c.weapon_x = Fixed::from_map_units(18),
            CarriedField::Psprites,
        ),
        (
            |c| c.flash_position[1] = Fixed::from_map_units(129),
            CarriedField::Psprites,
        ),
    ];
    for (change, field) in refused {
        let mut c = carried();
        change(&mut c);
        assert_eq!(ok(c), Err(LevelStartError::Carried(field)), "{field:?}");
    }
    let context = MapContextOptions {
        initial_rng: doom_types::DoomRng::from_indices(1, 0),
        ..MapContextOptions::default()
    };
    let r = Simulation::original_player_episode1_v4_level(
        base().map(),
        1,
        Skill::Medium,
        context,
        LevelStart::NewGame,
    );
    assert_eq!(r.err(), Some(LevelStartError::Context));
}

/// `LevelFinish` reads, never clears: the exit request and the reborn request are
/// both reported (the session applies the C order, reborn first, g_game.c:611-623),
/// a dead player carries nothing, and the counts and totals are the level's. Wire 24
/// has no finish (the session runs wires 25 and 26).
#[test]
fn level_finish_reports_both_requests_and_reads_only() {
    let mut s = level(LevelStart::Reborn { p_random: 9 }).unwrap();
    step(&mut s).unwrap();
    let before = s.snapshot().canonical_bytes();
    let f = s.level_finish().unwrap();
    assert_eq!((f.exit, f.reborn, f.player_dead), (None, false, false));
    assert_eq!(f.level_time, 1);
    assert_eq!(f.map, 1);
    assert_eq!(f.p_random, s.rng().indices()[0]);
    assert_eq!(f.carried.unwrap().p_random, f.p_random);
    assert_eq!(s.snapshot().canonical_bytes(), before);
    s.level_exit = true;
    s.level_secret_exit = true;
    s.original_player.as_mut().unwrap().reborn = true;
    s.original_player.as_mut().unwrap().state = PlayerBodyState::Dead;
    let f = s.level_finish().unwrap();
    assert_eq!(f.exit, Some(LevelExit::Secret));
    assert!(f.reborn && f.player_dead && f.carried.is_none());
    assert_eq!(step(&mut s), Err(SimError::LevelExited));
    let v24 = Simulation::original_player_episode1_v2_map(
        base().map(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .unwrap();
    assert_eq!(v24.level_finish().err(), Some(PlayerBodyError::Profile));
}

fn fields(s: &Simulation) -> Vec<(String, Vec<u8>)> {
    s.snapshot()
        .fields()
        .map(|f| (f.path().to_owned(), f.value().to_vec()))
        .collect()
}
fn encode(version: u32, fields: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"DSIM".to_vec();
    out.extend(version.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (p, v) in fields {
        out.extend((p.len() as u32).to_le_bytes());
        out.extend(p.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    }
    out
}
fn set(fields: &mut [(String, Vec<u8>)], path: &str, value: Vec<u8>) {
    fields.iter_mut().find(|(p, _)| p == path).unwrap().1 = value;
}

/// DSIM42 is DSIM41 plus the start group after `events.sounds.pending`, read by
/// `decode_episode1_v4` only; every earlier reader rejects 42, and the v42 reader
/// rejects 41. The reader refuses a start group the writer never produces.
#[test]
fn dsim42_reader_round_trip_and_refusals() {
    let limits = DiagnosticLimits::default();
    let s = level(LevelStart::Carried(carried())).unwrap();
    let f = fields(&s);
    let names: Vec<&str> = f.iter().map(|(p, _)| p.as_str()).collect();
    let first = names
        .iter()
        .position(|p| p.starts_with("level.start."))
        .unwrap();
    assert_eq!(names[first - 1], "events.sounds.pending.0");
    assert_eq!(names[first], "level.start.kind");
    assert_eq!(names[first + 25], "level.start.sounds.0");
    let bytes = encode(42, &f);
    assert_eq!(bytes, s.snapshot().canonical_bytes());
    assert_eq!(
        Diagnostic::decode_episode1_v4(&bytes, limits)
            .unwrap()
            .version(),
        42
    );
    for read in [
        Diagnostic::decode_episode1_v3,
        Diagnostic::decode_episode1_v2,
        Diagnostic::decode_episode1,
    ] {
        assert_eq!(
            read(&bytes, limits).err(),
            Some(DiagnosticError::Version(42))
        );
    }
    let v41 = encode(41, &f);
    assert_eq!(
        Diagnostic::decode_episode1_v4(&v41, limits).err(),
        Some(DiagnosticError::Version(41))
    );
    let read =
        |f: &[(String, Vec<u8>)]| Diagnostic::decode_episode1_v4(&encode(42, f), limits).err();
    let mut x = f.clone();
    set(&mut x, "level.start.kind", vec![3]);
    assert_eq!(read(&x), Some(DiagnosticError::Schema));
    let mut x = f.clone();
    set(&mut x, "level.start.cheats", 1_i32.to_le_bytes().to_vec());
    assert_eq!(read(&x), Some(DiagnosticError::Schema));
    let mut x = f.clone();
    set(
        &mut x,
        "level.start.sounds.count",
        0_u64.to_le_bytes().to_vec(),
    );
    x.retain(|(p, _)| p != "level.start.sounds.0");
    assert_eq!(read(&x), Some(DiagnosticError::Structure));
    let mut x = f.clone();
    x.push(("level.start.zzz".into(), vec![0]));
    assert_eq!(read(&x), Some(DiagnosticError::Structure));
    let mut x = f.clone();
    let mut wrong = x
        .iter()
        .find(|(p, _)| p == "level.start.sounds.0")
        .unwrap()
        .1
        .clone();
    wrong[1] ^= 1;
    set(&mut x, "level.start.sounds.0", wrong);
    assert_eq!(read(&x), Some(DiagnosticError::Identity));
    let mut x = f;
    x.retain(|(p, _)| p != "level.start.flash_y");
    assert_eq!(read(&x), Some(DiagnosticError::Structure));
    // A new game must hold index 0 and the fresh player; a reborn start any index.
    let mut g = fields(&level(LevelStart::NewGame).unwrap());
    assert_eq!(read(&g), None);
    set(&mut g, "level.start.p_random", vec![5]);
    assert_eq!(read(&g), Some(DiagnosticError::Schema));
    set(&mut g, "level.start.kind", vec![2]);
    assert_eq!(read(&g), None);
    set(&mut g, "level.start.health", 99_i32.to_le_bytes().to_vec());
    assert_eq!(read(&g), Some(DiagnosticError::Schema));
}
