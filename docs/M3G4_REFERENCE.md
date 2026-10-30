# M3g4 investigation: original player command movement and reaction time

This investigation is committed before runtime implementation. Preserved Linux
DOOM 1.10 is behavioral authority. Source readings and reused historical evidence
below do not establish new canonical-player execution comparisons. New compiled
observations must be recorded separately in the milestone evidence record.

COMPATIBILITY owns accepted inputs, STATE_FORMAT owns diagnostic contracts,
TESTING owns reproduction commands, and ROADMAP owns milestone status. Original
files, prior reference tools/helpers, fixtures and observations remain immutable.

Source references: [player commands and gate](../linuxdoom-1.10/p_user.c),
[ticker](../linuxdoom-1.10/p_tick.c), [mobj physics/state entry](../linuxdoom-1.10/p_mobj.c),
[collision and sliding](../linuxdoom-1.10/p_map.c), [BLOCKMAP/traversal](../linuxdoom-1.10/p_maputl.c),
[damage](../linuxdoom-1.10/p_inter.c), [definitions](../linuxdoom-1.10/info.c),
[identities](../linuxdoom-1.10/info.h), [player ownership](../linuxdoom-1.10/d_player.h),
[command types](../linuxdoom-1.10/d_ticcmd.h), [input clamps](../linuxdoom-1.10/g_game.c),
[fixed arithmetic](../linuxdoom-1.10/m_fixed.c) and [fine tables](../linuxdoom-1.10/tables.c).
The preserved source revision is a77dfb96cb91780ca334d0d4cfd86957558007e0.

## Source and dependency matrix

| Original source fact | Existing Rust service | Integration and evidence disposition |
| --- | --- | --- |
| p_user.c P_MovePlayer adds angleturn shifted by 16 before either directional thrust | command_thrust already uses updated binary angle, original fine tables and forward-then-side order | Reuse this arithmetic; make signed-to-unsigned bit interpretation explicit. Compare positive/negative turns, wrap and combined commands independently. |
| P_Thrust indexes angle by ANGLETOFINESHIFT and adds two FixedMul results; side heading is updated angle-ANG90 | Same command_thrust; fixed-point library and source FINE_SINE | No second physics calculator or floating point. Grounded admission bounds source arithmetic before actual shared execution. |
| d_ticcmd.h uses char forward/side and short angleturn; g_game.c caps directional commands to MAXPLMOVE=50 | Existing TicCommand has i8/i8/i16/u8 fields | Select directional -50..50, all signed 16-bit turns under the explicit angle interpretation, buttons zero and only source slot 0. GNU signed-char/16-bit-short reference ABI is qualified; no network-command decoding is added. |
| P_MovePlayer computes onground from mo.z<=floorz, applies forward then side, and enters S_PLAY_RUN1 only if directional input is nonzero and the current state is S_PLAY | Existing grounded world/movement; canonical player state closure; shared LifeHost entry | Require floor-grounded body. Enter the typed original RUN root through lifecycle services after thrust, before any thinker. Pain states do not get forcibly replaced by RUN. |
| P_PlayerThink first excludes PST_DEAD, then decrements nonzero mo.reactiontime instead of calling P_MovePlayer | weapons.rs apply_motion already has reaction gate and motion; ActorLife already stores reaction_time | Extract one checked reusable motion/gate service for the existing weapon path and new canonical path. A timer of 1 becomes 0 without moving/turning this tic; movement starts the next tic. No separate timer field. |
| P_Ticker processes active player stages before P_RunThinkers | Existing Phase::Players/Thinkers and bounded DemonPlan commit | Dispatch only the selected canonical command stage between those phases inside the existing transaction. The body remains unlisted; this is not complete active-roster P_Ticker execution. |
| P_XYMovement uses actual forward/side input in its strict low-speed stop condition, even if reaction time suppressed P_MovePlayer | Existing movement path receives optional TicCommand; stop hook uses command bindings or source mo association | Supply actual slot-0 command through exact mo association. Preserve the source condition instead of treating suppressed thrust as a zero command. Existing policies only accept zero inputs and retain their behavior. |
| P_XYMovement returns early for exact zero XY momentum; a blocked RUN body can therefore remain RUN on later zero input | Existing shared movement and had-XY stop hook | Preserve this exceptional chronology. Do not force PLAY simply because a body is stationary. Compare actual friction-driven stopping separately from blocked zero momentum. |
| P_DamageMobj clears reactiontime after surviving damage; lethal damage exits through P_KillMobj instead | Existing DamagePlan/LifeHost and source player health/death machinery | Observe damage before the player stage, cancellation of an initial delay, pain plus commands, and retained corpse momentum. Keep source and inflictor distinct. |
| P_MovePlayer/P_Thrust and the movement gate contain no gameplay RNG request | Existing owned RNG and event recording | No new draw. Independently compare RNG around existing spawn/damage/death and command execution; inventory is not branch coverage. |
| P_PlayerThink also rewrites MF_JUSTATTACKED, handles cheats, view, sector effects, weapons and counters; P_DeathThink handles camera/respawn | Selected historical mechanics do not establish these services for the genuine player | Reject unsupported command/settings/flag branches; omit these stages explicitly from this capability. Do not call them as silent no-ops or claim whole-player compatibility. |

## Arithmetic interpretation

The literal source expression `(cmd->angleturn<<16)` has undefined ISO-C behavior
for negative signed operands. Nonnegative short values 0..32767 produce a
representable signed result before addition to unsigned angle_t. Negative input
cannot acquire a portable original-C equivalence claim merely by compiling it.

Historical M2/M3a/M3b observers already use the explicit interpretation
`((uint32_t)(int32_t)cmd->angleturn << 16)`. Reuse this narrowly declared adapter.
Rust converts the sign-extended command's bytes to u32 and performs wrapping shift
and addition. The operation is a deterministic modulo-2^32 bit interpretation,
distinct from an ISO-C claim about the undefined expression.

New evidence must distinguish normalized negative-turn observations from a
defined positive-turn subset compared with the unchanged literal source body.
A separate source-expression sanitizer probe may diagnose the negative shift;
its undefined continuation must not supply gameplay expectations. P_Thrust and
P_MovePlayer were already compiled in historical observers; adding them to the
player-body observer is not globally new source-function coverage.

Within the selected directional and current-momentum bounds, command scaling,
FixedMul products and additions cannot overflow signed 32-bit source arithmetic.
Actual execution continues using the shared Fixed operations and original fine
tables. Wider integers are admission proofs, not substitute gameplay arithmetic.

## Selected runtime and ownership boundary

Add explicit PlayerCommandMovementV1 beside the two historical player policies.
It uses the same canonical MT_PLAYER, source player/mo record, actor store,
generational handles, execution scope, instance provenance and shared thinker pass.
There is no new player owner, persistent command copy, definition mirror or timer.
Source slot 0 is independent of the actor storage slot.

Medium/commercial settings, one player, grounded flat static axis-wall geometry,
existing capacities and unsupported roster/AI boundaries remain. Current generation,
definition, binding, health/state, flags, input ranges and physical support validate
before execution. Nonzero commands reject for unbound, pending, retired or dead
bodies. Zero inactive input may run the ordinary thinker/removal pass without
dereferencing an unavailable body or inventing P_DeathThink. This is explicit Rust
lifetime policy, not original dangling-pointer or full player-ticker equivalence.

Controlled initial reaction delay is an explicit bounded setup operation on the
existing body counter before the first tic. Default source spawning still sets
the pristine original value. This does not implement teleportation or invent a
second timer. Physical damage remains shared and can reset the source counter.

The command stage runs once in DemonPlan before all actor thinkers. Actual
commands also reach shared friction/sliding. Source running entry uses LifeHost,
and existing thinker countdown and stop-state entry retain their ordering.
Existing momentum is accumulated, not replaced. The same advance/step core and
late-failure transaction commit remain authoritative; no Snapshot is introduced.

## Coordinate and failure boundary

M3g3's potential P(M)=ceil(160M/3) relied on zero directional input for low-speed
stopping. It must not be assumed forward invariant when nonzero commands inhibit
that stop. Keep the historical proof and policy unchanged.

For this new policy, validate command-produced M<=30U and the conservative
axis-slide attempt margin abs(position)+4M+8 raw bits<=994U when M is nonzero.
For M=0, only the position bound is needed. After actual shared movement, validate
the existing P(M) envelope on the staged body before committing. This retains a
safe zero-command continuation for every accepted state while allowing commands
to be rejected at conservative support boundaries. It is a Rust admission policy,
not an original coordinate limit, clamp or second movement simulator.

Rejected command ranges, buttons, slots, references, arithmetic or late movement
results preserve actor/player/spatial state, reaction time, angle, RNG, statistics,
last/pending events and stored commands. Pause/error precedence remains historical.

## Independent evidence and diagnostics

Compile genuine P_Thrust/P_MovePlayer with the declared shift adapter plus the
verbatim movement conditional from P_PlayerThink. Dispatch the admitted live-body
gate before P_RunThinkers. Explicitly exclude full P_PlayerThink/P_Ticker, roster,
view, weapons, counter decay and P_DeathThink. Use actual canonical player and
map-spawned Demon identities; keep live Demon AI outside traversal.

Observe command inputs, stage boundaries, angles, XY/Z, momentum, reaction time,
state/timer/health, player association, collision/slide choices, actor order,
statistics and RNG. Prefer compact per-tic motion rows, selected complete actor/
player captures and a small set of full fixtures, with complete ordered events.
Report actual executed function entries separately from compiled inventory.

Existing DSIM-v22 fields already represent command/motion/state/reaction and the
existing Player event. Select explicit wire policy 4 and a new exact command
context schema; preserve old policy/context meanings, fields and fixtures. No
new DSIM version or event tag is justified. The new independent encoder must be
phase/command-aware while reusing frozen field/event primitives. It must not
change historical helper bytes or derive expectations from Rust.

Generation/provenance, inactive-input policy, arithmetic support limits, atomicity,
decoder rejection and advance/step equivalence are Rust safety/API evidence.
Source observations and independently encoded diagnostics remain separate.

## Exclusions and next gameplay prerequisite

Full PlayerThink/DeathThink, active roster, automatic Demon acquisition, weapons,
view/camera, inventory, powers/cheats, airborne controls, generalized geometry,
respawn, multiplayer and rendering remain excluded. The next separately validated
gameplay milestone should admit the active-player roster and original Demon
acquisition through these same shared services, not another gameplay engine.
