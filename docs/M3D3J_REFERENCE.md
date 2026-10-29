# Demon map context and statistics: source investigation

This investigation must be committed before runtime implementation. Authority:
original Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`.
Inspected p_setup.c, p_mobj.c/h, p_inter.c, p_tick.c, p_enemy.c, p_map.c,
p_maputl.c, p_local.h, info.c/h, doomstat.h, doomdef.h, g_game.c, m_random.c,
d_player.h and relevant intermission fields in wi_stuff.c. Existing M3d3i
source findings and admission/preflight services remain the foundation.

## Enabled source dependencies

| Area | Source behavior | Proposed bounded admission |
| --- | --- | --- |
| Metadata recognition | MT_SERGEANT12, DoomEd3002, full source states/flags/raise | Preserve original table; explicit native136 adaptation remains |
| Map admission | Single-player/commercial filters, original metadata lookup, nomonsters | Reuse profile16 ordering and filters |
| Actor spawn | Health150, reaction8, radius30, height56, mass400, speed10; raw state/no action | Reuse existing spawn and H state aliases |
| Original flags | SOLID/SHOOTABLE/COUNTKILL =4194310 | Restore COUNTKILL only in new profile17 fixture |
| Level totals | P_SetupLevel resets kills/items/secrets; map spawn increments available kills | Three initialized totals; only total_kills can increase in this slice |
| Player statistics | Player-pointer credit, or single-player fallback to players[0].killcount | Admit fallback counter only for unbound/absent sources; no real player subsystem |
| Damage/death | Shared damage, corpse initialization, credit, state/timer RNG | Use shared DamagePlan/LifeHost; no second damage implementation |
| Terminal corpse | COUNTKILL + respawn disabled returns before movecount/time/RNG | Explicitly disable respawn and verify stable counters/identity/physics |
| Respawn | Enabled terminal branch eventually allocates fog/replacement and retires corpse | Reject requested respawn/Nightmare; do not execute NightmareRespawn |
| Full actor admission | Original identity/states/raise plus broader game/player/interaction context | Still deferred; document identity/state/raise adaptations precisely |

## Setup and available kills

P_SetupLevel first resets totalkills, totalitems, totalsecret and maxfrags, sets
partime180, and clears all four players' kill/item/secret counts. This occurs
before thinker/geometry initialization. It sets leveltime0 before P_LoadThings.
It does not clear the gameplay RNG: a reload continues the current random stream.
G_InitNew's separate random reset and G_DoLoadLevel's global game/input/sound
work are not interchangeable with the bounded setup slice.

P_LoadThings preserves lump order. P_SpawnMapThing applies single-player/skill/
type/deathmatch/nomonsters rules before spawning. P_SpawnMobj draws lastlook and
installs spawn state without calling its action. P_SpawnMapThing copies the
spawnpoint, draws a positive initial timer, increments totalkills for COUNTKILL,
then sets angle and ambush. Filtered placements contribute no actor, RNG or total.
A source-fatal unknown type is distinct from Rust's recoverable atomic rejection.

Totalkills means available map kills; damage, death and explicit corpse removal
do not decrement it. Direct P_SpawnMobj alone does not increment that map total.
Our native owners remain map-batch-only. No items or sector specials are admitted,
so initialized totalitems/totalsecret stay zero; no inventory/secret subsystem is
invented. Complete P_SetupLevel resource loading, specials, player starts and
intermission remain excluded.

## Exact death credit chronology

P_DamageMobj ignores nonshootable/already-dead targets before health, RNG or credit.
For lethal damage P_KillMobj first clears SHOOTABLE/FLOAT/SKULLFLY and NOGRAVITY
(except SKULL), sets CORPSE/DROPOFF and quarters height. Only then:

1. If source exists and source->player exists, COUNTKILL credits that player's
   killcount; target players also affect frags.
2. Otherwise, !netgame and COUNTKILL credits players[0].killcount, including
   environment, self and monster-caused deaths. This does not depend on player0
   activity, roster health or the source's perception-roster slot.
3. The actual target-player branch performs player death/weapons/automap/frags.
4. Death state entry follows; then the original randomized/clamped timer and drops.

The supported actor sources/targets have no player_t pointers. A dedicated
single_player_kill_count therefore represents the genuine fallback branch, not
points or full player-attributed statistics. Actual bound/player-pointer scoring,
frags, item/secret counts and intermission output remain unsupported. Repeated
corpse damage must not credit again. Demon has no xdeath or item drop.

Cross-native nonlethal damage would retarget into unsupported infighting and must
reject. A narrowly validated lethal/ignored synthetic request can be considered
because it returns before retargeting; autonomous native targets remain135 only.
Any such extension needs separate original-C evidence. No generalized infighting
is authorized.

## Corpse and removal consequences

COUNTKILL is retained through death and A_Fall. P_MobjThinker still performs XY/Z
before state handling. At tics=-1 it tests COUNTKILL, then immediately returns
when respawnmonsters is false, without changing movecount, consulting leveltime
or drawing RNG. Physics, reduced height, delayed sound/Fall, sector/block links
and thinker identity remain supported. P_RemoveMobj does not alter kill totals or
credit; it unlinks spatially and marks deferred thinker retirement.

Enabled respawn increments movecount, waits420 updates and leveltime alignment,
then draws RNG and may call P_NightmareRespawn. That function checks the original
spawnpoint, allocates two teleport fogs and a replacement, restores angle/ambush,
sets reaction18 and removes the old corpse. None of that is admitted. Reject
respawn configuration rather than pretending the enabled branch is a no-op.

## Context, restart and failure policies

Proposed profile17 uses one LevelStatistics record: i32 total_kills, total_items,
total_secrets and single_player_kill_count, all initialized0. Explicit context
options reject respawn and can carry the existing gameplay RNG into fresh level
construction; serialize actual RNG state, not a redundant initial-seed copy.
Reuse the exact prior parser/store/geometry/state/combat services and bounds.

Fresh restart should construct a new instance through the same validated
constructor, retain the old RNG stream, reset statistics/tic/actors/spatial state,
and execute the ordinary batch. The old instance remains unchanged on failure.
Actor handles remain scoped to their instance; do not claim globally unique IDs
across independent stores. New test targets/roster are installed explicitly since
source player starts and full game reload are unsupported. A bounded setup-time
map/registry copy is acceptable; no full-simulation clone occurs per tic.

Preflight the complete batch, type/state closure, total increments and capacity
before mutation. Damage planning must prove fallback counter addition before
health/flags/sounds/RNG/state changes. Shared execution updates the counter after
corpse initialization and before death-state entry. Integer overflow is a Rust
checked-error policy, not C-undefined compatibility. The four native owners still
form a prefix and attack only stable unbound135 bodies without COUNTKILL, so no
new credited death can occur inside the admitted native thinker pass.

## Independent evidence plan

Compile the genuine original load/spawn/damage/kill/thinker/movement/RNG closure.
For bounded setup statistics, extract the original contiguous P_SetupLevel
initialization statements and its leveltime reset verbatim, hashing each fragment.
If only fragments are compiled, report them separately from whole-function bodies;
never claim complete P_SetupLevel or replace verified algorithms with approximations.
Resource/bootstrap exclusions and all adapter assumptions must be explicit.

Compare unadapted original spawn separately from admitted native gameplay. Remove
only the old COUNTKILL flag projection: original flags/totals now match, while
native identity/state aliases and disabled raise remain declared. Record counter
updates relative to state/actions and compare per-tic totals, fallback credit,
flags, health, corpse state/timers, physics/order and exact original RNG.

Exercise empty/max-four/filter/nomonsters/ambush/multiple-sector inputs, supported
source classes, nonlethal/lethal/repeated/ignored damage, reset/restart, terminal
respawn-disabled updates, removal/generation reuse, long runs and RNG wrap.
Unsupported player-pointer/network/respawn and unsafe overflow/unknown-type
atomicity belong to clearly separated source components and Rust policies.

Independent DSIM-v17 derives fixed-width statistics fields/events, full bytes and
per-tic digests from original-C evidence/public inputs/schema, never Rust output.
Keep every v1–v16 encoder/fixture/tool and all historical examples unchanged.
DSIM is a project diagnostic format, not an original demo or savegame. Full
admission remains an investigation outcome, not an assumption.

## Source locations

Paths refer to the preserved `linuxdoom-1.10/` tree. `p_setup.c:594–601`
contains the statistics reset; `p_setup.c:646` resets leveltime before the
`P_LoadThings` call at line664. `p_mobj.c:789–792` increments map totals after
spawn timer randomization. `p_inter.c:668–723` orders corpse initialization,
player/fallback credit and death-state entry. `p_mobj.c:450–470` contains the
terminal COUNTKILL/respawn gate. `g_game.c:1112–1125` copies separate available
and credited counters into intermission input. These locations identify source
claims; generated provenance will hash the actual extracted bodies/fragments.

Raw C statistics use signed int. Safe Rust checked additions, bounded work and
recoverable rejection of unsupported game context are separate safety policies.
The public API must not allow arbitrary mutation of canonical statistics or
removal of COUNTKILL from the admitted native fixture. Source type12 remains
recognized metadata; executable native136 and remapped state IDs remain labelled.

## Implemented admission and evidence boundaries

Profile17 restores source COUNTKILL flags4194310 to the labeled native136 Demon
fixture. Source type12/state identities and disabled raise remain explicit
adaptations. Available map kills, initialized items/secrets and the genuine
single-player players[0] fallback kill counter are distinct canonical fields.
Only the admitted source branch is implemented; real player-pointer scoring,
frags, multiplayer and complete intermission are excluded.

Setup/reset reuses existing validated constructors and THING batches. Statistics
are initialized before world construction; no geometry callback or RNG draw can
observe partial setup. Fresh restart returns a new instance retaining both RNG
streams. It does not clone the entire simulation per tic or implement full reload.

Synthetic cross-native lethal/ignored damage is admitted only because it returns
before retargeting. Nonlethal cross-native damage and autonomous native targeting
remain rejected. Existing shared DamagePlan/LifeHost services prove/update death
credit between corpse initialization and death-state entry. Respawn mode rejects.

The genuine C harness compiles70 whole bodies and hashes two verbatim setup
fragments separately. Dirty setup counters, partime/maxfrags and all four players'
kill/item/secret counts are observed in a separate source component, together with
retained RNG indexes211/87. This is not execution of complete P_SetupLevel.
Separate source filter/player-pointer/network components do not expand runtime
admission. Source and adapter provenance and final qualified counts are linked
from [the evidence record](validation/M3D3J.md). Independent v17 gameplay captures
and additional schema-derived Rust atomicity states are distinguished there.
