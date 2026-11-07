//! The DSIM39 reader's refusal domains, through the public API (M4a
//! mutation-testing step, run 1 shard 2: the survivors in `diagnostic.rs`).
//!
//! Each test takes a valid capture, changes one field (or one event payload,
//! appended to `events.last` or edited in place) to a value exactly on or just
//! past the boundary of its written domain, re-encodes it with the canonical
//! framing, and asserts that `Diagnostic::decode_episode1` (or, for the shared
//! event validators, the older `decode_level`) refuses it with the reader's
//! error class and accepts the boundary value itself. `DiagnosticError` carries
//! only the class (`Structure`, `Identity`, `Limit`, `Schema`), not the field, so
//! each case changes exactly one field of an otherwise accepted capture.
//!
//! Base captures:
//! - the committed independent fixtures `tests/reference/m4a_episode1_*.bin`
//!   (projected from the genuine engine's dumps by `level_snapshot.py`, never
//!   from Rust output) and `m3h9c_level_SYNIDLE_0.bin` (DSIM38);
//! - for event payloads, which the fixtures do not carry, Rust captures of the
//!   M4a mechanism scripts (`tests/reference/m4a_wads/`, `m4a_scripts/`), whose
//!   every capture `episode1_mechanism_goldens.rs` pins to the genuine engine's
//!   goldens. Only the presence of an event kind is taken from them; every
//!   expected accept/refuse decision comes from docs/STATE_FORMAT.md (the DSIM
//!   v39 section and the sections it inherits) and the linuxdoom-1.10 lines
//!   cited at each case.
use doom_sim::identity::{OriginalActorType, OriginalSprite, OriginalState};
use doom_sim::{demo::parse_demo_episode1, *};
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}
/// A DSIM capture as (path, value) pairs in encoded order, re-encoded after an
/// edit with the canonical framing (`DSIM`, u32 version, u64 count, then per field
/// u32 path length, path, u64 value length, value: STATE_FORMAT "Bounded
/// encoding and reading").
#[derive(Clone)]
struct Capture {
    version: u32,
    fields: Vec<(String, Vec<u8>)>,
}
impl Capture {
    fn parse(bytes: &[u8]) -> Self {
        assert_eq!(&bytes[..4], b"DSIM");
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let (mut at, mut fields) = (16, Vec::new());
        for _ in 0..count {
            let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            let path = String::from_utf8(bytes[at + 4..at + 4 + len].to_vec()).unwrap();
            at += 4 + len;
            let len = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
            fields.push((path, bytes[at + 8..at + 8 + len].to_vec()));
            at += 8 + len;
        }
        assert_eq!(at, bytes.len());
        Self { version, fields }
    }
    fn fixture(name: &str) -> Self {
        Self::parse(&std::fs::read(reference(name)).expect("fixture"))
    }
    fn encode(&self) -> Vec<u8> {
        let mut out = b"DSIM".to_vec();
        out.extend(self.version.to_le_bytes());
        out.extend((self.fields.len() as u64).to_le_bytes());
        for (path, value) in &self.fields {
            out.extend((path.len() as u32).to_le_bytes());
            out.extend(path.as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    }
    fn position(&self, path: &str) -> usize {
        self.fields
            .iter()
            .position(|(p, _)| p == path)
            .unwrap_or_else(|| panic!("missing field {path}"))
    }
    fn get(&self, path: &str) -> &[u8] {
        &self.fields[self.position(path)].1
    }
    fn has(&self, path: &str) -> bool {
        self.fields.iter().any(|(p, _)| p == path)
    }
    fn i32(&self, path: &str) -> i32 {
        i32::from_le_bytes(self.get(path).try_into().unwrap())
    }
    fn u64(&self, path: &str) -> u64 {
        u64::from_le_bytes(self.get(path).try_into().unwrap())
    }
    fn set(&mut self, path: &str, value: impl Into<Vec<u8>>) -> &mut Self {
        let at = self.position(path);
        self.fields[at].1 = value.into();
        self
    }
    fn insert_after(&mut self, after: &str, path: &str, value: impl Into<Vec<u8>>) -> &mut Self {
        let at = self.position(after) + 1;
        self.fields.insert(at, (path.to_string(), value.into()));
        self
    }
    /// Append `event` to `events.last` (count and the next index).
    fn push_event(&mut self, event: &[u8]) -> &mut Self {
        let n = self.u64("events.last.count");
        let after = if n == 0 {
            "events.last.count".to_string()
        } else {
            format!("events.last.{}", n - 1)
        };
        self.set("events.last.count", (n + 1).to_le_bytes());
        self.insert_after(&after, &format!("events.last.{n}"), event.to_vec());
        self
    }
    /// The sector-thinker slots holding a live record of `kind`.
    fn thinkers(&self, kind: u8) -> Vec<String> {
        (0..self.u64("sector_thinkers.slot_count"))
            .map(|i| format!("sector_thinker.{i}"))
            .filter(|p| {
                self.get(&format!("{p}.occupied")) == [1]
                    && self.get(&format!("{p}.kind")) == [kind]
            })
            .collect()
    }
    /// Occupied actor prefixes of type `ty`.
    fn actors(&self, ty: OriginalActorType) -> Vec<String> {
        (0..self.u64("store.slot_count"))
            .map(|i| format!("actor.{i}"))
            .filter(|p| {
                self.get(&format!("{p}.occupied")) == [1]
                    && self.get(&format!("{p}.type")) == ty.ordinal().to_le_bytes()
            })
            .collect()
    }
    fn decode(&self) -> Result<(), DiagnosticError> {
        let bytes = self.encode();
        match self.version {
            39 => Diagnostic::decode_episode1(&bytes, DiagnosticLimits::default()).map(|_| ()),
            38 => Diagnostic::decode_level(&bytes, DiagnosticLimits::default()).map(|_| ()),
            v => panic!("version {v}"),
        }
    }
}

impl Capture {
    /// The path of the first event (in `events.last`, then `events.pending`)
    /// whose payload starts with `prefix`.
    fn event(&self, prefix: &[u8]) -> String {
        self.fields
            .iter()
            .find(|(p, v)| {
                (p.starts_with("events.last.") || p.starts_with("events.pending."))
                    && !p.ends_with(".count")
                    && v.starts_with(prefix)
            })
            .map(|(p, _)| p.clone())
            .unwrap_or_else(|| panic!("no event {prefix:?}"))
    }
    /// The decision on this capture with field `path` replaced by `value`.
    fn with(&self, path: &str, value: impl Into<Vec<u8>>) -> Result<(), DiagnosticError> {
        let mut c = self.clone();
        c.set(path, value);
        c.decode()
    }
    /// The decision on this capture with event `path` edited by `edit`.
    fn with_event(
        &self,
        path: &str,
        edit: impl FnOnce(&mut Vec<u8>),
    ) -> Result<(), DiagnosticError> {
        let mut event = self.get(path).to_vec();
        edit(&mut event);
        self.with(path, event)
    }
    /// The decision on this capture with `event` appended to `events.last`.
    fn with_pushed(&self, event: &[u8]) -> Result<(), DiagnosticError> {
        let mut c = self.clone();
        c.push_event(event);
        c.decode()
    }
}
fn put_i32(event: &mut [u8], at: usize, value: i32) {
    event[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put_u16(event: &mut [u8], at: usize, value: u16) {
    event[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn state(name: &str) -> u16 {
    OriginalState::from_name(name).expect(name).ordinal()
}
fn sprite(name: &str) -> u16 {
    OriginalSprite::from_name(name).expect(name).ordinal()
}
fn actor_type(name: &str) -> OriginalActorType {
    OriginalActorType::from_name(name).expect(name)
}
/// 16.16 fixed point.
const UNIT: i32 = 1 << 16;

/// Runs the M4a mechanism `script` on its WAD (as `episode1_mechanism_goldens.rs`
/// does) and returns the canonical capture after each tic in `tics` (capture
/// `N` follows the `N`-th demo command; 0 is the load capture).
fn script(name: &str, wad: &str, tics: &[usize]) -> Vec<Capture> {
    let bytes = std::fs::read(reference(&format!("m4a_wads/{wad}.wad"))).expect("WAD");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("map");
    let lmp = std::fs::read(reference(&format!("m4a_scripts/{name}.lmp"))).expect("demo");
    let (_, demo) = parse_demo_episode1(&lmp, 1).expect("demo");
    let mut s = Simulation::original_player_episode1_map(map, MapContextOptions::default())
        .expect("admission");
    s.spawn_map_things().expect("THINGs");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    s.spawn_level_specials().expect("specials");
    let last = *tics.iter().max().expect("a tic");
    let mut out = Vec::new();
    for tic in 0..=last {
        if tic > 0 {
            let command = demo.commands[tic - 1];
            s.advance([
                command,
                TicCommand::default(),
                TicCommand::default(),
                TicCommand::default(),
            ])
            .expect("tic");
        }
        if tics.contains(&tic) {
            let c = Capture::parse(&s.snapshot().canonical_bytes());
            assert_eq!(c.decode(), Ok(()), "{name} capture {tic}");
            out.push(c);
        }
    }
    out
}
fn fixture(name: &str) -> Capture {
    let c = Capture::fixture(&format!("m4a_episode1_{name}.bin"));
    assert_eq!(c.version, 39);
    assert_eq!(c.decode(), Ok(()), "{name}");
    c
}
/// The independent DSIM38 load capture of the M3h9c synthetic WAD.
fn level_fixture() -> Capture {
    let c = Capture::fixture("m3h9c_level_SYNIDLE_0.bin");
    assert_eq!(c.version, 38);
    assert_eq!(c.decode(), Ok(()));
    c
}

/// Every committed independent DSIM39 fixture (`m4a_episode1_*.bin`, 18 files)
/// decodes with `decode_episode1`, the exact `store.max_slots` rule included.
#[test]
fn every_committed_dsim39_fixture_decodes() {
    let mut names: Vec<String> = std::fs::read_dir(reference(""))
        .expect("reference directory")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .filter(|n| n.starts_with("m4a_episode1_") && n.ends_with(".bin"))
        .collect();
    names.sort();
    assert_eq!(names.len(), 18);
    for name in names {
        let c = Capture::fixture(&name);
        assert_eq!(c.version, 39, "{name}");
        assert_eq!(c.decode(), Ok(()), "{name}");
    }
}

// --- Store and actor records -------------------------------------------------

/// An actor's `.previous` and `.next` are optional identities (STATE_FORMAT
/// "DSIM-v21"/field table: "Occupied slot `.pending_removal`, `.previous`,
/// `.next` | u8 boolean, optional identities"; "an optional identity is u8
/// presence + those 12 bytes; absent identities have all-zero payloads"): a
/// presence byte other than 0 or 1 is refused. Survivor `diagnostic.rs:662:17`.
#[test]
fn actor_links_are_optional_identities() {
    let c = fixture("DOORS1_86");
    for link in ["previous", "next"] {
        let path = (0..c.u64("store.slot_count"))
            .map(|i| format!("actor.{i}.{link}"))
            .find(|p| c.has(p) && c.get(p)[0] == 1)
            .expect("a present link");
        let mut value = c.get(&path).to_vec();
        value[0] = 2;
        assert_eq!(
            c.with(&path, value),
            Err(DiagnosticError::Identity),
            "{path}"
        );
        let mut absent = vec![0; 13];
        absent[1] = 1;
        assert_eq!(
            c.with(&path, absent),
            Err(DiagnosticError::Identity),
            "{path}"
        );
    }
}

/// `store.slot_count` never exceeds `store.max_slots` (1,024 under wire 23:
/// STATE_FORMAT "DSIM v39", `store.max_slots` u32 1024; COMPATIBILITY "Bounds",
/// actor slots 1,024): a store one slot over its maximum is refused, and a store
/// of exactly 1,024 slots (the added ones free: `.generation`, `.retired`,
/// `.occupied` 0, STATE_FORMAT "DSIM-v21" store layout) is accepted. Survivors
/// `diagnostic.rs:1726:13` (`>` with `==` and with `>=`).
#[test]
fn the_store_holds_at_most_max_slots() {
    let c = fixture("ITEMS1_121");
    let slots = c.u64("store.slot_count");
    assert_eq!(c.get("store.max_slots"), 1024u32.to_le_bytes());
    assert_eq!(
        c.with("store.max_slots", (slots as u32 - 1).to_le_bytes()),
        Err(DiagnosticError::Limit)
    );
    let mut full = c.clone();
    let mut after = format!("actor.{}.", slots - 1);
    after = full
        .fields
        .iter()
        .rev()
        .find(|(p, _)| p.starts_with(&after))
        .map(|(p, _)| p.clone())
        .expect("the last slot's fields");
    for slot in slots..1024 {
        let prefix = format!("actor.{slot}");
        full.insert_after(&after, &format!("{prefix}.generation"), 1u64.to_le_bytes());
        full.insert_after(
            &format!("{prefix}.generation"),
            &format!("{prefix}.retired"),
            [0],
        );
        full.insert_after(
            &format!("{prefix}.retired"),
            &format!("{prefix}.occupied"),
            [0],
        );
        after = format!("{prefix}.occupied");
    }
    full.set("store.slot_count", 1024u64.to_le_bytes());
    assert_eq!(full.decode(), Ok(()));
}

/// `store.max_slots` is exactly 1,024 under wire 23 (STATE_FORMAT "DSIM v39":
/// "`store.max_slots` | same field | u32 `1024`"; decision D1,
/// docs/M4A_REFERENCE_INVESTIGATION.md §14: "`store.max_slots` is the constant
/// 1,024 under wire 23"): 1,023 and 1,025 are refused, 1,024 is accepted. The
/// DSIM38 reader keeps its historical upper-bound check (385 refused); that it
/// accepts a value below 384 is a documented older-schema gap, not pinned here.
#[test]
fn max_slots_is_exactly_1024() {
    let c = fixture("ITEMS1_121");
    assert_eq!(c.with("store.max_slots", 1024u32.to_le_bytes()), Ok(()));
    assert_eq!(
        c.with("store.max_slots", 1023u32.to_le_bytes()),
        Err(DiagnosticError::Limit)
    );
    assert_eq!(
        c.with("store.max_slots", 1025u32.to_le_bytes()),
        Err(DiagnosticError::Limit)
    );
    let level = level_fixture();
    assert_eq!(level.get("store.max_slots"), 384u32.to_le_bytes());
    assert_eq!(
        level.with("store.max_slots", 385u32.to_le_bytes()),
        Err(DiagnosticError::Limit)
    );
}

/// The player record's domains (STATE_FORMAT "DSIM-v22" original player body,
/// widened by "DSIM v35"): `damage_count` 0..=100 (`if (player->damagecount > 100) player->damagecount = 100`,
/// p_inter.c:877–878),
/// `frags` never negative, and `state` 0 live, 1 dead or (DSIM35) 2 reborn
/// request (`playerstate_t`, d_player.h; decision N3). Survivors
/// `diagnostic.rs:2419:13`, `2420:13` and `2420:81`.
#[test]
fn player_counters_and_state_stay_in_their_domains() {
    let c = fixture("DOORS1_86");
    let damage = "original_player.0.damage_count";
    assert_eq!(c.with(damage, 100i32.to_le_bytes()), Ok(()));
    assert_eq!(
        c.with(damage, 101i32.to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(
        c.with(damage, (-1i32).to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
    let frags = "original_player.0.frags";
    assert_eq!(c.i32(frags), 0);
    assert_eq!(
        c.with(frags, (-1i32).to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
    // A dead player (EXDIE: the body killed at dump tic 106): state 1 is
    // accepted, 3 (no `playerstate_t` value) is refused.
    let dead = &script("EXDIE", "exits", &[107])[0];
    assert_eq!(dead.get("original_player.0.state"), [1]);
    assert_eq!(dead.i32("original_player.0.health"), 0);
    assert_eq!(
        dead.with("original_player.0.state", [3]),
        Err(DiagnosticError::Identity)
    );
}

/// Every `sector.N.floorpic` (and a donut's `texture`) is a normalized flat name:
/// at least one byte, upper-case ASCII graphic bytes up to the first NUL, NUL
/// padding after it (STATE_FORMAT "DSIM v39": "every `floorpic` is a normalized
/// name"; decision D2). All eight bytes NUL is the empty name and is refused; a
/// one-byte name is accepted. Survivor `diagnostic.rs:2515:9`.
#[test]
fn a_flat_name_has_at_least_one_byte() {
    let c = fixture("DOORS1_86");
    assert_eq!(
        c.with("sector.0.floorpic", [0; 8]),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(c.with("sector.0.floorpic", *b"A\0\0\0\0\0\0\0"), Ok(()));
    assert_eq!(c.with("sector.0.floorpic", *b"FLAT5_4\0"), Ok(()));
}

/// The ammunition counts are within `maxammo`, the `G_PlayerReborn` maxima
/// (g_game.c:831 copies `maxammo[NUMAMMO] = {200, 50, 300, 50}`, p_inter.c:58)
/// doubled once by the first backpack (p_inter.c:590–594): STATE_FORMAT "DSIM
/// v39": "each ammunition count is at most its maximum". Each count is accepted
/// at its maximum and refused one above, without and with the backpack.
/// Survivors `diagnostic.rs:1795:42`, `1795:82` (`>` with `==` and `>=`) and
/// `2311:90` (the backpack read for the weapon witness).
#[test]
fn ammunition_counts_reach_their_maxima_and_no_further() {
    let base = [200u16, 50, 300, 50];
    let names = ["bullets", "shells", "cells", "rockets"];
    let plain = fixture("DOORS1_86");
    assert_eq!(plain.get("original_player.0.backpack"), [0]);
    let pack = fixture("ITEMS1_121");
    assert_eq!(pack.get("original_player.0.backpack"), [1]);
    for (c, shift) in [(&plain, 0), (&pack, 1)] {
        for (i, name) in names.into_iter().enumerate() {
            let path = format!("original_player.0.weapons.{name}");
            let max = base[i] << shift;
            assert_eq!(
                c.get(&format!("original_player.0.weapons.max_ammo.{i}")),
                max.to_le_bytes()
            );
            assert_eq!(c.with(&path, max.to_le_bytes()), Ok(()), "{path} {max}");
            assert_eq!(
                c.with(&path, (max + 1).to_le_bytes()),
                Err(DiagnosticError::Schema),
                "{path} {}",
                max + 1
            );
        }
    }
}

/// Without a weapon group (`original_player.0.weapons.present` 0: the capture
/// before `initialize_player_weapons`, STATE_FORMAT "DSIM-v24": "0 means no
/// other weapon fields follow") the backpack flag is 0; a backpack there is
/// refused. Survivor `diagnostic.rs:1798:55`.
#[test]
fn no_backpack_without_a_weapon_group() {
    let bytes = std::fs::read(reference("m4a_wads/items.wad")).expect("WAD");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("map");
    let mut s = Simulation::original_player_episode1_map(map, MapContextOptions::default())
        .expect("admission");
    s.spawn_map_things().expect("THINGs");
    let c = Capture::parse(&s.snapshot().canonical_bytes());
    assert_eq!(c.get("original_player.0.weapons.present"), [0]);
    assert_eq!(c.get("original_player.0.backpack"), [0]);
    assert_eq!(c.decode(), Ok(()));
    assert_eq!(
        c.with("original_player.0.backpack", [1]),
        Err(DiagnosticError::Structure)
    );
}

/// A placed COUNTITEM item still in the world has not been collected: the live
/// uncollected COUNTITEM actors are at most `level.total_items -
/// pickups.item_count` (STATE_FORMAT "DSIM-v27": `item_count` 0..total, a
/// nonzero total equals the placed counted THINGs; p_mobj.c:792 counts
/// `MF_COUNTITEM` at spawn, p_inter.c:656 credits `itemcount` on pickup). Under
/// wire 23 the counted kinds include the soulsphere, partial invisibility,
/// computer map and light visor (info.c `MF_COUNTITEM` on MT_MISC12, MT_INS,
/// MT_MISC15, MT_MISC16), not only the two bonuses. ITEMS1 at tic 121 has four
/// of them standing and nothing collected; one item credited is refused.
/// Survivors `diagnostic.rs:2102:17` and `2102:34`.
#[test]
fn standing_counted_items_are_uncollected() {
    let c = fixture("ITEMS1_121");
    assert_eq!(c.i32("level.total_items"), 4);
    assert_eq!(c.i32("original_player.0.pickups.item_count"), 0);
    for ty in ["MT_MISC12", "MT_INS", "MT_MISC15", "MT_MISC16"] {
        assert_eq!(c.actors(actor_type(ty)).len(), 1, "{ty}");
    }
    assert!(c.actors(actor_type("MT_MISC2")).is_empty());
    assert!(c.actors(actor_type("MT_MISC3")).is_empty());
    assert_eq!(
        c.with("original_player.0.pickups.item_count", 1i32.to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
}

/// Decorations from DSIM39 include the candle (MT_MISC49) and the firestick
/// (MT_MISC43), checked as the E1M1 seven are (STATE_FORMAT "DSIM v36":
/// "health 1000"; info.c:3682–3706 MT_MISC49 and info.c:3526–3550 MT_MISC43,
/// spawnhealth 1000): one of each with health 999 is refused. Survivor
/// `diagnostic.rs:1456:25`.
#[test]
fn candles_and_firesticks_are_checked_decorations() {
    let c = fixture("SCALE2_0");
    for ty in ["MT_MISC49", "MT_MISC43"] {
        let actor = c.actors(actor_type(ty)).into_iter().next().expect(ty);
        let health = format!("{actor}.health");
        assert_eq!(c.i32(&health), 1000);
        assert_eq!(
            c.with(&health, 999i32.to_le_bytes()),
            Err(DiagnosticError::Identity),
            "{ty}"
        );
    }
}

/// The barrel bound `B` is counted from `map.things` with the single-player,
/// medium-skill filter of `P_SpawnMapThing` (COMPATIBILITY "Bounds": "the
/// DSIM39 reader derives the same values from `map.things`"; p_mobj.c:740:
/// a record with `options & 16` is not spawned in single player). With
/// one barrel record turned multiplayer-only the 24 barrels of SCALE2's load
/// capture exceed `B` = 23 and the capture is refused as over its limit.
/// Survivor `diagnostic.rs:1603:19`.
#[test]
fn barrels_beyond_the_counted_bound_are_refused() {
    let c = fixture("SCALE2_0");
    let barrels = c.actors(actor_type("MT_BARREL"));
    assert_eq!(barrels.len(), 24);
    let last = barrels.last().unwrap();
    let spawn = c.get(&format!("{last}.spawnpoint")).to_vec();
    assert_eq!(spawn.len(), 11);
    assert_eq!(spawn[0], 1);
    let mut things = c.get("map.things").to_vec();
    let matches: Vec<usize> = (0..things.len() / 10)
        .filter(|i| things[10 * i..10 * i + 10] == spawn[1..])
        .collect();
    assert_eq!(matches.len(), 1);
    let at = 10 * matches[0] + 8;
    let options = u16::from_le_bytes([things[at], things[at + 1]]) | 16;
    things[at..at + 2].copy_from_slice(&options.to_le_bytes());
    assert_eq!(c.with("map.things", things), Err(DiagnosticError::Limit));
}

// --- Sector thinkers, sector and line specials, buttons ------------------------

/// A door record's `type` is 0 `normal`, 1 `close30ThenOpen`, 3 `open` or 6
/// `blazeOpen` (STATE_FORMAT "DSIM v39", door record; `vldoor_e`, p_spec.h).
/// Survivor `diagnostic.rs:1225:56`.
#[test]
fn door_types_are_the_four_episode1_doors() {
    let c = fixture("DOORS1_86");
    let door = c.thinkers(5).into_iter().next().expect("a door");
    let ty = format!("{door}.type");
    for t in [0u8, 1, 3, 6] {
        assert_eq!(c.with(&ty, [t]), Ok(()), "type {t}");
    }
    for t in [2u8, 4, 5, 7, 255] {
        assert_eq!(c.with(&ty, [t]), Err(DiagnosticError::Identity), "type {t}");
    }
}

/// A plat record (STATE_FORMAT "DSIM v35" and "DSIM v39"): `status` 0 up, 1
/// down, 2 waiting (`plat_e`; `in_stasis` needs a stop special, none in Episode 1
/// part 1); `count` never negative (`plat->count = plat->wait`, then counts down
/// to 0, p_plats.c); `type` 1 `downWaitUpStay` or 3 `raiseToNearestAndChange`;
/// for type 1 `low <= high` (`low` is at most and `high` at least the floor,
/// p_plats.c:209–214), and for type 3 `low` is never written and reads 0
/// (decision H7), whatever `high` is. Survivors `diagnostic.rs:1232:33`,
/// `1235:33`, `1237:54`, `1238:41`, `1238:82`, `1750:17`, `1752:45` and `1752:52`.
#[test]
fn plat_records_stay_in_their_domains() {
    let c = fixture("PLATS1_319");
    let plats = c.thinkers(6);
    let of_type = |t: u8| {
        plats
            .iter()
            .find(|p| c.get(&format!("{p}.type")) == [t])
            .cloned()
            .unwrap_or_else(|| panic!("a type-{t} plat"))
    };
    let (p1, p3) = (of_type(1), of_type(3));
    let field = |p: &str, f: &str| format!("{p}.{f}");
    // status
    assert_eq!(c.with(&field(&p1, "status"), [2]), Ok(()));
    assert_eq!(
        c.with(&field(&p1, "status"), [3]),
        Err(DiagnosticError::Identity)
    );
    // count
    assert_eq!(c.with(&field(&p1, "count"), 0i32.to_le_bytes()), Ok(()));
    assert_eq!(
        c.with(&field(&p1, "count"), (-1i32).to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
    // type 1: low <= high, and a nonzero low is fine.
    let high = c.i32(&field(&p1, "high"));
    assert_eq!(
        c.with(&field(&p1, "low"), (high - 8 * UNIT).to_le_bytes()),
        Ok(())
    );
    assert_eq!(c.with(&field(&p1, "low"), high.to_le_bytes()), Ok(()));
    assert_eq!(
        c.with(&field(&p1, "low"), (high + 1).to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
    // type 3: low reads 0, even above high.
    assert_eq!(c.i32(&field(&p3, "low")), 0);
    assert_eq!(
        c.with(&field(&p3, "high"), (-8 * UNIT).to_le_bytes()),
        Ok(())
    );
    let mut low8 = c.clone();
    low8.set(&field(&p3, "low"), (8 * UNIT).to_le_bytes())
        .set(&field(&p3, "high"), (64 * UNIT).to_le_bytes());
    assert_eq!(low8.decode(), Err(DiagnosticError::Identity));
    // type
    for t in [0u8, 2, 4, 255] {
        assert_eq!(
            c.with(&field(&p1, "type"), [t]),
            Err(DiagnosticError::Identity),
            "type {t}"
        );
    }
}

/// A floor record (STATE_FORMAT "DSIM v39"): `type` 0, 1, 2, 3, 4 or 11
/// (`floor_e`, p_spec.h); `direction` −1 or 1; `speed` positive (raw 16.16:
/// `FLOORSPEED` and its multiples, p_floor.c); `texture` (a normalized flat name)
/// and `newspecial` exactly on a `donutRaise` (11) record. Survivors
/// `diagnostic.rs:1242:56`, `1757:17`, `1762:25`, `1763:25` and `1764:25`.
#[test]
fn floor_records_stay_in_their_domains() {
    let c = fixture("FLOORS3_153");
    let floor = c.thinkers(7).into_iter().next().expect("a floor");
    let field = |f: &str| format!("{floor}.{f}");
    assert!(!c.has(&field("texture")) && !c.has(&field("newspecial")));
    for t in [0u8, 1, 2, 3, 4] {
        assert_eq!(c.with(&field("type"), [t]), Ok(()), "type {t}");
    }
    for t in [5u8, 6, 10, 12, 255] {
        assert_eq!(
            c.with(&field("type"), [t]),
            Err(DiagnosticError::Identity),
            "type {t}"
        );
    }
    for d in [-1i32, 1] {
        assert_eq!(
            c.with(&field("direction"), d.to_le_bytes()),
            Ok(()),
            "direction {d}"
        );
    }
    for d in [0i32, 2, -2] {
        assert_eq!(
            c.with(&field("direction"), d.to_le_bytes()),
            Err(DiagnosticError::Identity),
            "direction {d}"
        );
    }
    assert_eq!(c.with(&field("speed"), 1i32.to_le_bytes()), Ok(()));
    for v in [0i32, -1] {
        assert_eq!(
            c.with(&field("speed"), v.to_le_bytes()),
            Err(DiagnosticError::Identity),
            "speed {v}"
        );
    }
    // The donut pair: present exactly on type 11.
    let with_pair = |ty: u8, texture: Option<[u8; 8]>, newspecial: bool| {
        let mut d = c.clone();
        d.set(&field("type"), [ty]);
        let mut after = field("speed");
        if let Some(t) = texture {
            d.insert_after(&after, &field("texture"), t);
            after = field("texture");
        }
        if newspecial {
            d.insert_after(&after, &field("newspecial"), 0i16.to_le_bytes());
        }
        d.decode()
    };
    assert_eq!(with_pair(11, Some(*b"FLAT1\0\0\0"), true), Ok(()));
    assert_eq!(
        with_pair(11, Some(*b"FLAT1\0\0\0"), false),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(with_pair(11, None, true), Err(DiagnosticError::Identity));
    assert_eq!(with_pair(11, None, false), Err(DiagnosticError::Identity));
    assert_eq!(
        with_pair(4, Some(*b"FLAT1\0\0\0"), false),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(with_pair(4, None, true), Err(DiagnosticError::Identity));
    assert_eq!(
        with_pair(11, Some([0; 8]), true),
        Err(DiagnosticError::Structure)
    );
}

/// A sector's runtime special is the map's, or 0 for a found secret (map special
/// 9, `P_PlayerInSpecialSector`, p_spec.c:1051) and, under DSIM39, 0 after a
/// `raiseToNearestAndChange` plat or a finished donut ring (STATE_FORMAT "DSIM
/// v34": "every other runtime special is the map's, or 0 for a found secret";
/// p_plats.c:192, and `T_MoveFloor` writing a donut's `newspecial`, p_floor.c:231). Any other change is
/// refused, a secret sector included. Survivors `diagnostic.rs:1307:39` and
/// `1310:44`.
#[test]
fn sector_specials_change_only_to_zero() {
    let c = fixture("DOORS1_86");
    let map = c.get("map.sectors");
    assert_eq!(i16::from_le_bytes([map[22], map[23]]), 0);
    assert_eq!(c.with("sector.0.special", 0i16.to_le_bytes()), Ok(()));
    assert_eq!(
        c.with("sector.0.special", 7i16.to_le_bytes()),
        Err(DiagnosticError::Identity)
    );
    let mut secret = c.clone();
    let mut map = map.to_vec();
    map[22..24].copy_from_slice(&9i16.to_le_bytes());
    secret.set("map.sectors", map);
    for (special, want) in [
        (9i16, Ok(())),
        (0, Ok(())),
        (7, Err(DiagnosticError::Identity)),
    ] {
        assert_eq!(
            secret.with("sector.0.special", special.to_le_bytes()),
            want,
            "{special}"
        );
    }
}

/// A line's runtime special is its map special, or 0 for the once-only lines
/// (STATE_FORMAT "DSIM v35": "W1 and S1 lines read 0 once cleared"; DSIM39 adds
/// the Episode 1 W1, S1 and D1 lines, decision H1: D1 `EV_VerticalDoor` clears
/// 31–34, p_doors.c:477–491, and W1/S1 bodies clear theirs, p_spec.c,
/// p_switch.c:212). A repeatable line keeps its special: the DR blue-key door 26
/// (p_doors.c:448) and the GR door 46 are refused at 0; the W1 door 2 is
/// accepted. Survivor `diagnostic.rs:1345:21`.
#[test]
fn only_once_only_line_specials_clear() {
    let c = fixture("DOORS1_86");
    let line_special = |line: usize| {
        let map = c.get("map.linedefs");
        i16::from_le_bytes([map[14 * line + 6], map[14 * line + 7]])
    };
    for (line, special, cleared) in [
        (39, 2, Ok(())),
        (3, 26, Err(DiagnosticError::Identity)),
        (15, 46, Err(DiagnosticError::Identity)),
    ] {
        assert_eq!(line_special(line), special);
        let path = format!("line.{line}.special");
        assert_eq!(c.with(&path, special.to_le_bytes()), Ok(()));
        assert_eq!(c.with(&path, 0i16.to_le_bytes()), cleared, "{path}");
    }
}

/// A pressed button (`buttonlist`) names a line whose special restarts a button:
/// S1/SR switches with `useAgain` (p_switch.c:231–256: `P_StartButton` only when
/// the switch is reusable), that is SR 63 and, under DSIM39, SR 62 and 70 and the
/// GR 46 (STATE_FORMAT "DSIM v35" button fields; decision N9; DSIM39 decision
/// H1). PLATS1 presses its SR 62 at tic 59; the same slot moved to a line
/// without a special, or to the W1 line 22, is refused. Survivor
/// `diagnostic.rs:1381:44`.
#[test]
fn a_pressed_button_names_a_reusable_switch() {
    let c = &script("PLATS1", "plats", &[59])[0];
    let k = (0..16)
        .find(|k| c.i32(&format!("button.{k}.btimer")) != 0)
        .expect("a pressed button");
    let map = c.get("map.linedefs").to_vec();
    let sides = c.get("map.sidedefs").to_vec();
    let word = |bytes: &[u8], at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let line = usize::from(u16::from_le_bytes(
        c.get(&format!("button.{k}.line")).try_into().unwrap(),
    ));
    assert_eq!(word(&map, 14 * line + 6), 62);
    for (other, special) in [(0usize, 0u16), (22, 22)] {
        assert_eq!(word(&map, 14 * other + 6), special);
        let front = usize::from(word(&map, 14 * other + 10));
        let mut moved = c.clone();
        moved
            .set(&format!("button.{k}.line"), (other as u16).to_le_bytes())
            .set(
                &format!("button.{k}.soundorg"),
                word(&sides, 30 * front + 28).to_le_bytes(),
            );
        assert_eq!(
            moved.decode(),
            Err(DiagnosticError::Identity),
            "line {other}"
        );
    }
}

// --- Events ---------------------------------------------------------------------

/// Tag 21, the map statistic updates (STATE_FORMAT "DSIM-v17": sub-tags 0
/// available-kill update and 1 death credit, each "actor identity and signed
/// 32-bit updated total"; "DSIM-v27": 21/2 the `totalitems` increment at a
/// counted spawn). A total is never negative; the reader's lower boundary for
/// 21/0 is 0, and a 21/2 total is at least 1 (`totalitems++` precedes it,
/// p_mobj.c:792). Survivors `diagnostic.rs:729:30` (`<` with `==` and `<=`) and
/// `730:25`.
#[test]
fn statistic_totals_are_not_negative() {
    let c = &script("ITEMS1", "items", &[0])[0];
    let kills = c.event(&[21, 0]);
    let items = c.event(&[21, 2]);
    let total = |path: &str, value: i32| c.with_event(path, |e| put_i32(e, 14, value));
    assert_eq!(total(&kills, 0), Ok(()));
    assert_eq!(total(&kills, 1), Ok(()));
    assert_eq!(total(&kills, -1), Err(DiagnosticError::Identity));
    assert_eq!(total(&items, 1), Ok(()));
    assert_eq!(total(&items, 0), Err(DiagnosticError::Identity));
    assert_eq!(total(&items, -1), Err(DiagnosticError::Identity));
}

/// A state named by an event (here tag 12 sub-tag 1, the state entered, at
/// offset 14) is a state of an admitted actor type, with two exceptions:
/// `S_NULL`, the removal (DSIM v23 pistol impacts), and `S_GIBS`, a crunched
/// corpse (STATE_FORMAT "DSIM v35": giblets enter `S_GIBS`, p_map.c:1269–1279).
/// The Cyberdemon's `S_CYBER_STND` (info.c:810; no Cyberdemon in Episode 1 part
/// 1) is refused. Survivor `diagnostic.rs:809:60`.
#[test]
fn an_entered_state_belongs_to_an_admitted_type() {
    let c = &script("ITEMS1", "items", &[1])[0];
    let entered = c.event(&[12, 1]);
    let with_state = |s: u16| c.with_event(&entered, |e| put_u16(e, 14, s));
    assert_eq!(with_state(state("S_NULL")), Ok(()));
    assert_eq!(with_state(state("S_GIBS")), Ok(()));
    assert_eq!(with_state(state("S_PLAY")), Ok(()));
    assert_eq!(
        with_state(state("S_CYBER_STND")),
        Err(DiagnosticError::Identity)
    );
}

/// Tag 26 mover events (STATE_FORMAT "DSIM v35" sub-tag 6, the mover call:
/// functions 1 `EV_DoDoor`, 2 `EV_VerticalDoor`, 3 `EV_DoPlat`, 4 `EV_DoFloor`;
/// "DSIM v39": `BuildStairs` 5 and `DoDonut` 6 too; sub-tag 13, the exit: "the
/// exit event carries a secret flag for special 51", a u8 boolean,
/// `G_SecretExitLevel`, g_game.c). Survivors `diagnostic.rs:1887:9`, `1961:68`
/// and `1988:56`.
#[test]
fn mover_events_name_known_functions_and_a_boolean_exit() {
    let c = &script("DOORS1", "doors", &[47])[0];
    let call = c.event(&[26, 6]);
    for f in 1..=6u8 {
        assert_eq!(c.with_event(&call, |e| e[2] = f), Ok(()), "function {f}");
    }
    for f in [0u8, 7, 255] {
        assert_eq!(
            c.with_event(&call, |e| e[2] = f),
            Err(DiagnosticError::Identity),
            "function {f}"
        );
    }
    let c = &script("EXIT1", "exits", &[153])[0];
    let exit = c.event(&[26, 13]);
    for (flag, want) in [
        (0u8, Ok(())),
        (1, Ok(())),
        (2, Err(DiagnosticError::Identity)),
    ] {
        assert_eq!(c.with_event(&exit, |e| e[2] = flag), want, "secret {flag}");
    }
}

/// Tag 25 sub-tag 1, a missile removed against the sky (STATE_FORMAT "DSIM v34":
/// it names an occupied fireball; DSIM39: or a rocket; `P_XYMovement`
/// removes any missile blocked by a sky-ceiling line, p_mobj.c:173–184). ROCKET1's rocket at tic 755 is accepted; the same event naming the
/// player body is refused. Survivors `diagnostic.rs:2020:61`, `2020:67` and
/// `2020:73`.
#[test]
fn a_sky_removal_names_a_fireball_or_a_rocket() {
    let c = &script("ROCKET1", "weapons", &[755])[0];
    let removal = c.event(&[25, 1]);
    let slot = u32::from_le_bytes(c.get(&removal)[2..6].try_into().unwrap());
    assert_eq!(
        c.get(&format!("actor.{slot}.type")),
        actor_type("MT_ROCKET").ordinal().to_le_bytes()
    );
    let body = c.get("original_player.0.mo").to_vec();
    assert_eq!(body[0], 1);
    assert_eq!(
        c.with_event(&removal, |e| e[2..14].copy_from_slice(&body[1..13])),
        Err(DiagnosticError::Identity)
    );
}

/// Tag 14 weapon events under wire 23 (STATE_FORMAT "DSIM-v24": sub-tag 2 Ammo
/// "slot u8, ammo u8, before/after i32", the ammunition of the firing weapon
/// decremented by one, `P_FireWeapon`/`A_Fire*`, p_pspr.c; sub-tag 4 Fire "slot
/// u8, weapon u8, refire i32"; "DSIM v39": the rocket launcher and the chainsaw
/// (weapons 4 and 7), rocket ammunition (3) and the doubled maxima). No admitted
/// weapon uses cells (2: the plasma rifle and the BFG are never owned), slot 0
/// is the only player, and the widths are 12 and 8 bytes. Survivors
/// `diagnostic.rs:2626:5`, `2691:16`, `2692:17`, `2693:17`, `2693:44`,
/// `2693:75`, `2698:78`, `2699:44`, `2713:16`, `2714:17`, `2715:17`,
/// `2716:30` and `2717:34`.
#[test]
fn weapon_ammo_and_fire_events_stay_in_the_episode1_domain() {
    let caps = script("ROCKET1", "weapons", &[203, 211]);
    let c = &caps[1];
    let ammo = c.event(&[14, 2]);
    assert_eq!(c.get(&ammo)[3], 3, "the rocket ammunition");
    let used = |kind: u8, before: i32, after: i32| {
        c.with_event(&ammo, |e| {
            e[3] = kind;
            put_i32(e, 4, before);
            put_i32(e, 8, after);
        })
    };
    // Rockets up to the doubled maximum 100 (p_inter.c:590–594).
    assert_eq!(used(3, 100, 99), Ok(()));
    assert_eq!(used(3, 1, 0), Ok(()));
    assert_eq!(used(3, 101, 100), Err(DiagnosticError::Identity));
    assert_eq!(used(3, 0, -1), Err(DiagnosticError::Identity));
    assert_eq!(used(3, 5, 5), Err(DiagnosticError::Identity));
    assert_eq!(used(0, 400, 399), Ok(()));
    assert_eq!(used(1, 100, 99), Ok(()));
    assert_eq!(used(2, 5, 4), Err(DiagnosticError::Structure));
    assert_eq!(used(4, 5, 4), Err(DiagnosticError::Structure));
    assert_eq!(
        c.with_event(&ammo, |e| e[2] = 1),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        c.with_event(&ammo, |e| e.push(0)),
        Err(DiagnosticError::Structure)
    );

    let c = &caps[0];
    let fire = c.event(&[14, 4]);
    for w in [0u8, 1, 2, 3, 4, 7] {
        assert_eq!(c.with_event(&fire, |e| e[3] = w), Ok(()), "weapon {w}");
    }
    for w in [5u8, 6, 8, 10] {
        assert_eq!(
            c.with_event(&fire, |e| e[3] = w),
            Err(DiagnosticError::Structure),
            "weapon {w}"
        );
    }
    assert_eq!(
        c.with_event(&fire, |e| e[2] = 1),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        c.with_event(&fire, |e| e.push(0)),
        Err(DiagnosticError::Structure)
    );
}

/// The chainsaw's psprite states and action exist only under wire 23: a tag-14
/// Entered event with `S_SAW1` (info.c:207, action `A_Saw`) and the Action event
/// that follows it in SAW1 are accepted by the DSIM39 reader and refused by the
/// DSIM38 reader (STATE_FORMAT "DSIM-v24": "Version24/policy8 admits the four
/// named weapon state/action closures"; "DSIM v39" adds the launcher's and the
/// chainsaw's). Survivors `diagnostic.rs:2642:29` and `2672:29`.
#[test]
fn chainsaw_weapon_events_need_wire_23() {
    let saw1 = state("S_SAW1");
    assert_eq!(
        OriginalState::from_name("S_SAW1")
            .unwrap()
            .descriptor()
            .action,
        Some("A_Saw")
    );
    let entered = [14, 0, 0, 0, saw1.to_le_bytes()[0], saw1.to_le_bytes()[1]];
    let captures = script("SAW1", "weapons", &[69, 70, 71, 72, 73, 74, 75, 76, 77]);
    let (c, action) = captures
        .iter()
        .find_map(|c| {
            let events: Vec<&Vec<u8>> = c
                .fields
                .iter()
                .filter(|(p, _)| p.starts_with("events.last.") && !p.ends_with(".count"))
                .map(|(_, v)| v)
                .collect();
            events
                .windows(2)
                .find(|w| w[0][..] == entered && w[1].starts_with(&[14, 1, 0, 0]))
                .map(|w| (c, w[1].clone()))
        })
        .expect("S_SAW1 entered, then its action");
    assert_eq!(c.decode(), Ok(()));
    let level = level_fixture();
    assert_eq!(level.with_pushed(&entered), Err(DiagnosticError::Identity));
    assert_eq!(level.with_pushed(&action), Err(DiagnosticError::Identity));
    // The pistol's are admitted by both (info.c S_PISTOL1, action A_FirePistol).
    let pistol = state("S_PISTOL");
    assert_eq!(
        level.with_pushed(&[
            14,
            0,
            0,
            0,
            pistol.to_le_bytes()[0],
            pistol.to_le_bytes()[1]
        ]),
        Ok(())
    );
}

/// Tag 15 sub-tag 5, a missile spawn: DSIM39 adds `P_SpawnPlayerMissile`'s
/// rocket, 17 bytes with aim kind 1 (no destination) and type `MT_ROCKET`
/// (p_mobj.c:935–987, STATE_FORMAT "DSIM v39"); the Imp's fireball keeps its
/// 29-byte form with aim kind 0 ("DSIM-v29"). ROCKET1's rocket spawn is accepted
/// by the DSIM39 reader and refused by the DSIM38 reader; with aim kind 0 or
/// another type it is refused. Survivors `diagnostic.rs:2764:5`, `2767:26` and
/// `2767:35`.
#[test]
fn a_rocket_spawn_is_a_wire_23_player_missile() {
    let c = &script("ROCKET1", "weapons", &[211])[0];
    let spawn = c.event(&[15, 5]);
    let event = c.get(&spawn).to_vec();
    assert_eq!(event.len(), 17);
    assert_eq!(event[14], 1);
    assert_eq!(
        c.with_event(&spawn, |e| e[14] = 0),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        c.with_event(&spawn, |e| put_u16(
            e,
            15,
            actor_type("MT_TROOPSHOT").ordinal()
        )),
        Err(DiagnosticError::Identity)
    );
    assert_eq!(
        level_fixture().with_pushed(&event),
        Err(DiagnosticError::Structure)
    );
}

/// Tag 23 pickup events under wire 23 (STATE_FORMAT "DSIM-v25", "DSIM-v26",
/// "DSIM-v27", "DSIM v39"), in ITEMS1: Touch (sub-tag 0) names the sprite of an
/// admitted item; Ammo (1) is 14 bytes, ammunition 0–3, `accepted` boolean,
/// before and after within the maximum (doubled by a backpack, p_inter.c:87–108,
/// 590–594), then the ready weapon (0–4 or 7) and the pending one (also 10,
/// `wp_nochange`); Weapon (3) gives the shotgun, chaingun, launcher or chainsaw
/// (2, 3, 4, 7; p_inter.c `P_GiveWeapon`); Message (4) is a pickup message,
/// codes 1–6, 8–13 and 14–29 (code 7 `GOTMEDINEED` is unreachable, 0 is no
/// message and 30 the turbo message, never a pickup's); Bonus (5) has a
/// nonnegative `bonuscount`. Survivors `diagnostic.rs:2967:28`, `2967:58`,
/// `3032:21`, `3052:17`, `3052:53`, `3053:17`, `3054:17`, `3055:17`, `3061:25`,
/// `3061:46`, `3081:17`, `3081:65`, `3082:17`–`3086:17`, `3091:31`, `3092:33`,
/// `3093:39`, `3097:17` and `3099:17`.
#[test]
fn ammo_weapon_message_and_bonus_pickup_events_stay_in_their_domains() {
    let c = &script("ITEMS1", "items", &[74])[0];
    // Touch: the sprite of an item this policy admits.
    let touch = c.event(&[23, 0]);
    for s in ["SPR_LAUN", "SPR_BKEY", "SPR_CSAW", "SPR_BON1", "SPR_CLIP"] {
        assert_eq!(
            c.with_event(&touch, |e| put_u16(e, 26, sprite(s))),
            Ok(()),
            "{s}"
        );
    }
    for s in ["SPR_TROO", "SPR_PLAS", "SPR_MEGA"] {
        assert_eq!(
            c.with_event(&touch, |e| put_u16(e, 26, sprite(s))),
            Err(DiagnosticError::Structure),
            "{s}"
        );
    }
    // Ammo.
    let ammo = c.event(&[23, 1]);
    assert_eq!(c.get(&ammo).len(), 14);
    let weapons = |ready: u8, pending: u8| {
        c.with_event(&ammo, |e| {
            e[12] = ready;
            e[13] = pending;
        })
    };
    for (ready, pending) in [(0u8, 10u8), (4, 10), (7, 10), (1, 4), (1, 7), (3, 2)] {
        assert_eq!(weapons(ready, pending), Ok(()), "{ready} {pending}");
    }
    for (ready, pending) in [(10u8, 10u8), (5, 10), (6, 10), (1, 5), (1, 6), (1, 11)] {
        assert_eq!(
            weapons(ready, pending),
            Err(DiagnosticError::Structure),
            "{ready} {pending}"
        );
    }
    assert_eq!(
        c.with_event(&ammo, |e| e.push(0)),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        c.with_event(&ammo, |e| e[2] = 4),
        Err(DiagnosticError::Structure)
    );
    assert_eq!(
        c.with_event(&ammo, |e| e[3] = 2),
        Err(DiagnosticError::Structure)
    );
    let amounts = |kind: u8, before: i32, after: i32| {
        c.with_event(&ammo, |e| {
            e[2] = kind;
            put_i32(e, 4, before);
            put_i32(e, 8, after);
        })
    };
    for (kind, max) in [(0u8, 400), (1, 100), (2, 600), (3, 100)] {
        assert_eq!(amounts(kind, max - 10, max), Ok(()), "{kind}");
        assert_eq!(
            amounts(kind, max - 10, max + 1),
            Err(DiagnosticError::Structure),
            "{kind}"
        );
    }
    // Weapon.
    let weapon = c.event(&[23, 3]);
    for w in [2u8, 3, 4, 7] {
        assert_eq!(c.with_event(&weapon, |e| e[2] = w), Ok(()), "weapon {w}");
    }
    for w in [0u8, 1, 5, 6, 8] {
        assert_eq!(
            c.with_event(&weapon, |e| e[2] = w),
            Err(DiagnosticError::Structure),
            "weapon {w}"
        );
    }
    // Message.
    let message = c.event(&[23, 4]);
    for m in [1u8, 6, 8, 11, 13, 14, 29] {
        assert_eq!(c.with_event(&message, |e| e[2] = m), Ok(()), "message {m}");
    }
    for m in [0u8, 7, 30, 31, 255] {
        assert_eq!(
            c.with_event(&message, |e| e[2] = m),
            Err(DiagnosticError::Structure),
            "message {m}"
        );
    }
    // Bonus.
    let bonus = c.event(&[23, 5]);
    assert_eq!(c.with_event(&bonus, |e| put_i32(e, 2, 0)), Ok(()));
    assert_eq!(
        c.with_event(&bonus, |e| put_i32(e, 2, -1)),
        Err(DiagnosticError::Structure)
    );
}

/// The DSIM39 pickup sub-tags (STATE_FORMAT "DSIM v39" events table): 11 key card
/// touched, card 0–2 (the skull keys 3–5 are never placed in Episode 1 part 1)
/// and a boolean "held before" (`P_GiveCard`, p_inter.c:274–283); 12 power
/// touched, power 2–5 (no invulnerability or berserk: `powers.0`/`.1` are 0),
/// `accepted` boolean, counters before and after within the power's domain
/// (`INVISTICS`/`IRONTICS` 2,100, the map 1, `INFRATICS` 4,200; doomdef.h:
/// 234–238) and unchanged when not accepted; 13 soulsphere, health before
/// 1–200 and after `min(before + 100, 200)` = the body's (p_inter.c:400–404);
/// 14 backpack, a boolean "first". Survivors `diagnostic.rs:3114:55`,
/// `3114:72`, `3116:31`, `3116:60`, `3116:77`, `3127:21`, `3128:21`,
/// `3132:31`, `3136:45`, `3136:65`, `3136:81` and `3139:55`.
#[test]
fn key_power_soulsphere_and_backpack_events_stay_in_their_domains() {
    let caps = script("ITEMS1", "items", &[121, 146, 154, 187]);
    let (backpack, soul, power, key) = (&caps[0], &caps[1], &caps[2], &caps[3]);
    // Key card.
    let card = key.event(&[23, 11]);
    for (k, held) in [(0u8, 0u8), (2, 1)] {
        assert_eq!(
            key.with_event(&card, |e| (e[2], e[3]) = (k, held)),
            Ok(()),
            "{k} {held}"
        );
    }
    for (k, held) in [(3u8, 0u8), (5, 0), (0, 2)] {
        assert_eq!(
            key.with_event(&card, |e| (e[2], e[3]) = (k, held)),
            Err(DiagnosticError::Structure),
            "{k} {held}"
        );
    }
    // Power.
    let touched = power.event(&[23, 12]);
    let given = |p: u8, accepted: u8, before: i32, after: i32| {
        power.with_event(&touched, |e| {
            (e[2], e[3]) = (p, accepted);
            put_i32(e, 4, before);
            put_i32(e, 8, after);
        })
    };
    for (p, max) in [(2u8, 2100), (3, 2100), (4, 1), (5, 4200)] {
        assert_eq!(given(p, 1, 0, max), Ok(()), "power {p}");
        assert_eq!(given(p, 1, max, max), Ok(()), "power {p}");
        assert_eq!(given(p, 0, max, max), Ok(()), "power {p}");
        assert_eq!(
            given(p, 1, 0, max + 1),
            Err(DiagnosticError::Structure),
            "power {p}"
        );
        assert_eq!(
            given(p, 1, max + 1, max),
            Err(DiagnosticError::Structure),
            "power {p}"
        );
        assert_eq!(
            given(p, 0, 0, max),
            Err(DiagnosticError::Structure),
            "power {p}"
        );
    }
    for p in [0u8, 1, 6] {
        assert_eq!(
            given(p, 1, 0, 1),
            Err(DiagnosticError::Structure),
            "power {p}"
        );
    }
    // Soulsphere.
    let sphere = soul.event(&[23, 13]);
    let health = |before: i32, after: i32, body: i32| {
        soul.with_event(&sphere, |e| {
            put_i32(e, 2, before);
            put_i32(e, 6, after);
            put_i32(e, 10, body);
        })
    };
    assert_eq!(health(50, 150, 150), Ok(()));
    assert_eq!(health(150, 200, 200), Ok(()));
    assert_eq!(health(200, 200, 200), Ok(()));
    assert_eq!(health(1, 101, 101), Ok(()));
    assert_eq!(health(0, 100, 100), Err(DiagnosticError::Structure));
    assert_eq!(health(201, 200, 200), Err(DiagnosticError::Structure));
    assert_eq!(health(50, 150, 149), Err(DiagnosticError::Structure));
    assert_eq!(
        soul.with_event(&sphere, |e| e.push(0)),
        Err(DiagnosticError::Structure)
    );
    // Backpack.
    let first = backpack.event(&[23, 14]);
    for (f, want) in [
        (0u8, Ok(())),
        (1, Ok(())),
        (2, Err(DiagnosticError::Structure)),
    ] {
        assert_eq!(backpack.with_event(&first, |e| e[2] = f), want, "first {f}");
    }
}

/// The DSIM39 pickup sub-tags 11–14 are refused by the DSIM38 reader (frozen
/// reader rule, STATE_FORMAT "DSIM v39"; "Pickup tag 23 gains four sub-tags"):
/// each of ITEMS1's real events is accepted in its own capture and refused when
/// appended to the DSIM38 load capture. Survivors `diagnostic.rs:3114:21`,
/// `3115:21`, `3131:21` and `3139:21`.
#[test]
fn episode1_pickup_sub_tags_need_dsim39() {
    let caps = script("ITEMS1", "items", &[121, 146, 154, 187]);
    let level = level_fixture();
    for (c, sub) in [
        (&caps[3], 11u8),
        (&caps[2], 12),
        (&caps[1], 13),
        (&caps[0], 14),
    ] {
        let event = c.get(&c.event(&[23, sub])).to_vec();
        assert_eq!(c.decode(), Ok(()));
        assert_eq!(
            level.with_pushed(&event),
            Err(DiagnosticError::Structure),
            "sub-tag {sub}"
        );
    }
}
