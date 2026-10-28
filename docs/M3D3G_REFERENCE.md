# M3d3g source investigation: Demon pain, death and corpse states

Recorded before runtime changes. Behavioral authority is the preserved Linux
DOOM 1.10 source at `a77dfb96cb91780ca334d0d4cfd86957558007e0`. This branch starts
at verified master **75b2a2bc9a28401de358859f0499fe4c81dfa0a2**. PR #16 was
regularly merged after documentation-only correction
`63efb2a585b9aa09032964cb1c4eae3dce1fae21` and successful
[correction CI 40761281692](https://github.com/Copgunn/DOOM/actions/runs/40761281692).
Ordered merge parents are `5b9182472123b8ec86b48cf6a4076426a7856f70` and that
correction; tree `cfeafd09f1649cfc5c4d8c8037af14fcc6af6abb` equals corrected head.
Exact merge [push CI 40762344038](https://github.com/Copgunn/DOOM/actions/runs/40762344038)
passed Linux, Windows, native Intel macOS 26 and Rust 1.85, completed
2026-10-28 20:26:10 UTC, before this branch or new runtime work. Local/remote
master, original 165 files, 940 historical reference/tool/support paths, original
ancestry, attribution, licenses and dependency locks were verified. No restricted
repository or commercial IWAD was accessed. Repository remains public/GPL-2.0-only.

This note records source findings and an implementation/evidence plan. Acceptance
requires independent comparisons and fresh checks; a plan is not implementation.
Reuse [M3d1 damage](M3D1_REFERENCE.md), [M3c lifecycle](M3C_REFERENCE.md),
[M3d3f melee](M3D3F_REFERENCE.md), [M3b movement](M3B_REFERENCE.md) and their bounds.

## Inspected source and metadata

Inspected `p_inter.c` (P_DamageMobj/P_KillMobj), `p_mobj.c` (P_SetMobjState,
P_MobjThinker, XY/Z, spawn/remove), `p_tick.c` (P_RunThinkers), `p_enemy.c`
(A_Pain/A_Scream/A_Fall/A_Look/A_Chase/P_CheckMeleeRange), `p_local.h`,
`p_mobj.h`, `p_map.c` (PIT_CheckThing/P_CheckPosition/P_TryMove), `p_maputl.c`
(link/unlink), `info.c/h`, `m_random.c`, `s_sound.c` and `sounds.c/h`.
The original files remain untouched; source/body/header hashes belong in the
independent harness provenance, together with adapter/input/output hashes.

MT_SERGEANT is Demon, source type 12 (not Shotgun Guy/type 2), Doom number 3002.
Original metadata: spawn health 150, reaction 8, speed 10, radius 30*FRACUNIT,
height 56*FRACUNIT, mass 400, pain chance **180**, pain sound **sfx_dmpain=26**,
death sound **sfx_sgtdth=64**. Spawn/see states are SARG_STND/RUN1; melee is ATK1,
missile and extreme-death states are S_NULL. Raise is SARG_RAISE1, not an implicit
resurrection service. Sounds see/attack/active are 41/52/77. Flags are
MF_SOLID|MF_SHOOTABLE|MF_COUNTKILL (4194310). Spectre/type 13 adds MF_SHADOW;
recognition does not grant full execution.

## Exact state chronology

All rows use **SPR_SARG=39**, without invented fullbright bits. State actions run
immediately on entry, including entry from another callback.

| Original state/ID | Frame | Raw tics | Action | Next |
| --- | ---: | ---: | --- | --- |
| SARG_PAIN / 488 | 7 | 2 | None | PAIN2 |
| SARG_PAIN2 / 489 | 7 | 2 | A_Pain | RUN1 |
| SARG_DIE1 / 490 | 8 | 8 | None | DIE2 |
| SARG_DIE2 / 491 | 9 | 8 | A_Scream | DIE3 |
| SARG_DIE3 / 492 | 10 | 4 | None | DIE4 |
| SARG_DIE4 / 493 | 11 | 4 | A_Fall | DIE5 |
| SARG_DIE5 / 494 | 12 | 4 | None | DIE6 |
| SARG_DIE6 / 495 | 13 | -1 | None | S_NULL |

Pain sound is **not** emitted at the initial pain decision: PAIN has no action.
After two thinker updates, entry to PAIN2 emits it; two updates later entry to
RUN1 executes Chase. A new hit may re-enter PAIN and restart these durations.
Death sound likewise waits for DIE2. Lethal damage enters DIE1 immediately,
then reduces its timer by one original gameplay draw &3 (minimum one). Here the
8-tic state becomes **5..8**, followed by 8/4/4/4/-1. DIE4 clears MF_SOLID only
after the shortened DIE1, DIE2 and DIE3 durations. DIE6's -1 is infinite: the
thinker never follows its S_NULL next field merely because time passes.

G_InitNew's fast/Nightmare rewrite includes RUN1..PAIN2. The bounded normal-timing
profile must reject Nightmare/fast initialization, preserving M3d3f's normal
8/8/8 attack timing and three separate faces. Full run/stand animation is still
an explicit fixture adaptation; copied PAIN2 returns to its admitted Chase loop.
No raise-state entry, Nightmare respawn or Arch-vile resurrection is admitted.

## Damage decisions, effects and RNG

P_DamageMobj first ignores nonshootable actors and health<=0 without RNG. For an
admitted living unbound body it applies horizontal thrust (if inflictor exists),
then subtracts health. A valid source and inflictor are distinct dependencies.
The thrust is damage*(FRACUNIT>>3)*100/mass at the original point angle; self
infliction and no-inflictor requests have the original respective behavior.
Low damage<40, damage>pre-hit health and target.z-inflictor.z>64*FRACUNIT may
consume a fall-forward draw; an odd value reverses and quadruples thrust. Keep
that draw before health/pain/death decisions, never unconditionally draw it.

Lethal health calls P_KillMobj and returns before pain, reaction or retargeting.
Nonlethal damage always draws one gameplay byte for `P_Random()<painchance`,
including zero damage. On success MF_JUSTHIT is set and PAIN is entered; pain
is not guaranteed. Reaction time is then reset to zero. When threshold is zero,
a different non-Vile source becomes target and threshold=BASETHRESHOLD=100.
If still in spawn state, a see state is entered immediately and executes Chase.
Self source does not retarget; a nonzero threshold preserves the current target.
Player armor/powers/skill damage, Vile special cases, drops and global scoring
are outside the supported slice, not silent approximations.

The real 256-byte LUT has **neither 179 nor 180**; 181 occurs at indexes 68/152.
Sweep actual indexes and wraparound for gameplay evidence. Controlled-dependency
original-C boundary probes at 179/180/181 must be labelled separately and must
not claim actual gameplay draws, scenarios or canonical captures. A_Pain and
Demon's fixed-sound A_Scream consume no gameplay RNG. Original presentation pitch
uses M_Random in s_sound.c; authoritative headless sound-origin/ID observations
reuse the existing inert sound boundary, without hardware/mixer RNG simulation.

## Kill initialization versus later fall

P_KillMobj clears MF_SHOOTABLE|MF_FLOAT|MF_SKULLFLY and, except MT_SKULL,
MF_NOGRAVITY; sets MF_CORPSE|MF_DROPOFF; quarters the current height. It does not
clear non-player MF_SOLID. Existing gravity/dropoff/solidity projections must stay
synchronized. Health was already reduced by damage. A low final health selects
xdeath only when metadata has a non-null xdeath state: Demon has none, so even
very large admitted damage selects ordinary death. Do not invent extreme death.

P_KillMobj increments a source player's killcount, or players[0] in non-network
COUNTKILL kills; target-player deaths also update frags/player state and weapon
logic. These dependencies require game/player statistics not present here.
Therefore the fixture must explicitly omit COUNTKILL and have no player_t, and
reject scoring/player/raise metadata. No global statistic may be fabricated.
Source Demon is not in the weapon-drop switch; no actor allocation is needed for
its pain, ordinary death or A_Fall. Hitscan still requires real blood/puff actors
and their RNG before shared damage; keep existing conservative impact reservation.

A_Fall only clears MF_SOLID. Neither kill nor Fall unlinks/reallocates the corpse.
PIT_CheckThing continues to block ordinary XY movement while MF_SOLID remains,
regardless of reduced height; after Fall it allows passage. Sector/block lists
retain the actor; ordinary movement relinks to list head using existing order.
The corpse retains thinker identity and target until an explicit supported
operation changes them. P_MobjThinker runs XY then Z then state decrement/entry;
-1 stops state progression but does not stop physics. Preserve corpse ledge
friction and existing static floor/ceiling behavior. P_RunThinkers preserves
append order and lazy retirement; death does not imply removal or slot reuse.

## Proposed explicit profile and safety proof obligations

Introduce profile 14 / `with_demon_lifecycle_registry`, retaining all v1-v13
behavior/bytes. One first native136 fixture extends the accepted SARG owner with
SHOOTABLE|SOLID, health150/mass400/pain180/sounds26/64, copied pain/death states,
no xdeath/raise/missile/COUNTKILL. Original type12/13 execution remains rejected.
Keep named Look/Chase fixture loops, SARG attacks and unbound type135 sources/
targets; permit unchanged original blood/puff effects only through shared hitscan.
Document every source-body adaptation. Unsupported projectiles, second AI owners,
full monster infighting, players, scoring, specials/dynamic sectors and raise
execution reject. Inspect projectile relationships, but do not grant cross-profile
plasma or other monster execution merely to supply a test source.

Use existing DamagePlan/apply_damage, LifeHost, state actions, thinker store,
movement and spatial services. No second damage/lifecycle/collision engine. All
recoverable failures, including unsupported later death/see transitions, invalid
source/inflictor generations, player bindings, coordinates, counters and capacity,
must precede health/RNG/events/flags/momentum/spatial mutation. Pain/death actions
allocate zero actors; direct damage must work at full capacity. Keep all existing
map/query/work/64actor limits and checked arithmetic.

Damage makes the old unshootable-owner/zero-momentum proof insufficient. Revalidate
bounds for knockback, airborne/corpse physics, immediate damage-to-Chase entry,
pain return, first-owner scheduling and current strike geometry after XY/Z.
Never use a cached pre-physics melee result for a post-physics strike. Small copied
owner/RNG and existing bounded plans can validate potential damage; actual source
range must run after real physics/facing. No full Simulation/Map/store clone per
tic or modern attack scheduler is permitted. Corpse target generations must never
resolve to a reused actor. If a dependency cannot be admitted safely, reject the
operation atomically and document the remaining acceptance criterion explicitly.

Public setup/state APIs must not provide an accidental resurrection route.
Dead owners cannot be reset to live SARG states or have health/shootability
restored through setup. Explicit removal/retirement remains distinct from death.
Public corpse damage with valid identities should take the original ignored
branch; invalid/pending/stale source or target is a labelled Rust safety rejection.

## Independent evidence plan and acceptance separation

Compile genuine original Damage/Kill/Pain/Scream/Fall/SetMobjState/MobjThinker/
RunThinkers, collision/link/fixed/RNG and admitted perception/melee/hitscan bodies
with original structures/info/state tables. Reuse frozen builders without changing
historical outputs. Record exact source/body/header/adapter/input/output hashes,
GNU flags/compiler/ABI and established signed-arithmetic/ordered-shadow assumptions.
Compiled inventory is not branch coverage. Adapters may configure explicit native
fixture metadata/state aliases and record sound requests; unsupported services
abort, not silently succeed.

Cases must cover real-LUT nonpain/pain and separate boundary probes; 2/2 sound/
Chase timing; repeated/during-pain damage; lethal entry and every randomized first
timer; delayed death sound/Fall; height/flags/collision/list membership; terminal
corpse stability/ignored damage; extreme-health ordinary death; knockback/fall-
forward; null/self/distinct source/inflictor; retarget/threshold; generation reuse;
multiple actually exercised synthetic maps; RNG wrap; complete multi-tic sequences
and repeated instances. Compare actor fields, transitions/sounds, raw RNG,
thinker/sector/block order, not only final hashes.

Independent `m3d3g_snapshot_goldens.py` consumes C traces/public inputs/schema only,
never Rust output, producing DSIM-v14 fields, full bytes and per-tic digests with
first-field diagnostics. Reuse health/flags/height/momentum/state/timer/target/
threshold/reaction/identity/order/RNG fields; no redundant persistent corpse flag.
Add transient observations only if existing combat/state/sound events cannot
express the exact chronology. DSIM is project diagnostic evidence, not a vanilla
savegame/demo format or full synchronization claim.

Rust policies separately test unsupported definitions/actions/players/resurrection,
stale/pending/reused identities, arithmetic/work/capacity, complete error atomicity,
pause/read independence and debug/release/repeated-instance determinism. Preserve
all thirteen older C/encoder/example sets. Add asserted `demon_lifecycle`, bounded
fuzz/ASan with actual counts/leak settings, all fuzz builds and native four-job CI.
The new M3d3g PR must stay open/unmerged without auto-merge for owner review.
