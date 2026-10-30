# M3g2 investigation: original player body, ownership, damage and death

This source investigation precedes runtime implementation. Preserved Linux DOOM
1.10 is behavioral authority; source paths below refer to the untouched upstream
files. This note owns the audit/design. COMPATIBILITY owns supported inputs,
STATE_FORMAT owns diagnostic schemas, TESTING owns reproduction commands and
validation/M3G2 owns actual measured results. Observations planned here are not
completed compatibility evidence.

## Source and dependency matrix

| Source behavior and symbols | Existing Rust consumer | Disposition and independent evidence required |
| --- | --- | --- |
| info.h/info.c MT_PLAYER=0; S_PLAY=149; health100, radius16, height56, mass100, painchance255; MF_SOLID/SHOOTABLE/DROPOFF/PICKUP/NOTDMATCH | Complete canonical registry; synthetic UnboundBody137 deliberately changes health/flags | Admit pristine original0 in a separate explicit execution policy. Never relabel137. Reuse complete independent metadata evidence; add actual source0 behavioral observations. |
| p_mobj.c P_SpawnMobj allocates, assigns source properties/state without executing spawn action, links spatially, appends thinker, draws lastlook | make_actor/LifeHost::spawn/ThinkerStore | Reuse allocation, original IDs, RNG and event chronology. Source body initialization observed independently. |
| P_SpawnPlayer establishes mobj.player=p, mobj.health=p.health, then p.mo=mobj/PST_LIVE and counters | Four command bindings, synthetic perception roster and legacy WeaponPlayer are separate historical contracts | One bounded source player record references the existing generational actor. Derive reverse association from exact mo handle. No second store or persistent per-actor player ID. Observe extracted association/initialization fragment with declared fresh player health100. |
| P_SpawnPlayer additionally does reborn, THING orientation, psprites, HUD/status, viewheight and optional deathmatch inventory | Partial historical fixture mechanics only | Full player-start loader/reborn/view/inventory are excluded. Explicit floor spawn with supplied fixed XY and zero angle is the admitted initialization input, not complete P_SpawnPlayer equivalence. |
| d_player.h health is distinct from mobj.health; mo and attacker are persistent pointers | No genuine persistent player record | Store only mo, PST_LIVE/DEAD, health, attacker, damagecount and one frags counter. Existing level.single_player_kill_count supplies players[0].killcount. Distinct health is source-required: player clamps0, mobj may become negative. |
| p_inter.c P_DamageMobj rejects nonshootable/dead bodies before mutations or RNG | Shared DamagePlan/LifeHost::apply_damage | Preserve ignored repeated damage; validate request references before execution as explicit Rust safety policy. |
| Baby difficulty halves damage before thrust; inflictor thrust precedes player armor/god branches; chainsaw can suppress it | Shared thrust and XY/Z exist, but genuine player movement/weapon/settings dependencies are unadmitted | Select Medium, commercial single-player, armor/powers/cheats absent and no physical inflictor. Reject unsupported inputs before mutation. No approximate knockback or silent Baby behavior. |
| Damage player branch subtracts/clamps health, assigns attacker=source including NULL, adds damagecount capped100, requests I_Tactile, then subtracts mobj health | Existing shared damage/health/pain/retarget events | Add bounded source player update within that same shared transaction, before actor-health update. Observe presentation request at explicit I_Tactile service boundary; no device effect. |
| Nonlethal damage always rolls P_Random for pain; player chance255 means draw255 does not pain; reaction becomes0; retarget source when threshold allows | Existing source pain, reaction, threshold100 and retarget services | Reuse; independently test zero/repeated/self/null/source damage, pain and nonpain255. NULL clears player.attacker but does not clear mobj.target; self sets attacker without self-retarget. |
| p_inter.c P_KillMobj clears shootable/float/skullfly/nogravity, sets corpse/dropoff, quarters height | Shared kill service | Reuse source state/flags with player-specific immediate MF_SOLID clearing. Independently observe normal/extreme death and retained allocated corpse. |
| Source-player kill credit, single-player fallback credit and frags differ | Existing LevelStatistics fallback kill credit | Reuse existing single-player kill count; bounded player frags increments for self/environment player deaths. Check overflow before commit. Observe genuine source-player credit for killing an existing Demon through controlled direct damage. |
| Player death sets PST_DEAD and calls P_DropWeapon before death-state entry; automap may stop | No genuine player death branch; basic body actions already implemented | Add source player dead transition. Compile genuine P_DropWeapon and observe its P_SetPsprite request for fixed pistol-down state. Actual psprite/lowering/camera execution excluded; automap is false. This is an explicit service boundary, not full weapon execution. |
| Death selects xdeath only for health < -spawnhealth; P_Random&3 death timer jitter clamped1 | Shared death selection/timer code | Observe -50/-51 scream threshold and -100/-101 normal/extreme boundary; no invented RNG draw. |
| p_enemy.c A_Pain/A_Fall/A_PlayerScream/A_XScream and info.c player pain/death graph | Existing implemented callbacks | Reuse actual original states149–153/156–173, derive permitted closure from roots; exclude weapon-attack154/155 and arbitrary manual pain/death entry. Commercial A_PlayerScream uses high scream only below -50. |
| p_tick.c P_Ticker calls P_PlayerThink for playeringame, then P_RunThinkers; p_user.c P_PlayerThink/P_DeathThink change view, damagecount, commands, weapons and reborn | One authoritative advance core and shared thinker pass | Select unlisted player body (playeringame=false) with genuine player_t/mobj association. No P_PlayerThink, death thinker, damagecount decay, camera, input movement, attacks or respawn claim. Reject nondefault command inputs. |
| p_enemy.c A_Look/LookForPlayers uses active roster; A_SargAttack uses target and physical inflictor | Existing canonical Demon closure | Keep roster inactive. A canonical Demon may be explicit direct damage source without physical inflictor; owned player may directly kill a Demon. Automatic player acquisition/melee/infighting are excluded, not inferred from identity resolution. |
| p_map.c/p_maputl.c collision/list order; p_mobj.c physics-before-countdown | Existing spatial ownership and thinker core | Player remains floor-grounded with zero momentum in an admitted flat static one-sector context. Reject momentum changes and unsupported geometry; keep same services/order, no player-specific simulation loop. |
| P_RemoveMobj unlinks immediately and defers removal through P_RemoveThinker/P_RunThinkers | Existing pending removal/generation advancement | Death is not removal. Retain mo/attacker stored identity after logical retirement; derive availability from existing store. Never dereference retired C pointers; quarantine/opaque identity observations are separately qualified. |
| C dangling-pointer/address reuse assumptions | ContextActor/ActorAvailability/GameInstance provenance | Rust-only safety: stale generation cannot bind a new occupant, cross-instance import rejects, pending is observable but not damage/bind permission. No unsafe C pointer emulation. |

## Representation and mutation boundary

Preserve the public historical Profile enum and its exhaustive callers. Its
OriginalDemon variant identifies the existing original-ID service family; add an
explicit OriginalExecutionProfile::PlayerBodyLifecycleV1 policy (wire2) rather
than repurposing DemonNormalV1. Constructor and registry capability select it
explicitly. A bounded optional source player record identifies that selected
policy in Simulation; no duplicate profile boolean or actor definition fields.
Historical Demon policy remains unchanged and never gains MT_PLAYER permission.

The source player record uses the same ActorId/mo, context brand, instance envelope,
actor store and scheduler. Bind only an initial live canonical0 body, once, through
a checked current handle. Player slot0 and actor slot are different namespaces.
Expose the checked operation through ExecutionContext; GameInstance imports still
check provenance first. Stored mo/attacker observations are not liveness leases.
Retirement/reuse does not silently clear or replace them. A fresh player body is
not respawn, and a dead/retired binding cannot be rebound in this policy.

Damage continues through the one DamagePlan and LifeHost service. Copy the small
player record into the existing bounded staged actor plan and commit it only with
actors/RNG/statistics/events after late failure checks. Plan source-specific frags
and supported damage inputs before mutation. Existing non-player contexts have no
player allocation, body, roster work or diagnostic capture. Measure any scalar
staging/branch cost instead of assuming it negligible.

Direct external player state/health/pursuit/momentum mutation rejects, as do
physical inflictors, unsupported sources, unsupported skill/settings, new roster
participation and restart/respawn. Shared damage controls pain and death; thinker
countdown controls the admitted source graph. Invalid requests preserve all fields,
RNG and pending/last event order. Paused advance retains its earlier no-op precedence.

## Diagnostic and independent evidence decision

Real mo/attacker/player health/state/frags fields are observable changes, so a new
DSIM-v22 contract is justified. Explicit policy2 and a distinct player context
schema preserve original registry identity. Record actual player fields and a
derived mobj association, with no repeated immutable metadata or provenance key.
Retain v20/v21 bytes/readers and their rejection of unknown22. A separately named
v22 entry point must share the bounded framing parser and validate the new policy.
Decoding remains read-only and never restores executable worlds.

Extend the independent genuine-body C observer through a new bounded driver,
keeping all old tools and evidence immutable. Inputs use actual MT_PLAYER0 and
MT_SERGEANT12 with no fixture-specific player aliases. Record declared source
initialization fragment, player damage/death fields, action/state/RNG/thinker order,
presentation service requests and safe logical retirement. Report actual executed
function entries separately from the inherited compiled inventory and new body
count. Expected complete diagnostics/events come from C rows and independently
specified schema policy, never Rust output. Keep compact distinct cases, selected
full fixtures and complete ordered-event comparison; no broad trace expansion.

Rust-only tests cover slot/generation/provenance/role failures, pending and retired
references, double binding, rejected-operation atomicity, frags bounds, explicit
unsupported input rejection, replay and advance+snapshot/step equivalence. Reuse
all historical source/encoder pairs and immutable fixtures, all supported toolchains,
examples, fuzz builds and bounded campaigns, with disclosed sanitizer/leak limits.
Performance uses preserved matched before/after advance workloads and separate
new player initialization/damage/death inputs; allocation counts and RSS are distinct.

## Additional source finding before runtime implementation

P_LookForPlayers skips inactive slots before its stopping condition; all-inactive
source scanning does not terminate. Existing Rust EmptyRoster is a safety policy,
not a C-observed result. This player-body policy therefore does not admit live
Demon AI stepping or manual AI callbacks: direct controlled damage may reference a
live Demon between tics, but it must be explicitly killed or marked for removal
before the shared thinker pass. Dying/corpse Demon actions are independently
validated and remain executable. New source scenarios retire live sources before
traversal, or use dead-but-allocated sources. No observer runs the all-inactive
loop, no source body is patched to invent a terminating result, and no player
roster is fabricated to conceal this dependency. Future acquisition requires
player roster/think integration as its own gameplay milestone.
