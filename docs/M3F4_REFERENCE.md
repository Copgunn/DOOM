# M3f4 investigation: original actor and state identity

Authority is preserved id Software Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. The initial investigation preceded
the identity implementation. [Compatibility](COMPATIBILITY.md) owns admission;
[testing](TESTING.md) owns commands; [evidence](validation/M3F4.md) owns observations.

## Original-C facts

`info.h` declares contiguous, zero-based `mobjtype_t`, `statenum_t` and
`spritenum_t` enumerations, ending in NUMMOBJTYPES, NUMSTATES and NUMSPRITES.
The terminal counts are not identities. `info.c` defines positional tables with
137 actor types, 967 states and 138 sprite names. MT_SERGEANT is 12; MT_SHADOWS
is 13. They share SARG state references but remain distinct types with different
flags. Sprite identity is an enum ordinal; its four-character resource name is
separate metadata, not loaded pixels.

Ordering is also used by code: g_game.c changes tics over the inclusive
S_SARG_RUN1 through S_SARG_PAIN2 range for Fast/Nightmare, and changes three
projectile speeds. Those runtime mutations do not change enum identities. The
canonical descriptors here represent pristine info.c initializers, not a snapshot
of a Fast/Nightmare-mutated C table. Those gameplay modes remain excluded.

A state contains sprite, frame, tics, action union, nextstate and misc1/misc2.
Frame includes the 32768 fullbright bit. Original frame/tics/misc fields are C
long, not necessarily host int. Values in this table fit i32; the new observer
uses matching printf formats and verifies representability. Actor records contain
23 int properties, including spawn, see, pain, melee, missile, death, extreme death
and raise state references. Zero in these references means S_NULL, including
absent optional attack/raise entries. It must not be renumbered or conflated with
an invalid identity. DoomEd number -1 means ordinary THING lookup exclusion;
it is not an invalid actor type.

Action pointers have no original numeric ordinal. d_think.h defines a union of
zero-, one- and two-parameter function pointers. P_SetMobjState calls acp1 for
mobjs; p_pspr.c uses the weapon context. Symbol equality records association, not
interchangeable signatures or granted execution. The existing alphabetical Rust
ActionId table is project policy. The new descriptor exposes symbolic names and
NULL separately; it never serializes a C address or invents an original action
ordinal. NUM* sentinels and thinker function pointer -1 are different namespaces.

P_SpawnMobj assigns type/info and raw spawn state without invoking an action,
consumes lastlook RNG, links spatially, then appends its thinker. P_SetMobjState
checks S_NULL before loading a descriptor, installs sprite/frame/tics before the
immediate action, and follows the entered descriptor's nextstate while the
current timer is zero. Nested actions can change that timer/state. P_MobjThinker
runs XY/Z before decrementing non-infinite tics; -1 stops ordinary transitions.
An initial zero timer decrements to -1. A terminal -1 state with next S_NULL
persists. p_tick.c preserves append-sensitive thinker traversal. These existing
semantics must remain untouched; see [M3c](M3C_REFERENCE.md).

p_enemy.c Look/Chase selects see/spawn/melee/missile references dynamically;
A_VileChase additionally depends on raise, corpse context and resurrection.
p_inter.c selects pain/death/extreme death and tests actor type in combat.
Recognizing those references cannot prove their behaviors. Existing
[M3f3](M3F3_REFERENCE.md) qualifications and all executable boundaries remain.

## Rust policy and chosen scope

The existing source_actors table already contains complete C-derived metadata.
Reuse it without a second persistent table, registry allocation or scheduler.
Introduce separate bounded OriginalActorType, OriginalState and OriginalSprite
identities with private storage, exact source-name constants, checked signed and
unsigned conversions, exact name lookup and deterministic ordinal iteration.
Provide named read-only actor/state descriptors. All canonical table records are
represented; this milestone admits no new gameplay closure or DSIM profile.

ActorTypeId and StateId remain registry-local candidates for backwards
compatibility. Explicit conversions require the complete record at its original
slot to match canonical name and every field. Renames, altered properties,
callbacks, sprites, next references and native fixture replacements reject.
Reverse conversion also rejects out-of-range/native-only slots. These are local
record checks, not proofs that every downstream registry record or game context
is original. Returned registry IDs still pass existing execution admission.
No implicit From conversion crosses namespaces. Existing callers and snapshots
retain their identities and behavior; type136/state aliases are not silently
relabeled MT_SERGEANT. A read-only query cannot mutate RNG, ordering or snapshots.

This is the smallest useful foundation: complete identity/descriptor recognition
and explicit integration with the owned registry, with executable props exercising
existing state services. Migrating the Demon fixture to original type12 would
require a separate profile, projection/schema and game-context audit. That is a
recommended next narrow investigation, excluding raise, players and Spectre
execution rather than forcing behavioral migration into identity infrastructure.

## Independent evidence plan

Preserve every historical tool and fixture. Continue the original compiled-C M3c
table comparison. Add a separately versioned DOOM-IDENTITY-v1 observer compiling
untouched info.c/h, printing enum *symbol values*, table fields, symbolic action
pointer matches and sprnames. Aborting action stubs are metadata-only; they do
not implement callbacks. Rust constants are derived from header declaration order,
while the C compiler evaluates actual symbols and initializer expressions.
No Rust output supplies expectations. Compare all records/fields and round trips;
label rejection, registry mismatch and observation purity as Rust policy tests.
Existing independent DSIM-v1–20 fixtures prove executable chronology/RNG retention;
no new capture schema or historical rewrite is needed for read-only identity APIs.
