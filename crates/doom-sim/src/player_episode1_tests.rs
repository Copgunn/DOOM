//! M4a Episode 1 policy, plan step 3 (wire 23, DSIM39): its identity, the per-map
//! bounds of decision H4, the load admission of decision H3 (the proofs that refuse
//! Episode 1 maps become per-tic witnesses or are dropped), the DSIM39 contract and
//! its reader, and that every wire-22 workload runs identically under wire 23.
//! Rust-only policy tests: the expected values come from the source reading cited
//! in the plan (p_mobj.c:708–797, p_switch.c:201–263, p_spec.c:1163–1221,
//! p_maputl.c `P_InterceptVector`), never from the C engine.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::*;

const EPISODE1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V1;
const LEVEL: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersLevelV1;

fn lump<'a>(l: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut l.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn with_start(mut l: Lumps, at: usize, position: [i16; 2], angle: i16) -> Lumps {
    let t = lump(&mut l, "THINGS");
    let at = at.min(t.len() / 10);
    t.splice(
        at * 10..at * 10,
        record(position[0], position[1], angle, 1, 7),
    );
    l
}
fn options(seed: u8) -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(seed, 0),
        respawn_monsters: false,
    }
}
fn setup(policy: OriginalExecutionProfile, map: doom_map::Map, seed: u8) -> Simulation {
    let mut s = if policy == EPISODE1 {
        Simulation::original_player_episode1_map(map, options(seed))
    } else {
        Simulation::original_player_level_map(map, options(seed))
    }
    .unwrap();
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}
fn admitted(lumps: &Lumps, policy: OriginalExecutionProfile) -> Result<Simulation, SimError> {
    let result = if policy == EPISODE1 {
        Simulation::original_player_episode1_map(lumps.map(), options(6))
    } else {
        Simulation::original_player_level_map(lumps.map(), options(6))
    };
    result.map_err(|e| match e {
        OriginalRegistryError::Runtime(e) => e,
        other => panic!("{other:?}"),
    })
}
fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}

/// Whether `path` is one of the DSIM39 additions of plan §A.6 (the plat `type`
/// is matched separately: doors and floors carry a `type` in DSIM38 too).
fn new_in_39(path: &str) -> bool {
    path == "level.secret_exit"
        || path == "original_player.0.backpack"
        || path.starts_with("original_player.0.powers.")
        || path.starts_with("original_player.0.cards.")
        || path.starts_with("original_player.0.weapons.max_ammo.")
        || matches!(
            path,
            "original_player.0.weapons.cells" | "original_player.0.weapons.rockets"
        )
        || (path.starts_with("sector.") && path.ends_with(".floorpic"))
        || (path.starts_with("sector_thinker.")
            && (path.ends_with(".direction") || path.ends_with(".speed")))
}
/// The capture fields as (path, value) pairs, in emission order.
fn fields(s: &Simulation) -> Vec<(String, Vec<u8>)> {
    s.snapshot()
        .fields()
        .map(|f| (f.path().to_string(), f.value().to_vec()))
        .collect()
}
/// Asserts that the wire-23 capture is the wire-22 capture with the DSIM39 fields
/// inserted and the policy labels (`profile`, `context.schema`,
/// `store.max_slots`) changed, in the same order.
fn same_but_labels(w22: &[(String, Vec<u8>)], w23: &[(String, Vec<u8>)]) {
    let old: std::collections::BTreeSet<&str> = w22.iter().map(|(p, _)| p.as_str()).collect();
    let mut kept = Vec::new();
    for (path, value) in w23 {
        if old.contains(path.as_str()) {
            kept.push((path.clone(), value.clone()));
        } else {
            let plat_type = path.starts_with("sector_thinker.") && path.ends_with(".type");
            assert!(
                new_in_39(path) || plat_type,
                "unexpected DSIM39 field {path}"
            );
        }
    }
    assert_eq!(kept.len(), w22.len());
    for ((p22, v22), (p23, v23)) in w22.iter().zip(&kept) {
        assert_eq!(p22, p23);
        match p22.as_str() {
            "profile" => {
                assert_eq!(v22, &22u32.to_le_bytes());
                assert_eq!(v23, &23u32.to_le_bytes());
            }
            "context.schema" => assert_eq!(v23, PLAYER_EPISODE1_CONTEXT_SCHEMA),
            "store.max_slots" => {
                assert_eq!(v22, &384u32.to_le_bytes());
                assert_eq!(v23, &1024u32.to_le_bytes());
            }
            _ => assert_eq!(v22, v23, "{p22}"),
        }
    }
}

#[test]
fn episode1_policy_identity_is_wire_23_with_its_own_context() {
    assert_eq!(EPISODE1.wire_id(), 23);
    assert_eq!(LEVEL.wire_id(), 22);
    assert_eq!(EPISODE1.context_schema(), PLAYER_EPISODE1_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_EPISODE1_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-EPISODE1-1-MEDIUM-v1"
    );
    assert_ne!(EPISODE1.context_schema(), LEVEL.context_schema());
    // A strict superset of the level policy; `has_episode1` is wire 23 alone.
    assert!(EPISODE1.has_episode1() && EPISODE1.has_level() && EPISODE1.has_barrels());
    assert!(EPISODE1.has_e1m1() && EPISODE1.has_movers() && EPISODE1.has_world_pickups());
    assert!(EPISODE1.has_player_weapons() && EPISODE1.has_player_commands());
    assert!(!LEVEL.has_episode1());
    assert_eq!(
        player_map_scale::actor_slots(EPISODE1),
        EPISODE1_ACTOR_SLOTS
    );
    assert_eq!(EPISODE1_ACTOR_SLOTS, 1024);
    assert_eq!(player_map_scale::actor_slots(LEVEL), BARRELS_ACTOR_SLOTS);
    // Decision D10: no per-type live missile cap; the store bounds them.
    assert_eq!(player_map_scale::live_projectile_bound(EPISODE1), 1024);
    assert_eq!(
        player_map_scale::live_projectile_bound(LEVEL),
        MAP_SCALE_MAX_LIVE_PROJECTILES
    );
    OriginalExecutionRegistry::new(EPISODE1).unwrap();
    // The constructor demands exactly the policy's slot count.
    let c = &cases()[0];
    let map = with_start(room(c.map), 0, c.position, 0).map();
    assert_eq!(
        Simulation::with_original_registry(
            map,
            BARRELS_ACTOR_SLOTS,
            OriginalExecutionRegistry::new(EPISODE1).unwrap(),
            ActorOptions::default(),
            options(0),
        )
        .err(),
        Some(SimError::Store(StoreError::InvalidLimit))
    );
}

/// The `P_SpawnMapThing` filter (p_mobj.c:708–797): starts (1–4) and deathmatch
/// starts (11) are not things, `MTF_NOTSINGLE` (16) and a missing medium bit (2)
/// skip the record; the rest is classified by DoomEd number.
#[test]
fn episode1_bounds_count_the_medium_single_player_things() {
    let mut lump = Vec::new();
    for (kind, options) in [
        (1, 7),
        (2, 7),
        (11, 7),
        (3004, 7),     // monster, dropper
        (9, 2),        // monster, dropper
        (65, 6 | 16),  // not in single player
        (3001, 1 | 4), // not at medium
        (3002, 7),     // monster
        (58, 7),       // spectre: monster
        (5, 7),        // blue keycard: pickup, keycard
        (2014, 7),     // health bonus: pickup, COUNTITEM
        (2013, 7),     // soulsphere: pickup, COUNTITEM
        (2007, 7),     // clip: pickup
        (2035, 7),     // barrel
        (34, 7),       // candle: static
        (2028, 7),     // floor lamp (E1M1 decoration): static
        (9999, 7),     // unknown: counted nowhere (refused by the spawn batch)
    ] {
        lump.extend(record(0, 0, 0, kind, options));
    }
    let b = Episode1Bounds::from_thing_records(&lump).unwrap();
    assert_eq!(
        b,
        Episode1Bounds {
            monsters: 4,
            droppers: 2,
            pickups: 4,
            keycards: 1,
            barrels: 1,
            statics: 2,
            countitems: 2,
        }
    );
    assert_eq!(b.world_items(), 6);
    assert_eq!(b.max_bonus(), 6 * (4 + 2 + 1));
    // Never below the older policies' constants (64, 216, 8) ...
    assert_eq!(b.sight_queries(), 64);
    assert_eq!(b.explosion_sight_queries(), EXPLOSION_SIGHT_QUERIES);
    assert_eq!(b.visit_roster(), MAX_PLAYER_MONSTERS);
    // ... and the per-map formulas above them.
    let big = Episode1Bounds {
        monsters: 100,
        barrels: 20,
        ..b
    };
    assert_eq!(big.sight_queries(), 8 * 101);
    assert_eq!(big.explosion_sight_queries(), 121 * 101);
    assert_eq!(big.visit_roster(), 100);
    assert_eq!(Episode1Bounds::from_thing_records(&lump[..15]), None);
    // A loaded map gives the same counts as its raw lump.
    for c in cases() {
        let mut l = room(c.map);
        let raw = lump_bytes(&mut l);
        assert_eq!(
            Episode1Bounds::from_map(&l.map()),
            Episode1Bounds::from_thing_records(&raw).unwrap()
        );
    }
}
fn lump_bytes(l: &mut Lumps) -> Vec<u8> {
    lump(l, "THINGS").clone()
}

/// Every M3h9b room with a start loads and runs under wire 23 exactly as under
/// wire 22: the per-map bounds admit what the constants admitted, the first
/// capture and 70 idle tics are the wire-22 captures plus the DSIM39 fields, and
/// every capture decodes with `decode_episode1`.
#[test]
fn every_m3h9b_room_runs_under_wire_23_as_under_wire_22() {
    let limits = DiagnosticLimits::default();
    let mut seen = std::collections::BTreeSet::new();
    for c in cases() {
        if !seen.insert(c.map) {
            continue;
        }
        let map = with_start(room(c.map), 1, c.position, 90).map();
        let mut old = setup(LEVEL, map.clone(), c.seed);
        let mut new = setup(EPISODE1, map.clone(), c.seed);
        let bounds = new.episode1_bounds().unwrap();
        assert_eq!(bounds, Episode1Bounds::from_map(&map));
        assert_eq!(new.map_demon_limit(), bounds.monsters);
        assert_eq!(new.decoration_limit(), bounds.statics);
        assert_eq!(new.barrel_limit(), bounds.barrels);
        assert_eq!(new.world.sight_query_limit, bounds.sight_queries());
        assert!(new.world.checked_arithmetic);
        assert!(!old.world.checked_arithmetic);
        assert_eq!(old.world.sight_query_limit, 64);
        assert_eq!(old.episode1_bounds(), None);
        for tic in 0..=70 {
            if tic > 0 {
                let a = old.advance(zero());
                let b = new.advance(zero());
                assert_eq!(a, b, "room {} tic {tic}", c.map);
                if a.is_err() {
                    break;
                }
            }
            same_but_labels(&fields(&old), &fields(&new));
            if tic % 35 == 0 {
                let bytes = new.snapshot().canonical_bytes();
                Diagnostic::decode_episode1(&bytes, limits)
                    .unwrap_or_else(|e| panic!("room {} tic {tic}: {e:?}", c.map));
            }
        }
    }
}

/// The DSIM39 additions carry the vanilla initial values before the later plan
/// steps change them: no secret exit, the map's floor flats, the
/// `G_PlayerReborn` maxima, no cells, rockets, powers, keys or backpack.
#[test]
fn dsim39_load_capture_carries_the_vanilla_initial_values() {
    let c = &cases()[0];
    let map = with_start(room(c.map), 0, c.position, 0).map();
    let s = setup(EPISODE1, map.clone(), c.seed);
    let all = fields(&s);
    let get = |path: &str| all.iter().find(|(p, _)| p == path).expect(path).1.clone();
    assert_eq!(get("level.secret_exit"), [0]);
    assert_eq!(get("store.max_slots"), 1024u32.to_le_bytes());
    for (i, sector) in map.sectors().iter().enumerate() {
        let name = get(&format!("sector.{i}.floorpic"));
        assert_eq!(
            name,
            sector_thinkers::normalized_flat(&sector.flats[0]).to_vec()
        );
    }
    for (i, max) in [200u16, 50, 300, 50].into_iter().enumerate() {
        assert_eq!(
            get(&format!("original_player.0.weapons.max_ammo.{i}")),
            max.to_le_bytes()
        );
    }
    assert_eq!(get("original_player.0.weapons.cells"), [0, 0]);
    assert_eq!(get("original_player.0.weapons.rockets"), [0, 0]);
    for i in 0..6 {
        assert_eq!(get(&format!("original_player.0.powers.{i}")), [0; 4]);
        assert_eq!(get(&format!("original_player.0.cards.{i}")), [0]);
    }
    assert_eq!(get("original_player.0.backpack"), [0]);
    assert_eq!(
        s.original_player_powers(PlayerSlot::One),
        Ok(PlayerPowers::default())
    );
    // Order: secret_exit right after exit; cells, rockets, maxima after shells;
    // powers, cards, backpack right after use_down; floorpic after special.
    let at = |path: &str| all.iter().position(|(p, _)| p == path).expect(path);
    assert_eq!(at("level.secret_exit"), at("level.exit") + 1);
    assert_eq!(
        at("original_player.0.weapons.cells"),
        at("original_player.0.weapons.shells") + 1
    );
    assert_eq!(
        at("original_player.0.weapons.max_ammo.3"),
        at("original_player.0.weapons.shells") + 6
    );
    assert_eq!(
        at("original_player.0.powers.0"),
        at("original_player.0.use_down") + 1
    );
    assert_eq!(
        at("original_player.0.backpack"),
        at("original_player.0.use_down") + 13
    );
    assert_eq!(at("sector.0.floorpic"), at("sector.0.special") + 1);
    // The older policies have no powers.
    let old = setup(LEVEL, map, c.seed);
    assert_eq!(
        old.original_player_powers(PlayerSlot::One),
        Err(PlayerBodyError::Profile)
    );
}

/// Re-encode `snap` with the DSIM version, `profile` and `context.schema`
/// replaced and `edit` applied to the fields.
fn relabel(
    snap: &Snapshot,
    version: u32,
    profile: OriginalExecutionProfile,
    edit: impl Fn(&str, Vec<u8>) -> Option<Vec<u8>>,
) -> Vec<u8> {
    let fields: Vec<(String, Vec<u8>)> = snap
        .fields()
        .filter_map(|f| {
            let value = match f.path() {
                "profile" => profile.wire_id().to_le_bytes().to_vec(),
                "context.schema" => profile.context_schema().to_vec(),
                _ => f.value().to_vec(),
            };
            edit(f.path(), value).map(|v| (f.path().to_string(), v))
        })
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(version.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (path, value) in fields {
        out.extend((path.len() as u32).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend((value.len() as u64).to_le_bytes());
        out.extend(value);
    }
    out
}

#[test]
fn decode_episode1_round_trip_and_refusals() {
    let limits = DiagnosticLimits::default();
    let c = &cases()[0];
    let map = with_start(room(c.map), 0, c.position, 0).map();
    let s = setup(EPISODE1, map.clone(), c.seed);
    let snap = s.snapshot();
    let bytes = snap.canonical_bytes();
    assert_eq!(&bytes[4..8], &39u32.to_le_bytes());
    let keep = |_: &str, v: Vec<u8>| Some(v);
    assert_eq!(relabel(&snap, 39, EPISODE1, keep), bytes);
    let d = Diagnostic::decode_episode1(&bytes, limits).unwrap();
    assert_eq!(d.encode(limits).unwrap(), bytes);
    // Every frozen reader refuses DSIM39.
    for read in [
        Diagnostic::decode_level,
        Diagnostic::decode_barrels,
        Diagnostic::decode_e1m1,
        Diagnostic::decode,
    ] {
        assert_eq!(
            read(&bytes, limits).err(),
            Some(DiagnosticError::Version(39))
        );
    }
    // decode_episode1 refuses every earlier version, the wire-22 capture included.
    let old = setup(LEVEL, map, c.seed);
    let old_bytes = old.snapshot().canonical_bytes();
    assert_eq!(
        Diagnostic::decode_episode1(&old_bytes, limits).err(),
        Some(DiagnosticError::Version(38))
    );
    Diagnostic::decode_level(&old_bytes, limits).unwrap();
    // The wire-22 capture relabelled as DSIM39 lacks the new fields.
    let relabelled = relabel(&old.snapshot(), 39, EPISODE1, keep);
    assert!(Diagnostic::decode_episode1(&relabelled, limits).is_err());
    // The DSIM39 capture relabelled as a wire-22 one is refused by its context.
    let wrong = relabel(&snap, 39, LEVEL, keep);
    assert_eq!(
        Diagnostic::decode_episode1(&wrong, limits).err(),
        Some(DiagnosticError::Schema)
    );
    // Field domains (plan §A.7).
    let refused = |path: &'static str, value: Vec<u8>| {
        let bytes = relabel(&snap, 39, EPISODE1, |p, v| {
            Some(if p == path { value.clone() } else { v })
        });
        Diagnostic::decode_episode1(&bytes, limits).is_err()
    };
    assert!(refused("level.secret_exit", vec![1])); // without level.exit
    assert!(refused("level.secret_exit", vec![2]));
    assert!(refused("sector.0.floorpic", b"flat5\0\0\0".to_vec()));
    assert!(refused("sector.0.floorpic", b"\0FLAT5\0\0".to_vec()));
    assert!(refused("sector.0.floorpic", b"FL\0AT5\0\0".to_vec()));
    assert!(refused(
        "original_player.0.powers.0",
        1i32.to_le_bytes().to_vec()
    ));
    assert!(refused(
        "original_player.0.powers.1",
        1i32.to_le_bytes().to_vec()
    ));
    assert!(refused(
        "original_player.0.powers.4",
        2i32.to_le_bytes().to_vec()
    ));
    assert!(refused(
        "original_player.0.powers.5",
        4201i32.to_le_bytes().to_vec()
    ));
    // Invisibility without MF_SHADOW on the body.
    assert!(refused(
        "original_player.0.powers.2",
        10i32.to_le_bytes().to_vec()
    ));
    assert!(refused("original_player.0.cards.3", vec![1]));
    assert!(refused("original_player.0.cards.0", vec![2]));
    assert!(refused("original_player.0.backpack", vec![2]));
    // A backpack without doubled maxima, and maxima doubled without a backpack.
    assert!(refused("original_player.0.backpack", vec![1]));
    assert!(refused(
        "original_player.0.weapons.max_ammo.0",
        400u16.to_le_bytes().to_vec()
    ));
    assert!(refused(
        "original_player.0.weapons.cells",
        301u16.to_le_bytes().to_vec()
    ));
    // In-domain values decode: the suit and the visor counting down, a key.
    for (path, value) in [
        ("original_player.0.powers.3", 2100i32.to_le_bytes().to_vec()),
        ("original_player.0.powers.5", 4200i32.to_le_bytes().to_vec()),
        ("original_player.0.powers.4", 1i32.to_le_bytes().to_vec()),
        ("original_player.0.cards.2", vec![1]),
        ("sector.0.floorpic", b"FLAT5_4\0".to_vec()),
    ] {
        assert!(!refused(path, value), "{path}");
    }
    // A missing DSIM39 field.
    let bytes = relabel(&snap, 39, EPISODE1, |p, v| {
        (p != "original_player.0.cards.5").then_some(v)
    });
    assert!(Diagnostic::decode_episode1(&bytes, limits).is_err());
}

// --- Load admission (decision H3) on the committed M3h8 rooms -----------------

fn m3h8(index: usize) -> Lumps {
    crate::player_movers_tests::room(index)
}
fn set_line_word(l: &mut Lumps, line: usize, offset: usize, value: u16) {
    lump(l, "LINEDEFS")[14 * line + offset..14 * line + offset + 2]
        .copy_from_slice(&value.to_le_bytes());
}
fn line_word(l: &mut Lumps, line: usize, offset: usize) -> u16 {
    let v = lump(l, "LINEDEFS");
    u16::from_le_bytes([v[14 * line + offset], v[14 * line + offset + 1]])
}
fn set_heights(l: &mut Lumps, sector: usize, floor: i16, ceiling: i16) {
    let s = lump(l, "SECTORS");
    s[26 * sector..26 * sector + 2].copy_from_slice(&floor.to_le_bytes());
    s[26 * sector + 2..26 * sector + 4].copy_from_slice(&ceiling.to_le_bytes());
}
fn set_ceiling_flat(l: &mut Lumps, sector: usize, name: &[u8; 8]) {
    lump(l, "SECTORS")[26 * sector + 12..26 * sector + 20].copy_from_slice(name);
}
fn set_front_texture(l: &mut Lumps, line: usize, part: usize, name: &[u8; 8]) {
    let front = usize::from(line_word(l, line, 10));
    lump(l, "SIDEDEFS")[30 * front + 4 + 8 * part..30 * front + 12 + 8 * part]
        .copy_from_slice(name);
}

/// Every committed M3h8 room loads under wire 23 with the same mover admission
/// as under wire 22 (the rules it keeps are the rules those rooms pass).
#[test]
fn episode1_admits_every_committed_mover_room() {
    for index in 0..crate::player_movers_tests::ROOM_COUNT {
        let lumps = m3h8(index);
        let old = admitted(&lumps, LEVEL).unwrap();
        let new = admitted(&lumps, EPISODE1).unwrap_or_else(|e| panic!("room {index}: {e:?}"));
        assert_eq!(old.movers, new.movers, "room {index}");
        // `L + 2·numsectors` sector-thinker slots under wire 23.
        assert!(new.actors.max_sector_slots >= old.actors.max_sector_slots);
    }
}

/// The wire-22 load rules decision H3 turns into per-tic witnesses or drops: rule 2
/// (two mover families), rule 4 (reachable heights beyond ±512, an inverting door),
/// rule 6 (nine special lines in one window), rule 8 (a sky ceiling over a mover)
/// and the switch rule generalised by decision D3. Wire 22 keeps refusing each.
#[test]
fn episode1_bypasses_the_mover_proofs_that_refuse_episode1_maps() {
    let both = |lumps: &Lumps, error: MapScaleError| {
        assert_eq!(
            admitted(lumps, LEVEL).err(),
            Some(SimError::MapScale(error))
        );
        admitted(lumps, EPISODE1).unwrap_or_else(|e| panic!("{error:?}: {e:?}"));
    };
    // Rule 2: `floors` line 4 (W1 36) retagged onto the door sector 11.
    let mut l = m3h8(13);
    set_line_word(&mut l, 4, 8, 5);
    both(&l, MapScaleError::MoverFamily);
    // Rule 4: a turbo destination of 514; heights saturate instead of refusing.
    let mut l = m3h8(13);
    set_heights(&mut l, 6, 506, 512);
    both(&l, MapScaleError::MoverHeights);
    let reach = admitted(&l, EPISODE1).unwrap().movers.unwrap().reach;
    assert_eq!(reach[1], 514);
    // Rule 4b: an inverting door.
    let mut l = m3h8(0);
    set_heights(&mut l, 0, 0, 20);
    both(&l, MapScaleError::InvertingDoor);
    // Rule 6: the Rust-only nine-line lane, without the test bypass.
    let dense9 = Lumps::parse(include_bytes!("../../../tests/reference/m3h8_dense9.bin"));
    both(&dense9, MapScaleError::SpecialWindow);
    // Rule 8: a door sector with an F_SKY1 ceiling.
    let mut l = m3h8(0);
    set_ceiling_flat(&mut l, 2, b"F_SKY1\0\0");
    both(&l, MapScaleError::SkyMover);
    // D3: switch lines without an episode-1 texture load; a later-episode-only
    // texture still refuses.
    let mut plain = m3h8(6);
    for line in [1, 10] {
        set_front_texture(&mut plain, line, 2, b"TESTWALL");
    }
    both(&plain, MapScaleError::SwitchTexture);
    let mut blue = m3h8(6);
    set_front_texture(&mut blue, 10, 2, b"SW1BLUE\0");
    for policy in [LEVEL, EPISODE1] {
        assert_eq!(
            admitted(&blue, policy).err(),
            Some(SimError::MapScale(MapScaleError::SwitchTexture))
        );
    }
    let mut both_columns = m3h8(6);
    set_front_texture(&mut both_columns, 10, 0, b"SW1BLUE\0");
    admitted(&both_columns, EPISODE1).unwrap();
}

/// Decision D3 at the tic: under wire 23 the exit switch of `switchyard` with a
/// plain texture still clears its special and exits, without a switch sound or a
/// button (the source's texture search finds nothing, p_switch.c:211–263); with
/// its episode-1 texture the sound plays as under every older policy.
#[test]
fn episode1_switch_without_a_texture_match_is_silent() {
    for (texture, sound) in [(b"TESTWALL", false), (b"SW1COMM\0", true)] {
        let mut l = m3h8(6);
        set_front_texture(&mut l, 10, 2, texture);
        let s = admitted(&l, EPISODE1).unwrap();
        let mut events = Vec::new();
        let mut buttons = [None; mover_rules::MAX_BUTTONS];
        let after = crate::use_lines::change_switch_texture(
            &s.world,
            &mut events,
            &mut buttons,
            10,
            mover_rules::LINE_EXIT,
            false,
        )
        .unwrap();
        assert_eq!(after, 0);
        assert!(buttons.iter().all(Option::is_none));
        assert_eq!(!events.is_empty(), sound, "{texture:?}");
    }
}

/// Rule 2 as a per-tic witness: under wire 23 a sector may be both a floor target
/// and a DR door's back sector; while its floor moves, a DR use casts
/// `specialdata` to a door and is refused with `MapScaleError::MoverFamily`,
/// leaving the state as it was.
#[test]
fn episode1_mover_family_is_refused_at_use_time() {
    let mut l = m3h8(13);
    set_line_word(&mut l, 4, 8, 5);
    lump(&mut l, "THINGS").extend(record(1100, 64, 0, 1, 7));
    let mut s = setup(EPISODE1, l.map(), 6);
    assert!(
        s.start_movers(|m| m.do_floor(4, movers::FloorKind::TurboLower))
            .unwrap()
    );
    let before = s.snapshot().canonical_bytes();
    // The DSIM39 floor tail (plan §A.6): `direction` and `speed` right after
    // `floordestheight`; no `texture` or `newspecial` for a turboLower floor.
    let all = fields(&s);
    let kind = all
        .iter()
        .position(|(p, v)| p.starts_with("sector_thinker.") && p.ends_with(".kind") && v == &[7])
        .expect("a floor record");
    let prefix = all[kind].0.trim_end_matches(".kind").to_string();
    let names: Vec<&str> = all[kind..kind + 6]
        .iter()
        .map(|(p, _)| p.strip_prefix(&format!("{prefix}.")).unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "kind",
            "sector",
            "type",
            "floordestheight",
            "direction",
            "speed"
        ]
    );
    assert_eq!(all[kind + 4].1, (-1i32).to_le_bytes());
    assert_eq!(all[kind + 5].1, (4 * 65536i32).to_le_bytes());
    assert!(!all[kind + 6].0.starts_with(&format!("{prefix}.")));
    Diagnostic::decode_episode1(&before, DiagnosticLimits::default()).unwrap();
    assert_eq!(
        s.start_movers(|m| m.vertical_door(22, true)),
        Err(SimError::MapScale(MapScaleError::MoverFamily))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
    s.advance(zero()).unwrap();
}

/// Decision H8's load rule (`mover_rules::donut_geometry`): a donut line's tagged
/// sector needs a two-sided first line whose next sector differs from it, and the
/// first line of that sector not facing back must have a back side.
#[test]
fn donut_geometry_refuses_the_null_dereferences_of_ev_dodonut() {
    // `doorway` (room 0): sector 2 is a door sector between rooms; make line 4 (a
    // DR line) a donut line tagging a sector by its tag.
    let mut l = m3h8(0);
    let map = l.map();
    assert_eq!(mover_rules::donut_geometry(&map), Ok(()));
    // A donut on sector 0 (the outer room, whose first line is one-sided).
    let lists = player_map_scale::sector_lines(&map);
    let one_sided_first = (0..map.sectors().len())
        .find(|&s| {
            lists[s]
                .first()
                .is_some_and(|&line| map.linedefs()[usize::from(line)].sides[1].is_none())
        })
        .expect("a sector whose first line is one-sided");
    let tag: i16 = 77;
    let s = lump(&mut l, "SECTORS");
    s[26 * one_sided_first + 24..26 * one_sided_first + 26].copy_from_slice(&tag.to_le_bytes());
    set_line_word(&mut l, 4, 6, 9);
    set_line_word(&mut l, 4, 8, tag as u16);
    assert_eq!(
        mover_rules::donut_geometry(&l.map()),
        Err(SimError::MapScale(MapScaleError::DonutGeometry))
    );
}

/// The witness variants of the later steps exist, encode as `SimError::MapScale`
/// and name themselves.
#[test]
fn episode1_witness_variants_are_constructible() {
    for (error, name) in [
        (MapScaleError::SecondDoor, "SecondDoor"),
        (MapScaleError::StairBlocked, "StairBlocked"),
        (MapScaleError::PlatBlocked, "PlatBlocked"),
        (MapScaleError::DonutBusy, "DonutBusy"),
        (MapScaleError::DonutGeometry, "DonutGeometry"),
    ] {
        let sim: SimError = error.into();
        assert_eq!(sim, SimError::MapScale(error));
        assert_eq!(error.to_string(), format!("map scale: {name}"));
    }
    assert_eq!(
        CombatError::Arithmetic.to_string(),
        "combat: Arithmetic".to_string()
    );
}

// --- The real-map proofs (decisions H3, D4) ------------------------------------

fn real_map_room(label: &str) -> Lumps {
    let cases = crate::real_map_evidence::cases();
    let c = cases.iter().find(|c| c.label == label).unwrap();
    Lumps::parse(crate::real_map_evidence::MAP_BYTES[c.map])
}
fn wedge() -> Lumps {
    let cases = crate::diagonal_evidence::cases();
    let c = cases.iter().find(|c| c.label == "slide_45").unwrap();
    Lumps::parse(crate::diagonal_evidence::MAP_BYTES[c.map])
}

/// Certificate clause (vii) and the ±512 rule: a static height of 513 refuses
/// wire 22 and loads under wire 23 (its traces compute heights checked); the
/// format envelope (±1,024) still refuses. A boundary failing C3 (decision D4)
/// loads under wire 23 with no boundary.
#[test]
fn episode1_admits_heights_beyond_512_and_a_map_without_a_boundary() {
    let mut town = real_map_room("town_patrol");
    admitted(&town, EPISODE1).unwrap();
    town.set_sector(0, [0, 513], 0, b"TESTCEIL");
    assert_eq!(
        admitted(&town, LEVEL).err(),
        Some(SimError::MapScale(MapScaleError::Arithmetic))
    );
    let s = admitted(&town, EPISODE1).unwrap();
    assert!(s.world.checked_arithmetic);
    town.set_sector(0, [0, 1025], 0, b"TESTCEIL");
    assert!(admitted(&town, EPISODE1).is_err());
    let mut wedge = wedge();
    let wall = wedge.find_line([-256, 768], [256, 256]);
    wedge.reverse_line(wall);
    assert_eq!(
        admitted(&wedge, LEVEL).err(),
        Some(SimError::MapScale(MapScaleError::Boundary))
    );
    let s = admitted(&wedge, EPISODE1).unwrap();
    assert!(s.boundary.is_none());
    assert!(
        admitted(&self::wedge(), EPISODE1)
            .unwrap()
            .boundary
            .is_some()
    );
}

/// The checked `P_InterceptVector` equals the source's wrapping one on the 131
/// committed source rows of M3d3a, and refuses where a signed `int` difference
/// would overflow.
#[test]
fn checked_intercept_matches_the_source_rows_and_refuses_overflow() {
    let f = |v: i64| Fixed::from_bits(v as i32);
    let mut count = 0;
    for line in include_str!("../../../tests/reference/m3d3a_fractions.csv")
        .lines()
        .skip(1)
    {
        let n: Vec<i64> = line.split(',').map(|s| s.parse().unwrap()).collect();
        let (v2s, v2d) = ([f(n[0]), f(n[1])], [f(n[2]), f(n[3])]);
        let (v1s, v1d) = ([f(n[4]), f(n[5])], [f(n[6]), f(n[7])]);
        if let Some(v) = sight::checked_intercept(v2s, v2d, v1s, v1d) {
            assert_eq!(i64::from(v.bits()), n[8], "row {count}");
        }
        count += 1;
    }
    assert_eq!(count, 131);
    // `v1.x − v2.x` past `i32`: refused.
    let max = Fixed::from_bits(i32::MAX);
    let min = Fixed::from_bits(i32::MIN + 1);
    let one = Fixed::ONE;
    assert_eq!(
        sight::checked_intercept(
            [min, Fixed::ZERO],
            [one, one],
            [max, Fixed::ZERO],
            [one, -one]
        ),
        None
    );
    // A `FixedMul` product out of `fixed_t`: refused.
    let big = Fixed::from_bits(1 << 30);
    assert_eq!(
        sight::checked_intercept(
            [Fixed::ZERO; 2],
            [big, big],
            [Fixed::ZERO; 2],
            [big, Fixed::from_bits(-(1 << 30))]
        ),
        None
    );
}

/// The store-full witness that replaces the older policies' "the store never
/// fills" proof (decision H4): a Rust-only room whose THINGS lump places more
/// pickups than the 1,024 slots hold loads (its bounds are its own counts), and
/// its spawn batch is refused with `StoreError::Capacity`, spawning nothing.
#[test]
fn episode1_store_full_refuses_the_spawn_batch() {
    let c = &cases()[0];
    let mut l = with_start(room(c.map), 0, c.position, 0);
    for _ in 0..1024 {
        lump(&mut l, "THINGS").extend(record(c.position[0], c.position[1], 0, 2007, 7));
    }
    let mut s = Simulation::original_player_episode1_map(l.map(), options(c.seed)).unwrap();
    assert!(s.episode1_bounds().unwrap().pickups >= 1024);
    let before = s.snapshot().canonical_bytes();
    assert_eq!(
        s.spawn_map_things().err(),
        Some(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(s.snapshot().canonical_bytes(), before);
}
