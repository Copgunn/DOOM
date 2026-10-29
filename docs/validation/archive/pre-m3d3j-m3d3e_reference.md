> Historical source investigation before public-documentation restructuring. Exact bytes: [pre-m3d3j-m3d3e_reference.txt](pre-m3d3j-m3d3e_reference.txt).

# M3d3e source investigation: chaingunner attack and refire

Recorded before runtime changes. Authority is untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. This branch starts from PR #14's
verified regular merge `31b4830945a60c837065c0e3c29d10494e84f20e`, ordered parents
`9b8fde85f64651f12a5643324e2ab93a9e4937d8` and
`64cf9a920ee21669945cb9537c1cd43076aa37ec`, tree
`26f37668dfb5ce50d14b99ee72d8da0a88e539c6`. Correction-head CI 40661503550 and
exact post-merge CI 40662401302 passed all four jobs before this investigation.

## Source and dependencies

Inspected p_enemy.c, info.c/h, p_mobj.c, p_tick.c, p_map.c, p_maputl.c,
p_sight.c, p_inter.c, p_local.h, m_random.c and the original angle/fixed helpers,
alongside the [M3d1](../../M3D1_REFERENCE.md), [M3d2](../../M3D2_REFERENCE.md),
[M3d3c](../../M3D3C_REFERENCE.md) and [M3d3d](../../M3D3D_REFERENCE.md) contracts.
Original files, metadata outside explicit fixtures, and historical evidence stay
unchanged. Reuse the existing source sight, perception, pursuit, LifeHost,
state-entry, hitscan/damage/effect and generational thinker services.

In p_enemy.c:845, A_CPosAttack returns on null, otherwise emits sfx_shotgn,
calls A_FaceTarget, saves bangle, aims once at MISSILERANGE 2048, draws left/right
spread then damage, and performs ONE P_LineAttack. Spread is wrapping BAM
addition of (left-right)*2^20; damage is ((R%5)+1)*3. Explicit ordered multiplication
defines the same historical negative-shift/operand-order interpretation as v10/11.
Blood/puff is a real actor before immediate damage; its spawn/jitter/timer and
pain/death/fall-forward draws finish before any later action. Every invocation
recomputes facing and aim from the current world. There is no shared cross-tic
aim, ammo inventory, cooldown, noise alert or aggregate attack.

At p_enemy.c:865, A_CPosRefire ALWAYS calls A_FaceTarget first. Face returns on
null; otherwise it clears AMBUSH and consumes two shadow draws when applicable.
Refire then draws once. A value below 40 returns without checking target health
or sight, retaining the natural attack loop. Otherwise source || short-circuits
null, then mobj health <=0, then !P_CheckSight. A stop immediately enters the
metadata see state. A healthy visible target retains the refire state. Roster
health is not mobj health. A dead body's remaining SHOOTABLE flag can still
affect nested Chase after a refire exit; do not substitute a modern eligibility
rule. Distinguish refire-condition sight from later nested Chase/Look queries.

## CPOS states and exact fixture boundary

Original info.c:552 uses SPR_CPOS (38), not POSS or SPOS:

| State | Frame | Tics | Action | Next |
| --- | ---: | ---: | --- | --- |
| ATK1 | 4 | 10 | FaceTarget | ATK2 |
| ATK2 | 32773 | 4 | CPosAttack | ATK3 |
| ATK3 | 32772 | 4 | CPosAttack | ATK4 |
| ATK4 | 5 | 1 | CPosRefire | ATK2 |

Both attack frames retain fullbright. Initial facing occurs once on ATK1;
each shot and refire performs its own internal facing. The loop after the first
sequence is ATK2 -> ATK3 -> ATK4 across normal tics, never a callback loop.
P_MobjThinker counts down after XY/Z. P_SetMobjState invokes actions immediately;
nested state replacement must remain visible. Appended impacts participate later
in that same P_RunThinkers traversal, with deferred retirement and real reuse.

Use separate profile 12 / DSIM-v12. Reuse reserved native type 136 as labelled
FIXTURE_CHAINGUNNER_136, states 1/2 as positive looping CPOS Look/Chase fixtures,
and states 4/5/6/7 as exact copied CPOS attacks mapped next 5/6/7/5. Keep dummy
type 135's unbound original player-body closure and exact puff 37/blood 38.
The owner remains unshootable, first/thinker head, floor-standing, with zero
physical momentum; direct source walking remains supported. Validate names,
all immutable metadata and reachable roles. Baseline action_supported must not
grant CPos actions to older profiles; Pos/SPos must not become executable in 12.

Original MT_CHAINGUY (type 10) has COUNTKILL, source pain/death/raise/full run
states, and a chaingun item-drop dependency in P_KillMobj. It remains rejected.
Actual player binding/damage, inventory/scoring, SpidRefire, other attacks,
missiles/splash, floating/doors/specials, demos and presentation remain excluded.

## Immediate Chase closure and preflight proof

Refire's exit must call the existing LifeHost enter_counted on see state 2,
executing genuine Chase immediately. Preserve JUSTATTACKED clearing/NewChaseDir,
target scans, movement/count changes, source relinking, active sounds and any
genuinely selected ATK1. Do not use an action-free marker or defer to another tic.

Reuse the existing owner/roster/sound/generation checks and map/query/work bounds:
maps +/-512, queries +/-1024, 8192 sight work, 128 intercepts, 64 actors;
owner direct/tic margins 1008/978 and non-owner tic XY 994, impact XY 994/Z 1018.
The owner is first and physically stationary, so its next state action's input
pose, target and RNG cannot change before that callback. Step preflight must
identify shots on expiry of states 4/5/7 and refire on expiry of state 6.
Direct calls and direct entry into 5/6/7 need the same preparation.

Each shot can reuse the existing single-shot MonsterShotPlan and trace/damage
reservation. Prepare only this action against current state, never future tics.
Refire needs only the existing read-only callback/entry validation: its facing,
draw, short-circuit sight and nested movement/state services are infallible once
the shared invariants below hold. It allocates no effects and needs no speculative
RNG or world projection. Health and sight are evaluated lazily after the threshold
test. All recoverable errors precede draws, state and movement; shot plans remain
ephemeral and no cross-tic plan is serialized.

Nested see -> Chase can reacquire and return, enter Look, or move/select ATK1.
Look resets threshold; an admitted Look -> see handoff can invoke Chase again.
At most two Chase countdowns and one successful source walking displacement
occur in this closure. Keep the existing INT_MIN guards and extra reaction guard
for that second decrement. Positive Look/Chase/attack tics bound immediate state
work well below ENTRY_LIMIT 128; newly selected ATK1 cannot immediately fire.
All candidate walking positions remain inside the existing speed 1..16 margin;
failed candidates do not cumulatively move the owner. No impact allocation is
needed by refire's nested closure. Existing pure bounded movement/sight services
have no late recoverable error once these invariants are validated. No full
Simulation/map/store clone, parallel scheduler or duplicate damage engine is needed.

## RNG threshold 40: attainable evidence versus boundary probe

The untouched 256-byte original rndtable NEVER returns 40 (or 38). It returns
39 at indices 72/242 and 41 at 114; 42 occurs at 71/130. Real gameplay tests
must select authentic table positions, including shadow draws BEFORE the roll.
An exact-40 gameplay-RNG scenario would be fabricated evidence.

Cover actual 39/41 draws, early null/dead returns, high-roll null/dead exits,
visible/blocked/REJECT sight, shadow ordering and nested handoffs with the genuine
original RNG. Separately compile an isolated original CPosRefire dependency probe
with an explicitly injected return value 40 (and boundary neighbours), preserving
the original decision body. This isolated probe uses recording FaceTarget/state-
entry doubles and scripted sight; the main gameplay harness uses the genuine
services. Compare its short-circuit outcome with the runtime's
pure decision helper. Label this controlled-dependency boundary evidence; it is
not an original-LUT gameplay trace and must not inflate gameplay comparison counts.
Do not add a production RNG override or modify original tables to manufacture 40.

## Independent validation plan

Compile genuine CPosAttack/CPosRefire, Face/Look/Chase/range/sight, trace/aim,
damage/kill/impact/state/thinker/RNG bodies with original structs/info/tables.
Instrumentation must retain actual control flow and short-circuit order. Record
source/header/function/adapter/input/output hashes, compiler, flags and ABI;
compiled inventories are not branch coverage. Report supplied versus exercised maps.

Cover independently aimed changing/moving targets, hit/miss permutations, death
between shots, real effects, source 10/4/4/1/fullbright chronology, continued and
exited loops, roster-versus-body health, nested movement/reacquisition/sound,
JUSTATTACKED, RNG wrap and retirement/reuse. Preserve all v1-v11 code/fixtures/
helpers. A new independent Python v12 encoder reads C/public inputs/schema only,
not Rust output; retain named fields, selected full bytes and digest diagnostics.
Existing events already encode shot/facing/sight/damage; add only necessary refire
decision observations, no persistent firing counter or cross-tic plan.

Rust safety tests separately cover capacity, invalid state/role/player/reference/
generation/arithmetic, complete canonical failure atomicity, pause, independent
instances and presentation reads. An asserted twelfth headless example and bounded
eleventh fuzz target must exercise the declared closure. Record only observed
counts, sanitizer settings, local checks and exact-head CI in VALIDATION and the
new PR, which stays open/unmerged for owner review. This establishes a bounded
action/fixture closure, not complete Chaingunner or vanilla/demo compatibility.

## Implemented result and observed evidence

Implemented the strict profile 12 and complete admitted action/return closure.
Current shots reuse the shared plan/executor; refire needs only read-only invariant
validation because its admitted sight/movement/state services are infallible and
its positive-duration nested path cannot fire immediately. No full state clone,
production RNG override or persistent attack plan was introduced. Null/miss/refire
can succeed at capacity; hits reserve their real impact before sound or draws.

The original-body harness compiles 67 bodies and executes 153 cases exercising all
20 supplied maps, including two new REJECT fixtures. Rust compares 268,606 actor
fields, 63,308 ordered observations and 10,697 raw RNG draws, including setup.
All five refire branches occur: 22 early, 10 null, 12 dead, 5 blocked and 109 visible
continuations. Source traces exercise immediate two-Chase/Look movement, independent
shots, target changes, pain/death, real impacts, RNG wrap and 166 retirements.
The controlled-dependency boundary probe separately covers 2,048 combinations;
original-LUT gameplay cannot produce 40. Its expected control flow is C-derived.

Independent v12 encoding checks all 1,544 captures and 110 full byte fixtures;
the eight-scenario asserted example ends at `54ac5f9c158c8e29`. Ten Rust policy
tests, 192 workspace/MSRV checks each, 168 release tests, twelve example comparisons,
eleven fuzz builds and the 10,000-execution ASan campaign passed; leaks were disabled.
Exact observed CI and preservation records belong in [VALIDATION](../../VALIDATION.md).
No complete original Chaingunner/vanilla/demo compatibility follows.
