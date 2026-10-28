# M3d3f source investigation: Demon melee

Recorded before runtime changes. Authority is untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. Revalidated against merged master
`5b9182472123b8ec86b48cf6a4076426a7856f70`, after PR #15's authorized regular
merge. Ordered parents are `31b4830945a60c837065c0e3c29d10494e84f20e` and
`08e27f72ee039e7dc0e20c26777f0ad09d9c3391`; tree
`4daf89e7f287430e3a852edf82cb80ceb577d02d` equals the recovered head tree.
[Recovery CI 40733786524](https://github.com/Copgunn/DOOM/actions/runs/40733786524)
and exact [post-merge CI 40735017337](https://github.com/Copgunn/DOOM/actions/runs/40735017337)
passed Linux/Windows/macOS/Rust 1.85 before this branch or runtime changes.
The macOS jobs executed on supported Intel macOS 26 after documented arm64
hosted acquisition/capacity failures. All 165 original files, 730 existing
reference paths and original history were verified unchanged; the repository
remains public/GPL-2.0-only. No private repositories or IWADs were accessed.
The bounded implementation and evidence below are a plan until validated.

## Source identity and animation

Inspected p_enemy.c, p_inter.c, p_mobj.c, p_tick.c, p_map.c, p_maputl.c,
p_sight.c, p_local.h, info.c/h, m_random.c, original fixed/angle helpers, and
G_InitNew in g_game.c. Reuse the accepted [damage](M3D1_REFERENCE.md),
[attack/range](M3D3C_REFERENCE.md), [shotgunner](M3D3D_REFERENCE.md) and
[chaingunner](M3D3E_REFERENCE.md) service contracts. Original files remain unchanged.

MT_SERGEANT is Demon/type 12, not Shotgun Guy/type 2. Original info.c:1420 has
Doom number 3002, health 150, speed 10, radius 30, height 56, mass 400, reaction
8, attacksound sfx_sgtatk (52), no missile state or extreme-death state, and
COUNTKILL. Its see/active sounds are sfx_sgtsit (41)/sfx_dmact (77). Original pain,
death and raise dependencies are separate. MT_SHADOWS/type 13 adds SHADOW to the
same Demon definition, with Doom number 58; complete Spectre execution stays out.

Raw info.c:621 defines SPR_SARG (39), without fullbright attack bits:

| State | Frame | Tics | Action | Next |
| --- | ---: | ---: | --- | --- |
| ATK1 | 4 | 8 | FaceTarget | ATK2 |
| ATK2 | 5 | 8 | FaceTarget | ATK3 |
| ATK3 | 6 | 8 | SargAttack | RUN 1 |

Two separate facing entries precede the actual strike by 16 tics. SargAttack
faces a third time. Each non-null SHADOW target consumes two actual gameplay draws
at each facing; do not reuse an angle or pre-draw the complete animation. Source
P_MobjThinker updates XY/Z before decrement/entry. Real P_SetMobjState actions
run immediately, and return to supported Chase can select another positive attack
state in that same callback without immediately striking.

G_InitNew (g_game.c:1423) halves SARG_RUN1..SARG_PAIN2 timers for fast/nightmare.
This profile promises the unmodified normal 8/8/8 table. Global fast/nightmare
initialization is outside the admitted fixture; reject Skill::Nightmare here
rather than imply complete original Nightmare Demon behavior. Older profiles and
skills remain unchanged. Full source run/pain/death/raise animation is not admitted.

## Exact action, range and sound order

The real p_enemy.c:935 body returns immediately on null; otherwise FaceTarget,
P_CheckMeleeRange, then only on success ((P_Random()%10)+1)*4 followed immediately
by P_DamageMobj(target,actor,actor,damage). No LineAttack, impact spawn, ammo,
noise alert, attack sound or modern target-health test occurs in this action.

Original Chase performs an earlier, independent melee-range check. On selection
it emits the metadata attacksound BEFORE entering ATK1, then returns before normal
movement/active sound and before missile logic. Melee does not set JUSTATTACKED.
The eventual strike checks current range/sight again, after the target may move,
change or die. A later miss preserves the earlier selection sound and animation.

P_CheckMeleeRange uses P_AproxDistance and rejects distance >=44*FRACUNIT plus
TARGET INFO radius, then calls the original REJECT/BSP P_CheckSight. Collision
radius is not the metadata radius. Retain strict raw fixed-bit neighbors,
negative coordinates and source short-circuit order; no Euclidean replacement.
Sight does not depend on facing angle/AMBUSH, so a pure preview can validate it
before real FaceTarget while preserving authoritative observation order.

Successful damage is 4,8,12,16,20,24,28,32,36 or 40 from the actual original LUT.
The LUT's absent value 40 is irrelevant to damage 40: a raw draw 149, for example,
produces 40. Null uses no facing/range/damage RNG. A miss still faces (and draws
shadow jitter), but does not draw damage. In-range dead/nonshootable targets still
draw damage and call P_DamageMobj, whose original early return ignores them.
Perception roster health is never a replacement for mobj health or shootability.

## Shared direct damage and allocation

Reuse the existing DamagePlan and LifeHost apply_damage implementation, exposing
only the necessary internal visibility and a spawn-demand accessor. No second
knockback, pain, kill or retargeting implementation is needed. Both source and
inflictor are the native owner. Retain source thrust angle, FixedMul, low-damage
lethal fall-forward draw, pain draw/state, reaction reset, target/threshold,
kill flags/quarter height, ordinary/extreme target state and death timer draw.

The strict source-player-body target closure allocates no actors. Do not reserve
a hitscan impact or emit blood/puff merely for melee. Reserve only actual damage
state demand (zero here), allowing successful melee at capacity. Dummy bodies
retain original player body states without player_t; original Demon pain/death/
raise and scoring are distinct unsupported services. A scripted remove/retire/
spawn scenario may exercise real generations, but is not autonomous Demon respawn.

## Admission and failure-atomic design

Use explicit profile 13 / DSIM-v13, labelled native 136 FIXTURE_DEMON_MELEE_136,
positive looping SARG Look/Chase states 1/2 and exact copied attack states 4/5/6,
next 5/6/2. The owner's labelled source-player-body definition explicitly adapts
Demon health 150/mass 400, normal defaults speed 10/radius 30/height 56, melee 4,
missile 0 and attacksound 52. Supported unbound target 135 keeps its source-body
pain/death/see closure and mass 100. Original metadata outside these adaptations
stays immutable. Original Demon 12/Spectre 13 are recognized but not executable.

| Capability | Admitted status |
| --- | --- |
| Original Demon/Spectre metadata | Recognized, full actor execution rejected |
| Named native fixture | Exact metadata/roles and positive state closure |
| Look/Chase/static walking | Existing source services |
| Melee selection and sound | Actual source priority and handoff |
| SARG animation and direct strike | Three distinct faces, 8/8/8, genuine range/damage |
| Target pain/death | Existing unbound source-player-body slice |
| Full Demon pain/death/raise/Spectre | Deferred |
| Player binding/damage, other attacks | Rejected before mutation |

Keep one unshootable first thinker-head owner, floor-standing with zero physical
momentum; direct walking remains supported. Preserve maps +/-512, queries +/-1024,
8192 sight work, 64 actors, owner direct/tic margins 1008/978, non-owner tic XY 994,
existing Z/height/momentum and nested reaction-counter guards. Unsupported callbacks
must reject, never no-op. New damage cannot recursively invoke attack callbacks.

Preflight copies only owner/RNG, validates identities/roles/roster/sound and entry
closure, previews internal shadow draws and current pure range/sight, and on a hit
previews the one damage draw and existing damage plan. All recoverable errors,
including a later bound-player rejection, precede real facing/flags/RNG/events.
Actual execution uses the original order: Face, call the existing melee helper
again against the current host, range/sight observations, damage draw, shared
direct damage. The plan retains only the accepted boolean and optional shared
damage plan; it asserts agreement with the recomputed range result. Sight ignores
facing angle/AMBUSH, and no intervening thinker can change geometry or targets.
Both preview and actual queries retain the existing per-query work bound. This
keeps actual query chronology explicit without caching a sight trace across tics.
Null returns need no plan. Step preflight prepares only
state 5 expiry into strike 6; direct entry 6 and direct calls share that preparation.
No cross-tic cached result or full Simulation/map/store clone is required.

Owner first/zero physical momentum means no earlier player/thinker can invalidate
the prepared action. New target damage changes momentum before its later thinker;
existing source XY clamps to 30 before movement, retaining the established 994
margin. Damage<=40 and target mass 100 keep integer thrust bounded; fall-forward
requires damage<40 and has at most 18 units of thrust. Fixed additions retain the
existing explicit wrapping semantics. Immediate Chase/Look paths remain bounded
by positive tics, with no immediate recursive Sarg action after return to Chase.

## Independent evidence plan

Compile actual original Sarg/Face/range/sight/Chase/damage/kill/state/thinker/RNG
bodies with original structs/info/tables/fixed helpers. Reuse existing harness
builders without modifying their code or outputs. Instrument original control
flow; ordered multiplication retains the established signed-shift/operand-order
interpretation for shadow RNG. Record source/header/function/adapter/input/output
hashes, GNU flags, ABI and compiler. Compiled inventory is not branch coverage.

Main cases will cover all ten actual-LUT damage values, null/miss/hit/dead and
nonshootable targets, strict raw radius boundaries, blocked/REJECT sight, source
selection sound, all three shadow faces, selection-versus-strike changes, direct
knockback/pain/death/extreme/fall-forward target effects, repeated cycles and RNG
wrap. Use the inherited public 20-map inventory and report actual exercised maps.
A scripted lifecycle case compares source retirement against real lowest-free
slot/generation reuse without claiming that direct melee spawns an effect.

Separately compare original P_CheckMeleeRange with metadata radius different from
current collision radius in component vectors, using real source sight. Private
Rust actor-copy tests can vary the collision field without adding a production
radius override. Keep component counts distinct from full gameplay capture counts.

The independent v13 encoder consumes original-C rows/public setup/schema only,
never Rust output. Existing event layouts express the complete chronology;
add no persistent result/counter or unnecessary transient tag. Preserve fields,
full bytes, independent digest and first-field diagnostics. DSIM is a project
diagnostic, not a vanilla demo/savegame/network format. All v1-v12 code/fixtures/
encoders/helpers remain unchanged; reference README prose may be updated.

Rust policy tests separately cover capacity, invalid references/roles/players,
unsupported metadata/callbacks, arithmetic/work and full failed-operation/tic
canonical equality. A thirteenth asserted example must prove pursuit, sound,
three faces, hit/miss, damage and return against C and independent bytes. Build
all twelve fuzz targets and run bounded Demon ASan with explicit leak settings.
Record observed counts and exact CI only after completion. The new PR stays
open/unmerged for owner review; complete Demon/Spectre and later gameplay remain
outside this milestone.

## Implemented result and evidence families

The implemented profile follows this investigation. The private specialized-action
enum extends admission without granting Sarg to earlier profiles. Existing shared
melee/Face and LifeHost damage services execute the action; only internal damage
planning visibility and spawn-demand access changed. The actual range query runs
after real facing, and its accepted result agrees with the small preflight plan.

The exact owner adapts source player body fields with Demon health 150/mass 400,
melee 4/missile 0/attacksound 52. It is deliberately unshootable and omits full source
COUNTKILL/scoring and original Demon pain/death/raise behavior. Targets retain
unbound source player-body states; their pain/death does not establish full Demon
pain/death. Normal default Look/Chase timers 10/2 may vary 1..35 only in these named
fixture loops. Attack 8/8/8 and frames 4/5/6 cannot vary. Public combat/binding and
other attack callbacks remain excluded in profile 13.

The harness compiles 68 original bodies and executes 104 gameplay/driver scenarios
on all 20 supplied maps, enforced by generator and Rust assertions. The main
comparison covers 170,118 actor fields and 5,060 raw RNG draws including setup and
discards. Its 27,039 ordered instrumentation rows contribute to independently
encoded observations/visits; not every raw row maps one-to-one to a DSIM event.
There is no injected gameplay RNG, no LineAttack and no impact birth. Separate
36 component vectors exercise info-radius/current-radius divergence with genuine
original range/sight bodies; these do not add to gameplay/canonical totals.

The independent encoder produced 1,309 v13 captures and 169 complete byte streams
before Rust comparison. Every capture matches, with named-field diagnostics for
full streams. The nine-scenario asserted example matches source pursuit, selection
sound, timing/facing/hit/miss/death/retirement and independent digest
`b6c312bf5fbcc7dc`. Nine public policy tests and three private component/late-error
tests separately prove the declared resource/reference/player/atomicity policies.
All historical v1-v12 source/expected files and eleven helpers are preserved.
Full local/fuzz/CI results and immutable reviewed heads are recorded in
[VALIDATION](VALIDATION.md) after execution. The new PR remains open/unmerged.
