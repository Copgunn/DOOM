//! The E1M1 policy's static decorations (M3h9 note §2.5–§2.7 and §5.1, decision
//! E2): the explicit seven-type list, the static predicate every listed type
//! satisfies, the per-map bound, the live-actor witness and the linked-thing
//! budget. Admission is the list; the predicate records why no body beyond the
//! existing ones is needed. Every failure is a field-less error.
use crate::identity::{OriginalActorType, OriginalState};
use crate::original_player::{MF_NOGRAVITY, MF_SHOOTABLE, MF_SOLID};
use crate::world_pickups::{MF_AMBUSH, MF_COUNTITEM, MF_COUNTKILL, MF_SPECIAL};
use crate::{
    Actor, ActorError, ActorTypeId, MAX_PLAYER_MONSTERS, MAX_WORLD_PICKUPS,
    OriginalExecutionProfile, PlayerBodyError, SimError, Solidity,
};
use doom_types::Fixed;

/// p_mobj.h `MF_NOSECTOR`: not linked to the sector thing list.
pub(crate) const MF_NOSECTOR: u32 = 8;
/// p_mobj.h `MF_NOBLOCKMAP`: not linked to the BLOCKMAP.
pub(crate) const MF_NOBLOCKMAP: u32 = 16;
/// p_mobj.h `MF_SPAWNCEILING`: `P_SpawnMapThing` spawns it at `ONCEILINGZ`.
pub(crate) const MF_SPAWNCEILING: u32 = 256;

/// The seven static decoration types of E1M1 (info.c; M3h9 note §2.5, decision
/// E2), by `mobjinfo` entry:
///
/// | DoomEd | type (info.c line) | spawn state (line) | radius / height | flags (line) |
/// | --- | --- | --- | --- | --- |
/// | 2028 | `MT_MISC31`, floor lamp (3215) | `S_COLU` (1022) | 16 / 16 | `MF_SOLID` (3236) |
/// | 48 | `MT_MISC48`, tall techno pillar (3657) | `S_TECHPILLAR` (1052) | 16 / 16 | `MF_SOLID` (3678) |
/// | 35 | `MT_MISC50`, candelabra (3709) | `S_CANDELABRA` (1048) | 16 / 16 | `MF_SOLID` (3730) |
/// | 24 | `MT_MISC71`, pool of blood and flesh (4255) | `S_GIBS` (1031) | 20 / 16 | 0 (4276) |
/// | 15 | `MT_MISC62`, dead player (4021) | `S_PLAY_DIE7` (300) | 20 / 16 | 0 (4042) |
/// | 10 | `MT_MISC68`, bloody mess (4177) | `S_PLAY_XDIE9` (309) | 20 / 16 | 0 (4198) |
/// | 12 | `MT_MISC69`, bloody mess (4203) | `S_PLAY_XDIE9` (309) | 20 / 16 | 0 (4224) |
///
/// Every spawn state has `tics` −1 and no action, every spawn health is 1000 and
/// none is `MF_SHOOTABLE`, `MF_SPECIAL`, `MF_COUNTKILL`, `MF_COUNTITEM`,
/// `MF_NOBLOCKMAP`, `MF_NOSECTOR`, `MF_SPAWNCEILING` or `MF_NOGRAVITY`
/// ([`static_decoration`]). Admission is this list under wire 20 alone; a later
/// policy that admits every static type (decision E2 option ii) is a list change
/// with its own evidence (hanging corpses are `MF_SPAWNCEILING|MF_NOGRAVITY`, a
/// new height path).
pub const E1M1_DECORATIONS: [OriginalActorType; 7] = [
    OriginalActorType::MT_MISC31,
    OriginalActorType::MT_MISC48,
    OriginalActorType::MT_MISC50,
    OriginalActorType::MT_MISC71,
    OriginalActorType::MT_MISC62,
    OriginalActorType::MT_MISC68,
    OriginalActorType::MT_MISC69,
];
/// The most decorations one map may spawn under the E1M1 policy (E1M1 spawns 27 on
/// every skill, M3h9 note §3); the next is `MapSpawnError::DecorationLimit` and the
/// batch spawns nothing.
pub const MAX_DECORATIONS: usize = 32;
/// The linked things `N` of the E1M1 policy (M3h9 note §5.3): the player, the
/// monsters, the pickups, the drops and the decorations, every one BLOCKMAP- and
/// sector-linked (none of the seven types is `MF_NOBLOCKMAP` or `MF_NOSECTOR`).
/// The §5.3 budgets this value feeds, with the values of this policy
/// (`D` = decorations ≤ 32, `K` = monsters ≤ 8, `P` = pickups ≤ 16, `R` = drops
/// ≤ 8, `k` = Imps ≤ `K`):
///
/// | Budget | Formula | Value | Store or certificate |
/// | --- | --- | --- | --- |
/// | Linked things `N` | `1 + K + P + R + D` | 65 (was 33) | `P_ChangeSector` work, intercept reasoning |
/// | Live actors | `1 + K + P + 26k + (K−k) + 8(K−k) + R + D` | ≤ 273 at `k = 8` | `MAP_SCALE_ACTOR_SLOTS` 320 |
/// | Spawns in one pass | `39 − 3k` | ≤ 39 | 273 + 15 = 288 ≤ 320 at `k = 8`; 241 + 39 = 280 at `k = 0` |
/// | Thinker visits per pass | `320 + L + 2M` | unchanged | the visit budget (decorations are actors inside the 320) |
/// | Sector-thinker slots | `L + 2M` | unchanged | `sector_thinkers.slot_count` |
/// | `P_ChangeSector` per tic | `Σ_movers 4·(B(s) + N·Q)` | `N` 33 → 65 | re-evaluated, not re-proved |
/// | `spechit` | ≤ 8 per position check, every radius < 64 | radii 16, 20 | rule 6 unchanged |
/// | Hitscan intercepts | window lines + things in those cells ≤ 128 | 28 + 65 = 93 (E1M1 window 28) | the intercept witness stays unreachable under this crude bound |
/// | Slide intercepts | window lines ≤ 128 | 28 | unchanged (lines only) |
/// | Sight queries | 64 per operation, monsters only | unchanged | decorations are never queried |
/// | Scrollers | ≤ 64 | 8 | rule 5 unchanged |
/// | Blockmap links, sector thing lists | `N` per cell at most | 65 | linked lists, no fixed-capacity store |
/// | `MAX_BONUS` | `6 × P` | 96 | unchanged |
///
/// No fixed-capacity store grows and no arithmetic certificate has a roster term,
/// so the older policies' private `MAX_LINKED_THINGS` (33) and its 95-linedef
/// static assert are left as they are: this constant is the E1M1 value alone.
pub const E1M1_LINKED_THINGS: usize =
    1 + MAX_PLAYER_MONSTERS + MAX_WORLD_PICKUPS + MAX_PLAYER_MONSTERS + MAX_DECORATIONS;
const _: () = assert!(E1M1_LINKED_THINGS == 65);

/// The listed decoration type of `ty`, policy-free (the policy gate is
/// [`is_decoration`]).
pub(crate) fn decoration(ty: ActorTypeId) -> Option<OriginalActorType> {
    E1M1_DECORATIONS.into_iter().find(|d| d.ordinal() == ty.0)
}
/// Whether `ty` is an admitted decoration under `policy`: one of
/// [`E1M1_DECORATIONS`] and the policy has `has_e1m1`.
pub(crate) fn is_decoration(policy: OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    policy.has_e1m1() && decoration(ty).is_some()
}
/// The static predicate every listed type satisfies (M3h9 note §2.5, decision E2):
/// a DoomEd number, spawn state with `tics` −1 and no action (so `P_MobjThinker`
/// never cycles it, p_mobj.c:441–449), flags 0 or `MF_SOLID` alone (so
/// `PIT_CheckThing` either passes it at 258–259 or blocks it at 345 and the missile
/// branch at 304–305, and `PIT_ChangeSector` returns at 1290–1294), none of
/// `MF_SHOOTABLE` (never damaged, thrust or gibbed), `MF_SPECIAL` (never touched),
/// `MF_COUNTKILL`/`MF_COUNTITEM` (no counter), `MF_NOBLOCKMAP`/`MF_NOSECTOR` (linked
/// like any thing) or `MF_SPAWNCEILING`/`MF_NOGRAVITY` (spawned at `ONFLOORZ`,
/// p_mobj.c:770–777), spawn health 1000, radius 16 or 20 and height 16.
pub(crate) fn static_decoration(ty: OriginalActorType) -> bool {
    let d = ty.descriptor();
    let s = d.states.spawn.descriptor();
    d.doomed_number > 0
        && s.tics == -1
        && s.action.is_none()
        && d.flags & !MF_SOLID == 0
        && d.flags
            & (MF_SHOOTABLE
                | MF_SPECIAL
                | MF_COUNTKILL
                | MF_COUNTITEM
                | MF_NOBLOCKMAP
                | MF_NOSECTOR
                | MF_SPAWNCEILING
                | MF_NOGRAVITY)
            == 0
        && d.spawn_health == 1000
        && (d.radius == 16 * 65536 || d.radius == 20 * 65536)
        && d.height == 16 * 65536
}
/// `ty` is a listed decoration whose compile-time definition satisfies
/// [`static_decoration`]; computed once per type ordinal.
pub(crate) fn validate_definition(ty: ActorTypeId) -> Result<(), ActorError> {
    static MEMO: [std::sync::OnceLock<Result<(), ActorError>>; 256] =
        [const { std::sync::OnceLock::new() }; 256];
    let compute = || {
        decoration(ty)
            .filter(|d| static_decoration(*d))
            .map(|_| ())
            .ok_or(ActorError::InvalidType)
    };
    match MEMO.get(usize::from(ty.0)) {
        Some(cell) => *cell.get_or_init(compute),
        None => compute(),
    }
}
/// The live witness of a spawned decoration (`P_SpawnMapThing` p_mobj.c:760–795
/// through `P_SpawnMobj` 797–870, then nothing per tic, §2.5): its listed type and
/// definition, the spawn state with the state's sprite and frame and `tics` −1,
/// spawn health 1000, the type's reaction time, no target, threshold, `movedir`
/// or `movecount`, `lastlook` from one `P_Random() % 4`, the type's radius and
/// height, no momentum, `z == floorz` (spawned at `ONFLOORZ`, kept there by
/// `P_ThingHeightClip`, §2.7), the type's flags with at most `MF_AMBUSH` added
/// (p_mobj.c:792–793), solidity from `MF_SOLID`, and the THING record it was
/// spawned from at its own position (the spawnpoint, 795).
pub(crate) fn validate_decoration(
    room: Option<crate::player_map_scale::RoomBounds>,
    a: Actor,
) -> Result<(), SimError> {
    let l = a.life().ok_or(ActorError::InvalidType)?;
    let ty = decoration(l.actor_type).ok_or(ActorError::InvalidType)?;
    validate_definition(l.actor_type)?;
    let d = ty.descriptor();
    let s = d.states.spawn.descriptor();
    if l.state != Some(crate::StateId(d.states.spawn.ordinal()))
        || l.sprite != s.sprite.ordinal()
        || l.frame != s.frame
        || l.tics != -1
        || l.health != d.spawn_health
        || l.reaction_time != d.reaction_time
        || l.target.is_some()
        || l.threshold != 0
        || l.movedir != 0
        || l.movecount != 0
        || l.death_requested
        || a.lastlook() > 3
        || a.radius() != Fixed::from_bits(d.radius)
        || a.height() != Fixed::from_bits(d.height)
        || a.momentum() != [Fixed::ZERO; 2]
        || a.momz != Fixed::ZERO
        || a.z != a.floor
        || l.flags & !(d.flags | MF_AMBUSH) != 0
        || l.flags & d.flags != d.flags
        || (a.solidity == Solidity::Solid) != (d.flags & MF_SOLID != 0)
        || !crate::player_map_scale::position_admitted(room, a.position(), 978)
        || l.spawnpoint.is_none_or(|p| {
            i32::from(p.kind) != d.doomed_number
                || a.position()
                    != [
                        Fixed::from_map_units(p.position.x),
                        Fixed::from_map_units(p.position.y),
                    ]
        })
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    Ok(())
}
/// Whether `state` is the spawn state of the listed decoration `ty`: the only state
/// a decoration ever holds (`tics` −1, no successor entered, never killed).
pub(crate) fn spawn_state(ty: ActorTypeId, state: OriginalState) -> bool {
    decoration(ty).is_some_and(|d| d.states().spawn == state)
}
