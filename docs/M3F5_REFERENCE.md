# M3f5 investigation: canonical Demon execution binding

Authority is preserved Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. This initial investigation is
recorded before M3f5 implementation. [M3f4](M3F4_REFERENCE.md) established
canonical metadata; [M3f3](M3F3_REFERENCE.md) established the bounded executable
closure. This milestone connects them, preserving the distinction between original
identity, runtime registry representation and executable game context.

## Source identity and state facts

The existing independent compiled-C metadata capture establishes these facts:

| Actor | Original ordinal | DoomEd | Flags |
| --- | ---: | ---: | ---: |
| MT_SERGEANT, Demon/Pinky | 12 | 3002 | 4194310 |
| MT_SHADOWS, Spectre | 13 | 58 | 4456454 |
| MT_SHOTGUY, Shotgun Guy | 2 | 9 | 4194310 |

Demon and Spectre share state references, but are distinct actors. A shared sprite
or state graph does not authorize Spectre behavior. MT_SHOTGUY is unrelated to
this melee milestone. The original Demon has health150, reactiontime8, painchance180,
speed10 map units, radius30/height56 fixed units, mass400, no missile/extreme death
state, and a raise reference496. These values come from info.c, not inference.

| Sequence | Original symbols / ordinals | Normal tics and actions |
| --- | --- | --- |
| Spawn/stand | S_SARG_STND / STND2, 475–476 | 10/10, A_Look |
| See/run | S_SARG_RUN1–8, 477–484 | 2 each, A_Chase |
| Melee | S_SARG_ATK1–3, 485–487 | 8 each; FaceTarget, FaceTarget, SargAttack |
| Pain | S_SARG_PAIN / PAIN2, 488–489 | 2/2; NULL, A_Pain |
| Death | S_SARG_DIE1–6, 490–495 | 8/8/4/4/4/-1; Scream on DIE2, Fall on DIE4 |
| Raise | S_SARG_RAISE1–6, 496–501 | 5 each; metadata only, excluded execution |

DIE6 has next S_NULL but an infinite timer. It persists without ordinary transition.
The executable normal closure has21 states plus the special null reference; raise
is not reachable by an ordinary next edge from that closure.

## Behavior and dependency classification

| Finding | Classification and authority |
| --- | --- |
| Enum ordinals, descriptor fields and action symbols | Independently observed original-C metadata, M3f4 capture |
| Raw spawn installs info/state without calling its action; lastlook draws once | Original P_SpawnMobj; already independently validated |
| THING filters and original DoomEd lookup precede spawn; positive timer draws again; COUNTKILL increments before angle/ambush | Original P_SpawnMapThing/P_LoadThings; already independently validated |
| Entry installs state/timer/frame before action; zero-tic chaining observes current timer; nested actions preserve outer next | Original P_SetMobjState; already independently validated |
| XY/Z precede state countdown; -1 persists; thinker appends and traversal remain ordered | Original P_MobjThinker/p_tick.c; already independently validated |
| Look/Chase dynamically select spawn/see/melee references | p_enemy.c; existing source-derived perception/pursuit services |
| Melee faces, checks current range/sight, draws damage once after acceptance | A_SargAttack/P_CheckMeleeRange; existing shared melee/damage services |
| Pain chance180, health/flags before death entry, delayed Scream/Fall, quarter-height corpse and fallback kill credit | p_inter.c and p_enemy.c; existing independent lifecycle/statistics evidence |
| Source BLOCKMAP order, movement attempts, sight clipping and RNG ordering | p_map.c/p_maputl.c/p_sight.c/m_random.c; existing bounded Profile20 evidence |
| Native type136/state aliases and unbound target135 | Explicit Rust/reference-fixture representation policy, not original ordinals |
| Generational references, bounded work, transactional rejection and clipped INT_MIN rejection | Rust safety policy; not claims of defined original-C continuations |
| Raw canonical type12 gameplay throughout a new capture | Independently observed by the M3f5 compiled-C observer |
| Canonical requests driving existing state/spawn services | Newly validated definition binding over already implemented Profile20 services |
| Spectre, players, generalized infighting, Fast/Nightmare, respawn, resurrection, dynamic sectors | Unsupported/unverified game contexts; no admission |

The type-branch audit includes P_CheckMissileRange, A_Look/A_Scream, damage/kill
selection and PIT_CheckThing. Source12 does not acquire a boss/global sound, Vile,
projectile or player branch merely by replacing native136. The missile-dependent
collision branches are unreachable under the admitted flags; missile state is null.
These source observations motivate tests, not a universal equivalence proof.

Raise requires A_VileChase/corpse selection, height restoration and additional
collision/game context. No such dependency is implemented here. g_game.c's global
Fast/Nightmare rewrites remain excluded. Runtime native raise suppression must
remain explicit while the canonical source descriptor retains its real reference.

## Chosen integration boundary

Use a checked, one-to-one canonical execution binding over Profile20. Replace the
current map-context fixture's layered hardcoded metadata construction with one
projection of M3f4-resolved source records. Preserve every resulting native field,
label and next reference. Add typed canonical spawn/state requests and definition
observations that delegate to the existing simulation services.

The binding must verify the complete canonical actor record, every admitted
canonical state record, and the corresponding runtime projection. It must reject
altered metadata, unbound states, other actor identities and fixture substitutions.
Map lookup still uses the real original actor record; the explicit runtime mapping
occurs only after filtering. No extra RNG, event, thinker, actor store or scheduler
is introduced. Pain/death remain damage-controlled; direct state entry keeps the
existing live stand/run/attack restriction. Raise never gains an entry binding.

Raw ActorLife and DSIM retain the native136/alias representation. A canonical
observation names the source definition behind a verified execution binding; it
must not falsely report that raw runtime storage uses original ordinals. M3f4's
strict same-record resolver must continue rejecting native136 as original metadata.

A direct raw-ID migration would also change role/state tests across spawning,
combat, perception, pursuit and lifecycle, plus canonical diagnostic identity bytes.
That is a separate representation migration. It is unnecessary to establish the
requested canonical-to-execution connection and would broaden this milestone.
There is no new behavior/capacity to justify a gratuitous Profile21 or DSIM version.

## Independent verification design

The new versioned observer runs the existing genuine C bodies with type12 and
states475–495 throughout, removing the old Demon type/state adaptation from its
C setup. It retains and declares only the unbound target135/roster and established
arithmetic/geometry adapters. Retain raw canonical observations, including RNG and
state-bearing event payloads. Never use Rust output to manufacture expectations.

Compare physical/lifecycle fields against Rust plus its explicitly named canonical
definition observation. Independently project C identity fields into the frozen
DSIM-v20 representation for full snapshots and ordered event-byte comparisons.
Keep source behavior, projection, canonical encoding and Rust-only rejection
results separate. Existing Profiles1–20 and all historical fixtures/tools remain
unchanged. Actual counts and conclusions belong in the [measured evidence record](validation/M3F5.md).

## Exact source dependencies and observation boundary

The source inventory and per-body digests are retained in
[m3f5_provenance.json](../tests/reference/m3f5_provenance.json). These are the same
compiled bodies as M3f3, under different identity setup; this is not new function
or exhaustive branch coverage. Source facts below have existing independent
coverage and the stated new canonical-identity input slice; untested contexts
remain unsupported.

| Source location | Symbols and relevant ordering |
| --- | --- |
| [info.h](../linuxdoom-1.10/info.h), [info.c](../linuxdoom-1.10/info.c) | mobjtype_t/statenum_t/spritenum_t, mobjinfo/states; source enum and descriptor references remain pristine |
| [p_mobj.c](../linuxdoom-1.10/p_mobj.c) | P_SpawnMobj, P_SpawnMapThing, P_SetMobjState, P_MobjThinker, P_XYMovement, P_ZMovement: raw spawn/action distinction, floor/state setup, movement before countdown, nested entry, terminal timer |
| [p_tick.c](../linuxdoom-1.10/p_tick.c) | P_AddThinker/P_RunThinkers/P_RemoveThinker: append traversal and deferred retirement; no identity-based sorting |
| [p_enemy.c](../linuxdoom-1.10/p_enemy.c) | A_Look/P_LookForPlayers/P_NoiseAlert: sound target and ambush sight, bounded roster scan, see entry; A_Chase/P_NewChaseDir/P_Move/P_TryWalk: reaction/threshold, turn, melee selection, source movement candidates |
| [p_enemy.c](../linuxdoom-1.10/p_enemy.c) | A_FaceTarget clears ambush; A_SargAttack repeats melee range/sight at callback time, then (P_Random()%10+1)*4 damage. Shared states alone do not admit Spectre's shadow-dependent facing branch |
| [p_inter.c](../linuxdoom-1.10/p_inter.c) | P_DamageMobj/P_KillMobj: thrust/health, lethal branch before pain, pain roll/state, reaction/retarget, flags/corpse/height, kill credit and death timer adjustment |
| [p_enemy.c](../linuxdoom-1.10/p_enemy.c) | A_Pain/A_Scream/A_Fall: sounds and delayed SOLID removal; source COUNTKILL survives death |
| [p_map.c](../linuxdoom-1.10/p_map.c), [p_maputl.c](../linuxdoom-1.10/p_maputl.c) | P_TryMove/P_CheckPosition/PIT_CheckThing/PIT_CheckLine, P_BlockThingsIterator/P_BlockLinesIterator and P_SetThingPosition: source list order, blocking/floor/ceiling and relinking |
| [p_sight.c](../linuxdoom-1.10/p_sight.c) | P_CheckSight/P_CrossBSPNode/P_CrossSubsector: REJECT/BSP/portal clipping; original INT_MIN subtraction remains excluded by checked Rust policy |
| [m_random.c](../linuxdoom-1.10/m_random.c) | P_Random advances prndindex before reading, wraps at256; M_Random owns separate rndindex |

RNG sites include raw spawn lastlook; positive map-spawn timer; selected pursuit
candidate order/walk duration/active sound; admitted melee damage; pain decision;
ordinary death timer shortening; and source sound randomization where applicable.
A rejected range attack draws no damage. Actual source draws are logged in order,
reconciled to the simulation's owned index and dependent observations; the Rust
implementation does not expose an independent log for every internal draw.
Presentation/metadata queries consume neither stream. The raw spawn trace retains
setup skips/discards separately from gameplay trace draws.

Raise metadata is observed without entering it. Source global Fast rewrites,
Nightmare timing/respawn, A_VileChase corpse selection, real player pointers,
nonlethal native infighting and Spectre shadow branches are unsupported/unverified.
The synthetic fallback roster and target135 are explicit context policy, not proof
of real player gameplay or whole-game Demon compatibility.
