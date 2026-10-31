# Original world pickup investigation

This investigation defines the opt-in original world pickup capability.
Source observations, Rust execution and safety policy remain distinct claims. Original Linux DOOM 1.10 at the revision named in
AGENTS is the authority; the source checkpoint precedes matching Rust gameplay.

| Source behavior | Existing service / implementation disposition | Evidence boundary |
| --- | --- | --- |
| `p_mobj.c:P_SpawnMapThing/P_SpawnMobj`, `info.c` static item definitions | Same actor store, THING filtering/order, canonical definitions and lastlook draw; at most16 items within64 slots | Actual source map and controlled raw dropped spawns |
| `p_map.c:P_CheckPosition/PIT_CheckThing` things before lines, strict square overlap | Shared movement contact hook before line checks; ordinary blocked movement retains collection | Source wall/solid ordering and exact overlap components |
| `p_inter.c:P_TouchSpecialThing` vertical inclusive height/−8, live health, sprite dispatch | Checked current original player and item; no airborne admission | Vertical components remain source observations only |
| `P_GiveAmmo` clip10/half5/box50, shell4/box20; capacities200/50 | Existing player weapon record; current ready weapon determines zero-ammo selection | Independent original grants, lowering/raising cases |
| `P_GiveBody` +10/+25 capped100, synchronize player/mo | Existing health owner and moving actor; no new health store | Live1/95/99/100 and above100 component |
| Medikit message tests health after healing | Typed persistent message identifiers | GOTMEDINEED unreachable for positive admitted health |
| `P_GiveWeapon` ammo before ownership, placed two loads/dropped one | Reuse four-weapon inventory/psprites; new ownership requests pending weapon | Placed/dropped/owned/full and live encounters |
| Accepted message, optional COUNTITEM, removal, bonus+6, sound | Same deferred retirement/spatial unlink and event ordering; all eight definitions lack COUNTITEM | Actual source events, zero item totals |
| `P_RemoveMobj` inactive itemrespawn queue | Retain C-only PQ observations; queue storage and respawn excluded from Rust policy | No full RemoveMobj whole-state equivalence |
| Live `P_PlayerThink` bonuscount decrement after weapon stage | Only that declared fragment, before shared thinkers | Not full PlayerThink/Ticker/DeathThink, HUD or camera |
| Handle reuse, scope and rollback | Existing generational store, instance provenance and transaction | Rust safety policy, not C pointer equivalence |

The eight canonical definitions are MT_MISC10/STIM (53/840), MT_MISC11/MEDI
(54/841), MT_CLIP (63/870), MT_MISC17/AMMO (64/871), MT_MISC22/SHEL (69/876),
MT_MISC23/SBOX (70/877), MT_CHAINGUN/MGUN (73/880), MT_SHOTGUN/SHOT (77/884).
The second value is the original spawn-state ordinal. All have static tics−1,
MF_SPECIAL, radius20 and height16. Weapon items are distinct from monster types.

The separate policy9 uses one genuine active player0, at most one map Demon,
Medium/commercial settings, existing grounded static axis-wall geometry and
four-weapon execution. Controlled dropped Clip/Shotgun/Chaingun setup precedes
body binding and tic0; no monster loot or runtime respawn is claimed.

DSIM25 is necessary because persistent bonus/item/message fields and pickup event
observations are absent from DSIM24. It retains the existing weapon fields, adds
pickup presence/bonus_count/item_count/message before events and uses event23.
The independent encoder specifies exact bytes before matching Rust changes.
Historical DSIM20–24 and all source tools/expectations remain immutable.

The source observer records actual trace callback roles. Geometry-only slide
traversal rows remain raw source evidence; gameplay movement/slide decisions are
compared without inventing aim events. Source expressions retain the inherited
declared arithmetic/evaluation adapters. No invalid ammo index, empty-roster
source loop, freed C pointer, proprietary asset or Rust-generated expectation is
used. See the new provenance manifests for hashes and exact observations.

The sustained pistol cases start with ordinary50 bullets and use fixed command
scripts, genuine Demon melee and actual world ammo/healing. One wins, another
retains a losing player-death outcome. Controlled damaged-health cases remain
separate. World shotgun/chaingun acquisition is feasible in the source and executes through the same player inventory and psprites after the ammo and
health checkpoints; it is not deferred merely because it changes pending weapon
state.

During initial comparison, the new encoder's pickup-event flush exposed a
candidate-prefix bug: source C rows emitted before a touch must remain attached
to the later M movement result. The correction buffers those read-only C/O rows
across pickup events. Original observations and all historical families remain
unchanged; the new DSIM expectations are corrected from their actual source
candidate rows. This is an encoder correction, not changed source gameplay.
