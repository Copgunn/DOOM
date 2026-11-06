//! Per-map bounds of the Episode 1 policy (`PlayerMonstersEpisode1V1`, M4a
//! decision H4).
//!
//! Every older map policy fixed its roster, pickup, decoration and barrel bounds
//! as constants and proved from them that the actor store never fills. Episode 1
//! maps hold 41 to 118 monsters, so wire 23 derives each bound once at load from
//! the map's own THINGS lump, counted with the filter `P_SpawnMapThing` applies
//! in single player at the medium skill (p_mobj.c:708–797: player and deathmatch
//! starts are not spawned as things; `MTF_NOTSINGLE` (16) is skipped; the medium
//! skill bit (2) is required), and classified by DoomEd number. A spawn site that
//! finds the 1,024-slot store full refuses the tic with `StoreError::Capacity`
//! instead (the store-full witness). The bounds are immutable, never canonical
//! state, and the DSIM39 reader derives the same values from `map.things`
//! (`Episode1Bounds::from_thing_records`).
use crate::identity::OriginalActorType;
use doom_map::{Map, Thing};

/// DoomEd numbers of the monsters an Episode 1 map may place: the former human,
/// the sergeant, the chaingunner, the Imp, the Demon and the Spectre (info.c).
pub(crate) const EPISODE1_MONSTERS: [i16; 6] = [3004, 9, 65, 3001, 3002, 58];
/// The monsters among them whose `P_KillMobj` drops an item (p_inter.c:704–722):
/// the former human (clip), the sergeant (shotgun) and the chaingunner (chaingun).
pub(crate) const EPISODE1_DROPPERS: [i16; 3] = [3004, 9, 65];
/// DoomEd numbers of every Episode 1 pickup (info.c): ammunition, health, armor,
/// the six weapons and power-ups of part 1, the three keycards and the backpack.
pub(crate) const EPISODE1_PICKUPS: [i16; 25] = [
    2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015, // wire 22
    5, 6, 13, // blue, yellow and red keycard
    2013, 2024, 2025, 2026, 2045, // soulsphere, invisibility, suit, map, visor
    8, 2010, 2046, 2005, 2003, // backpack, rocket, rocket box, chainsaw, launcher
];
/// DoomEd numbers of the keycards (`P_GiveCard` adds a second `BONUSADD`,
/// p_inter.c:274–283).
pub(crate) const EPISODE1_KEYCARDS: [i16; 3] = [5, 6, 13];
/// DoomEd number of the exploding barrel (info.c `MT_BARREL`).
pub(crate) const EPISODE1_BARREL: i16 = 2035;
/// DoomEd numbers of the static things beyond E1M1's seven decorations: the candle
/// (`MT_MISC49`, 34) and the firestick (`MT_MISC43`, 46).
pub(crate) const EPISODE1_EXTRA_STATICS: [i16; 2] = [34, 46];
/// `BONUSADD` (p_inter.c): the `bonuscount` one pickup adds.
const BONUS_ADD: usize = 6;

/// The per-map bounds of the Episode 1 policy, counted from a THINGS lump with
/// the single-player, medium-skill `P_SpawnMapThing` filter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Episode1Bounds {
    /// `K`: placed monsters.
    pub monsters: usize,
    /// `R`: placed monsters that drop an item when killed (each drops once).
    pub droppers: usize,
    /// `P`: placed pickups, keycards included.
    pub pickups: usize,
    /// Placed keycards (a subset of `pickups`).
    pub keycards: usize,
    /// `B`: placed exploding barrels.
    pub barrels: usize,
    /// `D`: placed static decorations.
    pub statics: usize,
    /// Placed pickups whose definition carries `MF_COUNTITEM` (the source of
    /// `totalitems`).
    pub countitems: usize,
}
impl Episode1Bounds {
    /// Count the bounds of a loaded map.
    pub fn from_map(map: &Map) -> Self {
        Self::from_things(map.things().iter().map(|t: &Thing| (t.kind, t.options)))
    }
    /// Count the bounds of a raw THINGS lump in the DSIM `map.things` layout (ten
    /// bytes per record: x, y, angle, type and options as little-endian `i16`);
    /// `None` when the length is not a whole number of records.
    pub fn from_thing_records(records: &[u8]) -> Option<Self> {
        if records.len() % 10 != 0 {
            return None;
        }
        Some(Self::from_things(records.chunks_exact(10).map(|r| {
            (
                i16::from_le_bytes([r[6], r[7]]),
                u16::from_le_bytes([r[8], r[9]]),
            )
        })))
    }
    fn from_things(things: impl IntoIterator<Item = (i16, u16)>) -> Self {
        let mut b = Self::default();
        for (kind, options) in things {
            if (1..=4).contains(&kind) || kind == 11 || options & 16 != 0 || options & 2 == 0 {
                continue;
            }
            if EPISODE1_MONSTERS.contains(&kind) {
                b.monsters += 1;
                b.droppers += usize::from(EPISODE1_DROPPERS.contains(&kind));
            } else if EPISODE1_PICKUPS.contains(&kind) {
                b.pickups += 1;
                b.keycards += usize::from(EPISODE1_KEYCARDS.contains(&kind));
                b.countitems += usize::from(counts_item(kind));
            } else if kind == EPISODE1_BARREL {
                b.barrels += 1;
            } else if is_static(kind) {
                b.statics += 1;
            }
        }
        b
    }
    /// Live world items: the placed pickups plus one drop per dropping monster.
    pub const fn world_items(&self) -> usize {
        self.pickups + self.droppers
    }
    /// The `bonuscount` bound: `BONUSADD` per pickup and drop, and a second one
    /// per keycard.
    pub const fn max_bonus(&self) -> usize {
        BONUS_ADD * (self.pickups + self.droppers + self.keycards)
    }
    /// The per-operation monster sight-query allowance of decision D8:
    /// `8·(K + 1)`, eight per monster and one more share, and never below the
    /// older policies' constant 64, so that every workload an older policy admits
    /// is admitted (a Rust-only work witness, not source behaviour).
    pub const fn sight_queries(&self) -> usize {
        let queries = 8 * (self.monsters + 1);
        if queries > 64 { queries } else { 64 }
    }
    /// The sight queries one explosion may make (decision F6 with per-map values):
    /// the player, every monster and every barrel (a rocket's window may hold all
    /// of them), each revisited once per relinking monster: `(1 + K + B)·(1 + K)`,
    /// and never below the barrels policy's 216 (`EXPLOSION_SIGHT_QUERIES`).
    pub const fn explosion_sight_queries(&self) -> usize {
        let queries = (1 + self.monsters + self.barrels) * (1 + self.monsters);
        if queries > crate::EXPLOSION_SIGHT_QUERIES {
            queries
        } else {
            crate::EXPLOSION_SIGHT_QUERIES
        }
    }
    /// The roster term of the explosion's defensive visit bound
    /// (`9·order·(K + 2)`): `K`, and never below the older policies' 8.
    pub const fn visit_roster(&self) -> usize {
        if self.monsters > crate::MAX_PLAYER_MONSTERS {
            self.monsters
        } else {
            crate::MAX_PLAYER_MONSTERS
        }
    }
}
fn counts_item(kind: i16) -> bool {
    OriginalActorType::all().any(|ty| {
        let d = ty.descriptor();
        d.doomed_number == i32::from(kind) && d.flags & crate::world_pickups::MF_COUNTITEM != 0
    })
}
fn is_static(kind: i16) -> bool {
    EPISODE1_EXTRA_STATICS.contains(&kind)
        || crate::E1M1_DECORATIONS
            .iter()
            .any(|ty| ty.descriptor().doomed_number == i32::from(kind))
}

impl crate::Simulation {
    /// Copy of the source `powers` and `cards` of the Episode 1 policy; no
    /// mutable player alias escapes. `PlayerBodyError::Profile` under every other
    /// policy.
    pub fn original_player_powers(
        &self,
        _slot: crate::PlayerSlot,
    ) -> Result<crate::PlayerPowers, crate::PlayerBodyError> {
        self.original_player
            .as_ref()
            .and_then(|p| p.powers)
            .ok_or(crate::PlayerBodyError::Profile)
    }
    /// The per-map bounds of the Episode 1 policy, `None` under every other
    /// policy.
    pub fn episode1_bounds(&self) -> Option<Episode1Bounds> {
        self.world.episode1
    }
    /// Sector `i`'s runtime `floorpic` as DSIM39 encodes it (decision D2): the
    /// normalized name of its floor flat, changed by the Episode 1 movers.
    pub(crate) fn floorpic(&self, i: usize) -> [u8; 8] {
        self.specials
            .as_ref()
            .and_then(|s| s.sectors.get(i))
            .map_or_else(
                || crate::sector_thinkers::normalized_flat(&self.world.map().sectors()[i].flats[0]),
                |s| s.floorpic,
            )
    }
    /// The decorations one map may spawn: `D` under the Episode 1 policy,
    /// [`crate::MAX_DECORATIONS`] under every other one.
    pub(crate) fn decoration_limit(&self) -> usize {
        self.world
            .episode1
            .map_or(crate::MAX_DECORATIONS, |b| b.statics)
    }
    /// The exploding barrels one map may spawn: `B` under the Episode 1 policy,
    /// [`crate::MAX_BARRELS`] under every other one.
    pub(crate) fn barrel_limit(&self) -> usize {
        self.world
            .episode1
            .map_or(crate::MAX_BARRELS, |b| b.barrels)
    }
}
