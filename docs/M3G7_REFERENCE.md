# Original-player pistol execution investigation

Authority is the preserved Linux DOOM 1.10 source. This investigation separates
source observations, shared Rust services, explicit execution limits and Rust
safety policy. Metadata recognition alone grants no weapon execution permission.

## Source and integration matrix

| Source behavior | Existing service / required integration | Disposition and evidence |
| --- | --- | --- |
| `g_game.c:G_PlayerReborn` owns fist+pistol, selects pistol, gives50 clips, sets attackdown; original player index owns `mo` | OriginalPlayer already owns the genuine body, health, state and attacker | Reuse that owner. Add only dynamic pistol fields; compare original initialization. Fist metadata is not fist execution. |
| `p_pspr.c:P_SetupPsprites/P_BringUpWeapon` clear overlays, set bottom128 and immediately enter `S_PISTOLUP` | Existing weapon Machine implements nested psprite entry | Share the interpreter. Initial Raise action immediately movesY to122; ready is reached on tic15. |
| `P_SetPsprite` executes state actions before returning and follows zero-tic transitions | Existing immutable canonical state registry and psprite interpreter | Preserve nested ordering. Null entry clears the state pointer while retaining timer/coordinates. `misc1` controls both source coordinate assignments. |
| `p_user.c:P_PlayerThink` command/reaction gate and `P_CalcHeight` precede `P_MovePsprites`; `P_Ticker` then runs thinkers | Existing command stage, fixed-point bob and shared thinker scheduler | Run command stage once, then shared weapon stage, then existing thinkers. Harness executes actual CalcHeight but compares only admitted bob, not camera/view compatibility. |
| `A_WeaponReady/P_FireWeapon` test ammo, enter bodyATK1, enter pistol attack state, then callNoiseAlert | Existing shared Machine plus actual current body-state entry | BT_ATTACK admission is newpolicy7 only. Attack selection is not an immediate bullet. Keep exact nested event ordering. |
| `S_PISTOL1..4` are13..16 with4/6/4/5 tics | Canonical state descriptors | FirePistol runs on entering14. ReFire runs on entering16 and can immediately replace it with13. Held fire interval is14 tics, not19. First continuous request fires on tic19. |
| `A_FirePistol` requests sound, enters bodyATK2, consumes clip, entersFlash17, aims, shoots | Existing Machine, sound/event services, shared trace/combat | Consume ammo at the firing action. Process weapon then flash; a newly entered flash also decrements on its birth tic. Compare state ordinals/timers/coordinates, light and ammo. |
| `A_ReFire` increments refire only for held attack, no pending weapon and positive health; otherwise resets and checks ammo | Shared Machine currently assumed always-live historical player | Supply current authoritative health/death through a narrow host adapter. Release during windup does not cancel an already selected shot. |
| `P_GunShot` draws damage5*(random%3+1), then two spread draws if inaccurate | Shared weapon RNG/shot service | Values5/10/15, initial refire0 accurate, subsequent refire spread. Preserve damage-before-spread order. No new RNG stream. |
| Source spread subtracts two calls and shifts a possibly negative signed value | Existing qualified historical source adapter | Explicit left-before-right draws and representable multiplication define the recorded GNU bit-pattern domain. Do not claim portable unmodified ISO-C equivalence. |
| `P_BulletSlope` tries current angle,+2^26,-2^26 at1024 units | Existing bounded Aim/intercept service | Reuse the actual collector/window algorithm on current staged actors. Fallback chooses slope only; it does not turn the actual2048-unit bullet. |
| `P_LineAttack` traverses source BLOCKMAP/intercepts, can miss or stop on a wall | Existing combat_trace and attack execution | No nearest-target shortcut. Compare candidate/intercept order, final hit and full trace payload. Aim queries are read-only evidence, not invented monster-aim gameplay events. |
| `P_SpawnPuff/P_SpawnBlood` consume jitter/spawn/expiry RNG and create real thinkers | Existing shared effect allocation/lifecycle | Reserve bounded capacity before infallible spawn. Effects created before thinker traversal must think that same tic. Compare effect identities and birth/order. |
| `P_NoiseAlert/P_RecursiveSound` set sector.soundtarget | Existing sound-target storage and source scanner | Admit one flat sector with genuine one-sided sector line lists. Update the actual body target before A_Look. No general sound propagation; ambush retains its sight requirement. |
| `P_DamageMobj` applies mass-scaled thrust, health, pain, reaction reset and source retargeting | Existing DamagePlan/LifeHost | Use actual current Demon and player, with source=inflictor=owned player for bullets. Extend only newpolicy7's validated closure. Demon mass400 differs from player mass100. |
| `P_KillMobj` updates source player.killcount and retains a Demon corpse | Existing level single-player kill counter and corpse service | Reuse one authoritative counter, not a duplicate pistol/player statistic. Preserve source death state, flags, height and RNG. |
| Player death setsPST_DEAD and callsP_DropWeapon before body death state/jitter | Existing shared player death service; historical weapon interpreter lacks genuine death | Invoke shared lowering immediately at that boundary. Dead continuation executes only the source MovePsprites portion, never firing or raising again. This is not complete DeathThink. |
| Empty ammo selects an available fallback, eventually fist | Existing historical interpreter supports additional weapons | Observe real CheckAmmo fallback independently as a component, but reject its unadmitted continuation atomically in policy7. Zero-ammo idle is valid; no infinite ammunition or silent fist execution. |

## Ownership and execution boundary

`OriginalPlayer` remains authoritative for body, health, state, attacker and
lifetime. The actor store remains the sole mobj owner. Optional player-owned
pistol state contains bullets, attackdown, refire, bob, extralight and two
psprites. It contains no body handle, player slot, health or lifetime token.
Immutable pistol ownership/readiness/capacity are policy constants.

The historical WeaponPlayer remains unchanged for its own profile. A transient
adapter may supply the shared interpreter with current authoritative ownership
and health, then return only weapon-specific fields. It is never persisted as a
second player and cannot rebind a body. No second scheduler, combat engine,
movement implementation, actor store or RNG is introduced.

The new execution capability composes the existing canonical player commands,
fixed active roster, one map-spawned MT_SERGEANT, pursuit and autonomous melee.
Older capabilities retain their old button/error/diagnostic contracts. Original
player attack states154/155 are newly admitted only for this weapon capability;
psprites10..17 remain overlays, never extra actor instances.

The admitted map is static, flat, one-sector geometry with existing axis-aligned
one-sided walls and at most128 traceable lines. Existing actor, coordinate,
momentum, travel, sight and work limits remain. Default commands are required for
other player slots and after player death. No automatic rebirth or roster change.

## Arithmetic and source qualifications

All new source scenarios use nonnegative turn commands and map angles0 or-180.
The initial scout used+180 and strict arithmetic correctly diagnosed the known
signed overflow in `P_SpawnMapThing`. Changing only those uncommitted new inputs
to-180 preserves the west-facing BAM value while staying in the existing defined
signed multiplication domain. No new spawn-angle adapter or runtime bound is
needed. The failed scout and corrected raw observations are retained separately;
historical evidence is never changed.

Negative command turns retain the separately established M3g4 qualification;
they are not newly established by these nonnegative-input C scenarios. Existing
ordered random/jitter adapters retain their stated limits. Grounded actors have
zero height difference, so reverse-thrust RNG is unreachable here.

## Transactions and diagnostics

Use the existing bounded mutable transaction projection. Weapon state, ammo,
soundtarget, RNG, impacts, damage, retargeting, player commands, thinker actions,
spatial order and event queues commit together. Late trace, allocation, impulse,
unsupported fallback or AI errors discard the entire staged tic. No full-world
clone, release-only rollback exception or implicit Snapshot is permitted.

Persistent pistol fields require a new DSIM23 player schema and policy7 context.
Preserve all historical20/21/22 readers and bytes. Encode only dynamic weapon
state, using original psprite ordinals and existing weapon/combat event payloads.
Diagnostic decoding observes state; it does not restore an executable world.

## Evidence categories and unsupported work

Compiled original-C observations establish admitted initialization, psprites,
ammo, cadence, aim/trace, effects, noise, damage, pain/death and actor ordering.
Independent Python encoding establishes the declared diagnostic representation.
Rust tests separately establish generation/provenance safety, unsupported-input
rejection, rollback, bounded arithmetic and step/advance equivalence. These are
different claims, not interchangeable coverage.

Source harness sequencing is an explicitly bounded command/weapon stage before
original thinkers, not complete P_Ticker/P_PlayerThink/P_DeathThink. View, camera,
damagecount decay, powers, armor, cheats, other weapons, pickups, general sound
propagation, multiplayer, respawn, dynamic sectors and generalized infighting
remain unsupported. Source noise and direct damage retaliation are distinct.
No all-inactive player scan, freed-pointer dereference or proprietary asset is
used. Compiled-function inventory is not branch coverage or globally new source
coverage; actual entries are counted independently.
