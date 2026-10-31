# Original four-weapon execution investigation

Original Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`
is the authority. The new observer extends the immutable
[M3g7 observer](../tools/reference/m3g7_reference.py); it does not modify upstream
source or historical expectations. Implementation and measured results belong in
[the engineering record](validation/M3G8.md).

| Source behavior | Existing service / integration disposition | Evidence and boundary |
| --- | --- | --- |
| `p_pspr.c:P_CheckAmmo` chooses an owned usable weapon, assigns pending, enters downstate | Reuse shared `weapons::Machine`; admit fallback only under the new policy | Original priority restricted to the four admitted weapons is chaingun, shotgun, pistol, fist. Never replace ready immediately. |
| `A_Lower`, `A_Raise`, `P_BringUpWeapon`, `P_SetPsprite` | Same interpreter, original states, immediate nested actions | Six-unit increments; bottom128, top32. Dead lowering clamps at bottom and retains ready/pending. |
| `p_user.c:P_PlayerThink` weapon-selection fragment | Run existing selection after command/reaction stage, before psprites | Supported unowned requests are ignored; same-ready requests leave pending alone. This fragment is not full PlayerThink. |
| `A_Punch` | Shared RNG, aim/line trace, effect allocation and DamagePlan | Damage2..20, two spread draws, range64. On actual actor hit, sound and facing use the target's position **after** damage and any immediate retarget/chase action. No Berserk. |
| `A_FireShotgun` | Seven sequential uses of shared gunshot/trace/damage | One shell, one aim-slope calculation, seven independent inaccurate pellets. Each effect, damage, pain/death and RNG action completes before the next pellet. |
| `A_FireCGun` | Same bullet algorithm as pistol, distinct original states | Sound precedes empty-ammo return. First two shots have refire0; later shots spread. State-dependent flashes include Light2. |
| `P_FireWeapon` | Same body/psprite/noise services | Attack-state entry can immediately fire (chaingun) **before** NoiseAlert. Delayed pistol/shotgun/fist actions occur later. |
| `A_ReFire` | Source counter, checked Rust integer | Held cadence: fist17, pistol14, shotgun37, chaingun4 tics. Initial ready-triggered discharge is fist/pistol19, shotgun18, chaingun15 in this fresh setup. |
| `P_BulletSlope`, `P_GunShot`, map traces | Existing fixed-point tracing | Aim1024 at current/+offset/-offset; actual bullets2048 at current/spread angle. No nearest-target substitution. |
| `P_DamageMobj`, `P_KillMobj`, `P_DropWeapon` | Shared combat/lifecycle/thinker transaction | Genuine player0 and Demon12, Demon health150/mass400. Retaliation, pain, kill credit, corpse and actual weapon downstate remain ordered. |
| `G_PlayerReborn`, source inventory | Existing OriginalPlayer owns body/health; additive weapon-only state | Fresh pistol50; shotgun/chaingun ownership and alternative ammo are explicit controlled initial loadouts, not pickups. |

The harness runs the declared command/reaction and weapon-selection fragments,
actual `P_CalcHeight`/`P_MovePsprites`, then original thinkers. It is not complete
`P_Ticker`, `P_PlayerThink`, view logic or death camera. All four weapons share one
player, actor store, RNG, damage service and scheduler.

Spread/jitter negative-shift and operand-order qualifications retain the previous
observer's explicit left-before-right, representable multiplication adapter.
Supplied command turns are nonnegative; map angles0/-180 keep source signed
multiplication defined. Results describe this qualified GNU execution domain,
not portable equivalence for undefined C expressions. Grounded z0 excludes the
conditional reverse-thrust draw. No freed pointers or all-inactive roster scans
are executed. Generational lifetime, rejected-operation atomicity and bounded
parsing are separately specified Rust policies.

The winning fist scenario uses original seed8, selected from a bounded source
experiment over seeds0..63. No damage or random values are injected. Other cases
retain losing encounters, misses, wall impacts, empty-ammo transitions, switching,
sequential pellets and death during transitions. Source counts, per-function
entries, source hashes and packed/unpacked hashes are recorded in
[observer provenance](../tests/reference/m3g8_provenance.json).

New test evidence uses the bounded, deterministic DREFLZ1 container in
[evidence_pack.py](../tools/reference/evidence_pack.py). This is test storage,
not gameplay or a save format. Regeneration compares expanded independent C
observations and deterministic packed bytes. Historical evidence is untouched.
