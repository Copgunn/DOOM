> Historical source investigation before public-documentation restructuring. Exact bytes: [pre-m3d3j-m3d3a_reference.txt](pre-m3d3j-m3d3a_reference.txt).

# M3d3a source investigation: perception and target acquisition

Recorded before runtime changes. Authority is untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. M3d2b reviewed head `062647d`
received prose-only finalization `f2705598f382cd7c3e308f3edfbe3756f694c598`.
Fresh CI 40631701968 passed before regular PR #9 merge
`e109ee193bcce6077f5c8d90802c97edc89a55a6`, parents
`955f690d141ccc6a9074a8a8017f78833854af6f` and `f2705598f382cd7c3e308f3edfbe3756f694c598`.
Post-merge CI 40632182190 passed all four jobs before this branch was created.

## Source and dependency boundary

Inspected p_sight.c, p_enemy.c, p_local.h, p_maputl.c, p_map.c, p_mobj.c,
p_tick.c, r_main.c/r_defs.h, d_player.h, doomstat.c/h, info.c/h and m_random.c,
with existing M1/M3b/M3c/M3d1/M3d2/M3d2b investigations and implementations.
Sight needs validated sector/subsector/seg/linedef/BSP/REJECT data, source fixed
multiply/divide, current actor XYZ/height and temporary duplicate stamps. Player
scanning adds playeringame[4], player.health/mo, actor.lastlook/angle/target,
R_PointToAngle2, P_AproxDistance and MELEERANGE=64*FRACUNIT. A_Look adds threshold,
sector.soundtarget, shootability/AMBUSH, metadata see sound/state, P_Random,
S_StartSound and P_SetMobjState. No chase or attack implementation is needed to
isolate those functions with an explicitly action-free see handoff.

A_Chase was inspected to establish the next boundary: reaction/threshold decay,
movedir angle changes, missing-target reacquisition, JUSTATTACKED handling,
melee/missile range and attack selection, nightmare/fast behavior, network target
switching, movecount/P_Move/P_NewChaseDir, active-sound RNG and callbacks. These
require movement decisions, target health/shootability, door/special interactions
and attack actions. None is executable in this milestone. Original monster see
states normally dispatch A_Chase immediately and must reject before mutation.

## Sight arithmetic and traversal

P_CheckSight first computes `pair=observer_sector*numsectors+target_sector`,
then byte `pair>>3` and mask `1<<(pair&7)`. A set bit returns false without BSP
work, even for the same sector. Structural REJECT validity does not prove actual
visibility; asymmetric synthetic bits are legitimate test inputs.

For a possible sightline, eye Z is `observer.z+height-(height>>2)`. Top/bottom
initially are target top/bottom minus eye Z, without a distance division. strace
is observer XY plus target-observer deltas. Traverse the last BSP node, or -1
for a node-free map, which dispatches subsector 0. No same-leaf shortcut exists.

P_DivlineSide returns 0/1/2. Vertical partitions return 2 for exact X equality.
The horizontal branch contains the original **x==node.y** equality test (not
 y==node.y); preserve it. General classification subtracts the origin, shifts
both factors by 16 BEFORE integer multiplication, compares right versus left,
and returns 2 on equality. This differs from movement and hitscan predicates.

P_CrossBSPNode maps a starting side 2 to 0, traverses that starting child FIRST,
returns immediately if it blocks, then computes the target side. If it equals
the normalized start side, return true; otherwise traverse the other child.
A target side 2 can therefore visit both children. Use an explicit local stack
with an after-first-child continuation, preserving the actual source order.

P_CrossSubsector processes stored seg order. Each seg uses its parent linedef;
mark duplicate suppression BEFORE either line crossing test. Compare both line
vertices against strace, then observer/target against the linedef divline. Equal
sides skip the line; unequal sides, including side 2, continue. A crossing
without TWOSIDED blocks. For a two-sided crossing, use **that seg's front/back
sectors**, not an invented nearest surface. Equal floor AND ceiling heights skip
clipping. Otherwise min ceiling/max floor form the opening; bottom>=top blocks.
P_InterceptVector2 has its own source arithmetic:

```
den = FixedMul(v1.dy>>8,v2.dx) - FixedMul(v1.dx>>8,v2.dy)
num = FixedMul((v1.x-v2.x)>>8,v1.dy) + FixedMul((v2.y-v1.y)>>8,v1.dx)
frac = den==0 ? 0 : FixedDiv(num,den)
```

Here the call is (&strace,&linedef_divline). Only unequal floors raise the bottom
slope; only unequal ceilings lower the top slope. Divide opening-minus-eye by
frac with the existing source FixedDiv guard/saturation. Stop when top<=bottom.
No BLOCKMAP hitscan, modern raycast, floating-point vision cone or rebuilt BSP
can replace this algorithm.

Global sightcounts are diagnostics. Source validcount supplies fresh line stamps;
noise uses the same counter but sector stamps. Before integer wrap, these counters
have no other gameplay effect. Local per-query masks and scratch state preserve
duplicate behavior without mutating simulation state or serializing diagnostics.
Rejected queries need not expose stale C global slope values from earlier calls.

## Player scan and lastlook

Spawn lastlook is the existing P_SpawnMobj gameplay draw modulo four. Search starts
at CURRENT lastlook; stop=(lastlook-1)&3. Each loop first filters playeringame.
For active slots, `if (c++==2 || lastlook==stop) return false` happens BEFORE
health/sight testing. Thus at most two active candidate encounters are examined;
stop can prevent examining a slot, and repeats of the same active player count.
Inactive slots do not consume this count. On each continued iteration lastlook
advances modulo four; returning leaves it at the current slot.

Use player.health<=0, not actor health or metadata identity. Sight precedes FOV.
Without allaround, subtract actor.angle from source PointToAngle2 in wrapping
unsigned BAM. Only strict `an>ANG90 && an<ANG270` is rear. Rear candidates are
still accepted when AproxDistance<=64*FRACUNIT; the approximate distance is
abs(dx)+abs(dy)-(min(abs(dx),abs(dy))>>1). Successful acquisition assigns target
without an RNG draw. Do not select nearest or sort actor handles.

With no active players the original loop never reaches its stopping condition.
A declared safe policy will return false with unchanged lastlook for an empty
roster; the original nonterminating case cannot be counted as C equivalence.
For a nonempty four-slot roster, a bounded loop of at most 12 slot visits covers
the source stopping rule. Active/pending/stale identities must be prevalidated;
no failed operation may partially update lastlook or targets.

## Sound and A_Look

Reuse the accepted M3d2 P_NoiseAlert/P_RecursiveSound implementation. Its emitter
sector and retained target are distinct inputs in original source. Sector lines
are visited in source order, closed openings stop propagation, and at most one
SOUNDBLOCK crossing is allowed. A sector can be revisited by a cheaper path.
The visited depth is temporary; sector sound targets persist as generation IDs.

A_Look FIRST resets threshold=0 (not reactiontime), then reads its sector's sound
target. A shootable sound target is assigned to actor.target BEFORE AMBUSH sight
checking. Ordinary listeners wake immediately on that valid sound target.
Ambush listeners require sight, otherwise fall back to P_LookForPlayers(false).
If fallback fails, the previously assigned sound target remains stored even
though no see-state transition occurred. Preserve that order and distinction.
A nonshootable sound target causes player fallback without assignment.

On waking, seesound posit1/2/3 chooses posit1+P_Random()%3; bgsit1/2 chooses
bgsit1+P_Random()%2; other nonzero sounds are unchanged; zero consumes no draw.
SPIDER/CYBORG types emit globally (null sound origin), other types use the actor.
Then enter metadata seestate immediately through P_SetMobjState, including any
immediate action. Reuse LifeHost's recursive state-entry semantics; never queue
or silently suppress A_Chase. Validate the full see closure before admission.

## Planned bounded profile and safe ownership

Profile 8 deliberately composes typed actors/static-sector physics, threshold,
shared target/state services, sight and shared noise. Weapon commands, projectiles,
actual player damage, pursuit, attacks and multiplayer networking remain separate.
A minimal perception-only roster stores active, signed player health and an
optional generation-aware actor ID for four slots. It is not player_t and does
not equate player metadata, roster activity, health or actor identity.

Original states/types remain recognized. Executable A_Look tests may adapt source
states 1/2 into clearly labelled FIXTURE_LOOK/FIXTURE_SEE and copy player-shaped
metadata into explicit fixture type 136 (and 19/21 solely for global-sound branch
isolation). The look state invokes real source A_Look; the see handoff is action
free and stable. Full original monster admission remains rejected for chase/
attack dependencies. No scripting/probe callbacks enter this profile.

Proposed arithmetic slice: map XY/node origin/delta within +/-512 map units,
actor query XYZ within +/-1024, bounded heights and at most 64 actors. Existing
map limits remain; expanded BSP traversal must have a bounded worst-case cost
also for acyclic shared-child inputs. Reject excess work before gameplay mutation,
never rebuild topology. Step preflight must prove the same slice after ordinary
physical movement before A_Look; use small actor records/bounds, not a full state
clone. Record final measured bounds after implementation and comparison.

Active candidate and observed sound-target identities must resolve to active
original generations. Pending/retired sound targets should reject before threshold,
target, lastlook, RNG or event changes, rather than emulate a dangling C pointer.
Retained identities elsewhere remain unchanged and never alias reused slots.
Read-only sight must not depend on roster configuration, consume RNG or mutate
persistent fields. Pure query stacks/masks/trace observations stay local.

## Reference and schema plan

Compile genuine P_CheckSight/P_DivlineSide/P_InterceptVector2/P_CrossSubsector/
P_CrossBSPNode, P_LookForPlayers, A_Look, noise, point-angle/distance, state,
thinker/spawn and RNG bodies with original structs/metadata. Synthetic adapters
load legal classic maps, seg sides, REJECT and minimal player records. Unsupported
source actions abort only when unreachable. Retain retired C allocation bytes for
observation; Rust validates and retires generation handles. Exclude nonterminating
empty-roster/dangling/extreme-overflow cases from source-equivalence claims and
test the declared safe policies separately.

Record source/function/header/adapter/input/output hashes, compiler flags and ABI.
Compare ordered decisions, slopes/fractions, seg/line duplicates, player scan/FOV,
threshold/targets/states/sounds, RNG and thinker order, with first divergence.
Independent Python DSIM-v8 bytes must read C outputs/public setup/schema only.
Compose v5 persistent typed fields plus sector sound targets and the roster;
existing lastlook/target/state/threshold fields already represent actor results.
Add explicit perception events without changing v1-v7 encoders or bytes. Do not
serialize local sight stacks, masks, temporary stamps, counters or rendering data.
The preceding plan was committed before implementation. Delivered behavior and
measured evidence follow; planned language above records that initial boundary.

## Delivered admission and failure policy

The plan above is implemented in `sight.rs`, `perception.rs` and the shared
LifeHost/noise services. `monster_perception` selects profile 8; the explicit
registry constructor admits either exact original metadata or only the labelled
native overrides described above. Fixture look tics are1..35; see is stable-1.
Only fixture types 19/21/136 execute A_Look. Entire see/spawn/death closures are
prevalidated; original monsters with A_Chase reject before allocation/mutation.
The original registry still supports pure sight/scanning over admitted actors.

Final map bounds are +/-512 for vertices, node origin/delta and sector heights;
actor query XYZ is +/-1024. At most64 actors and inherited static-map limits
apply. Expanded traversal costs leaf=1+seg_count, node=2+both child costs, including
repeated DAG paths, and must fit 8192. Traversal is iterative with local masks.
Before a tic, nonzero XY momentum requires each moving coordinate within +/-994
(the original30-unit clamp margin); momZ is at most64 and z+momZ must fit +/-1024.
Bounds reject before state/position/RNG changes rather than relying on C overflow.

Roster active, signed player health and optional generation ID remain distinct.
Each active slot, even health<=0, must resolve to an active stable source player
body state 149..173 or native look/see state 1/2 with admitted metadata. Setup also
validates any supplied inactive identity; a later inactive retained old generation
may remain encoded and cannot alias reuse. Referenced actors cannot enter removing
states. This proves that earlier thinkers cannot invalidate a later looker's
roster/sound dependencies inside the tic. No full Simulation/Map clone is used.

Explicit A_Look preflights its current sound target; a tic containing any possible
looker preflights **all retained sector sound IDs** because XY can change sectors
before the action. This conservative check can reject a stale cache in a sector
that is not ultimately read. Pending/stale IDs reject before threshold, targets,
lastlook, movement, sounds, events or RNG. Pure sight only validates its two actors
and remains independent of invalid roster/cache state. Empty-roster false with
unchanged lastlook is a separately tested safe departure from nonterminating C.

## Delivered original-C evidence

The isolated harness compiles 56 original bodies, including all five p_sight
functions, P_LookForPlayers, A_Look and shared noise, plus their source movement,
angle, lifecycle, thinker, spawn and RNG dependencies. It uses original structs,
info/tables and source fixed helpers. Query/decision logging wraps or instruments
source statements without replacing their decisions. Native fixture metadata
isolates genuine A_Look; unreachable unsupported callbacks abort. Source pointer
identities are normalized to spawn ordinals, and C allocation bytes are retained
for observation rather than reused unsafely. Spawn jitter's ordered negative-shift
adapter and compiler/ABI assumptions are recorded in hashed provenance.

GCC 14.2.0 produced68 scenarios over18 public synthetic maps:24 sight, 20 scan and 24
explicit/thinker A_Look, plus 1536 side and 131 fraction vectors. Maps include same-
sector and cross-byte REJECT bits, multi-node/rotated BSPs, duplicate seg/lines,
one-sided/equal/closed/partial openings, floor/ceiling clipping, negative heights,
diagonal ties and omitted BLOCKMAP walls. Scan cases cover inactive/dead records,
slot 3 wrap, candidate/stop limits, FOV boundaries, allaround and approximate rear
distances64/65. Sound cases cover one/two blockers, closed openings, distinct
emitter/target, unregistered/dead-record/nonshootable targets, ambush failed
fallback with retained assignment, wake sound groups/global origins and RNG wrap.

Across these cases Rust compares 47,520 actor fields, 625 raw RNG draws, 365 sight
queries/3955 ordered sight visits, 460 slot observations and 4752 roster fields.
A_Look also compares 346 sector sound fields and 19 source noise visits. Detailed
per-group ordered-observation counts and local check results are in
[VALIDATION.md](../../VALIDATION.md); counts are asserted rather than inferred from
fixture size. Five extra noise probes repeat cases without inflating totals.
Pure queries preserve snapshots/RNG. Twelve separate Rust policy tests exercise
unsafe/nonterminating-C exclusions, identity reuse, unsupported handoffs and
failure atomicity. No dangling/infinite/overflow reference case is run or claimed
as equivalence.

The independent v8 Python encoder reads only C rows, public inputs and schema:
420 captures/45 complete byte fixtures, including transient perception events.
The eighth asserted headless example ends at `30721bdec7a0d47e`. These are project
schema/determinism evidence, not original savegames/demo synchronization. All
historical generators/encoders/code fixtures and v1-v7 bytes remain unchanged.
M3d3b pursuit/movement decisions requires a separate source investigation; attacks
remain later. No original monster A_Chase execution is claimed.
