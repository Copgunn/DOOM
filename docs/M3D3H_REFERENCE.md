# M3d3h source investigation: original Demon stand/run closure

Recorded and committed before runtime changes. This branch starts from verified
master `f44828d5ad3169fe77c116bdebb30c74d1b5feb2`. PR #17's documentation-only
correction `9d48f6d47c1a8c21f68cd66f6d56cb67f09bc8b3` passed fresh local checks
and [CI 40780721974](https://github.com/Copgunn/DOOM/actions/runs/40780721974).
The authorized regular merge has ordered parents
`75b2a2bc9a28401de358859f0499fe4c81dfa0a2` and that correction; tree
`4ed21d4d63ce376a7d5599b5b0f3c5416415edaa` equals the tested correction tree.
Exact merge [push CI 40781481835](https://github.com/Copgunn/DOOM/actions/runs/40781481835)
passed Linux, Windows, native macos-26-intel and Rust1.85, completed
2026-10-28 22:45:23 UTC before this branch or runtime work. PR17 is closed/merged;
local/remote master agree. All165 original files and1699 historical reference/
tool/support paths, ancestry, attribution, licensing and locks remain preserved.
No private repository or commercial IWAD was accessed. This source investigation
is planning material; implementation and independent validation are required.

Behavioral authority: preserved original id Software Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. Inspected `info.c/h`, `p_enemy.c`,
`p_mobj.c`, `p_tick.c`, `p_inter.c`, `p_map.c`, `p_maputl.c`, `p_sight.c`,
`m_random.c`, `r_main.c`, `s_sound.c` and the sound tables. Reuse the accepted
M3d3a–M3d3g perception, pursuit, melee and lifecycle services, not a new AI,
movement, damage or thinker scheduler.

## Source states and proposed explicit aliases

All states use SPR_SARG (39), with no fullbright bit. Copy complete original
state definitions, changing only the named fixture ID/next-state mapping.

| Source state / ID | Fixture ID | Frame | Tics | Action | Next fixture |
| --- | ---: | ---: | ---: | --- | ---: |
| S_SARG_STND / 475 | 1 | 0 | 10 | A_Look | 3 |
| S_SARG_STND2 / 476 | 3 | 1 | 10 | A_Look | 1 |
| S_SARG_RUN1 / 477 | 2 | 0 | 2 | A_Chase | 15 |
| S_SARG_RUN2 / 478 | 15 | 0 | 2 | A_Chase | 16 |
| S_SARG_RUN3 / 479 | 16 | 1 | 2 | A_Chase | 17 |
| S_SARG_RUN4 / 480 | 17 | 1 | 2 | A_Chase | 18 |
| S_SARG_RUN5 / 481 | 18 | 2 | 2 | A_Chase | 19 |
| S_SARG_RUN6 / 482 | 19 | 2 | 2 | A_Chase | 20 |
| S_SARG_RUN7 / 483 | 20 | 3 | 2 | A_Chase | 21 |
| S_SARG_RUN8 / 484 | 21 | 3 | 2 | A_Chase | 2 |

Keep the accepted aliases 4–6 (ATK1–3), 7–8 (PAIN1–2), 9–14 (DIE1–6).
Both ATK3 and PAIN2 return to RUN1/fixture2. Raise states are never admitted.
This is 21 admitted native-owner states; matching repeated frames never merges
distinct actions or state identities. The run cycle has eight entries in the
half-open 16-tic interval starting with RUN1; the next RUN1 entry starts the next
cycle. Idle action frequency is one Look per ten-tic entry, not every tic.

## Source timing and nested actions

P_SpawnMobj installs spawn sprite/frame/tics directly without executing its
action, explicitly because callbacks cannot run yet. Thus a direct spawn starts
STND1 with ten tics and first calls Look on entry to STND2 ten thinker updates
later. Explicit P_SetMobjState executes the entered action immediately.

P_MobjThinker runs XY movement, then Z movement, then timer decrement and state
entry; a newly installed positive timer is not decremented a second time in that
tic. P_SetMobjState installs sprite/frame/tics before invoking the action. Its
local `st` remains the entered state's record even if the action enters a nested
state; with this positive/infinite closure, the resulting mobj timer terminates
the outer zero-tic loop. Reuse existing LifeHost entry semantics and bounded
closure preflight. No interpolation or presentation frame controls gameplay.

A_Look resets threshold, consults the sector sound target and shootability,
assigns target before ambush sight/fallback, or scans the original four player
slots using lastlook, health, front-angle/range and REJECT/BSP sight. Successful
wake emits the fixed Demon see sound, then immediately enters RUN1 and executes
Chase in the same call. Chase can immediately enter ATK1 and FaceTarget.

A_Chase decrements reaction/threshold, turns toward movedir, handles missing or
nonshootable targets through all-around reacquisition and spawn-state entry,
handles JUSTATTACKED, selects melee before missile logic, then moves/chooses a
direction and conditionally makes active sound. Source melee range uses strict
approximate distance, target metadata radius and sight. With no missile state,
the native fixture must not gain missile callbacks or infighting rules.

P_Move uses the original eight fixed vectors and immediate P_TryMove; success
relinks cell/sector heads and snaps nonfloating Z to floor. P_TryWalk draws its
movement count only after success. P_NewChaseDir preserves diagonal priority,
swap/axis/old-direction/random-order/turnaround retries and RNG. Preserve things
before lines, supplied BLOCKMAP ordering, BSP/REJECT quirks, portal steps and
dropoff rules from the existing bounded movement profile. Specials/float/doors
are rejected, not successful no-ops.

## Metadata, combat and RNG

Original MT_SERGEANT/type12: Doom number3002, spawn475, health150, see477,
see sound41, reaction8, attack sound52, pain488, chance180, pain sound26,
melee485, missile0, death490, extreme death0, death sound64, speed10,
radius30*FRACUNIT, height56*FRACUNIT, mass400, damage0, active sound77,
flags SOLID|SHOOTABLE|COUNTKILL, raise496. Type13 Spectre adds SHADOW and remains
outside executable admission. Normal raw timing only: G_InitNew rewrites the
inclusive RUN1..PAIN2 timers for fast/Nightmare, which this profile rejects.

Continue the explicitly labelled native136 fixture: copy source properties,
retain DoomNum-1 and alias spawn/see/melee/pain/death, omit COUNTKILL and raise
because source player/global scoring and resurrection dependencies are absent.
Keep unbound type135 body fixtures and admitted blood/puff roles. Original actor
recognition is distinct from complete execution or original map spawning.

Fixed Demon see/attack/active/pain/death sound IDs do not add gameplay draws.
Active-sound probability draws occur only at the source Chase tail; fixed wake
sound itself does not draw. Keep source TryWalk/NewChaseDir RNG, source spawn
lastlook draw and all melee facing/strike/pain/death draws in actual order.
P_Random increments the original LUT index before reading and wraps at256.
Sound origin/ID/action timing is authoritative; presentation pitch/M_Random,
audio devices and mixer behavior are outside the headless boundary.

Retain SARG attack8/8/8 and three FaceTarget calls, fresh strike-time range/sight,
damage `(R%10+1)*4`, pain180 and2/2 with delayed sound, death8/8/4/4/4/-1 with
initial timer randomization, delayed Scream/Fall, immediate corpse/dropoff flags
and quarter height, later SOLID clearing and persistent terminal corpse physics.
Use the existing shared DamagePlan/LifeHost services.

Important P_DamageMobj distinction: the nonlethal source retarget branch enters
see immediately only when current state equals the exact spawnstate. STND2 has
the same Look action but is not spawnstate. Independently exercise no-pain hits
in both stand states instead of broadening this pointer-equality condition.

## Admission and recoverable failures

Create separate profile15, `ActorRegistry::demon_stand_run_fixture` and
`Simulation::with_demon_stand_run_registry`; profile14 retains its simplified
loops and every historical canonical byte. Require exact immutable fixture
definitions. Preserve the existing first single native owner,64 actors, map
and coordinate margins, sight work, positive timers and counter bounds.

Validate identities/generations, roles, sound/roster/current targets, all reachable
states/actions including nested Look/Chase/Face, movement, arithmetic, capacity
and work before mutation. Reuse bounded owner/RNG and potential-damage planning;
do not clone Simulation/Map/store each tic. No failed operation may partly move,
draw RNG, emit sound or enter state. Corpses may retain unresolved old target
generations without aliasing replacements. Setup must not revive a corpse.

The original all-inactive player scan can fail to terminate. Do not execute that
undefined fixture in C: use an active slot with dead roster health for real idle
comparisons. Bounded empty-roster termination and stale-ID rejection remain
explicit Rust safety policies, not original-C equivalence claims.

Unsupported: full original Demon/Spectre/map spawning, fast/Nightmare,
COUNTKILL/global statistics, player damage/inventory/scoring, raise/resurrection,
other autonomous monsters/projectiles, dynamic sectors/specials, multiplayer,
demo synchronization, rendering and presentation audio. Unsupported callbacks
must reject or abort in the isolated C adapter, never silently no-op.

## Independent evidence plan

Compile genuine original Look/Chase/Move/TryWalk/NewChaseDir/sight/melee/damage/
state/thinker functions with original structs/info and state tables. Copy the
ten original stand/run records into the same declared aliases in the C adapter.
Record original source/body/header hashes, adapter assumptions, compiler/ABI/
flags, exact public inputs and outputs. Body inventory is not branch coverage.

Exercise idle and full run loops, repeated frames, sight/sound/ambush wake,
nested wake-to-Chase-to-melee, target loss/reacquisition, obstacles/portals,
stand1-versus-stand2 damage, pain/death during stand/run/melee, terminal corpse,
all prior admitted damage relationships, actual RNG wrap and generation reuse.
Compare fields, timers/actions/sounds/RNG, thinker and spatial order. Multi-tic
captures must independently demonstrate that every one of the eight run states
executes, not merely appears in metadata.

Independent `m3d3h_snapshot_goldens.py` must read C traces/public inputs/schema,
never Rust output. DSIM-v15 composes existing persistent fields and observation
tags; no redundant state or corpse flag. Produce complete byte fixtures, digests
and first-field divergence diagnostics. DSIM is a project diagnostic format,
not an original DOOM savegame/demo. Keep original-C equivalence, independent
canonical consistency and Rust-only safety tests separate in all reporting.
