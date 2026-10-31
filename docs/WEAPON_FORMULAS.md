# Shared original weapon formulas

`weapons.rs` owns one bullet damage formula, one unpowered-punch damage formula
and one ordinary angular-spread operation. Both historical preview/commit hosts
and genuine-player execution invoke the same `gun_shot` / `punch_shot` helpers.
The historical preview supplies its staged RNG; authoritative hosts retain their
existing RNG/event adapters. Firing state, ammo, ownership, traces, effects,
retaliation, pain/death and seven-pellet sequencing remain in their existing
services. No execution capability or diagnostic interpretation changes.

## Source and arithmetic

The preserved `p_pspr.c` `P_GunShot` uses `5 * (P_Random()%3+1)`.
Unpowered `A_Punch` uses `(P_Random()%10+1)<<1`, equivalently multiplication by two
for its positive bounded values. The shared spread scale is named once: 2^18.
Damage draws precede left then right spread draws; accurate bullets omit spread.
Pistol/chaingun callers retain their original accuracy decision. Shotgun calls
resolve seven inaccurate bullets individually; punches always draw spread.

The random-value difference is in [-255,255], and multiplication fits i32.
Unsigned BAM addition wraps explicitly. Original C's negative signed left shift
and unspecified subtraction-operand evaluation order are not portable ISO-C
behavior. The component observer uses the already declared ordered left/right
multiplication adapter, matching the engine's qualified bit-pattern contract.
This maintenance adds no Berserk or unsupported weapon behavior.

## Independent component evidence

[weapon_formulas_reference.py](../tools/reference/weapon_formulas_reference.py)
extracts two complete original bodies. Its no-hit, unpowered harness stubs aim,
line attack and unused sound/turn callbacks. Controlled input bytes independently
produce the [damage rows and spread summaries](../tests/reference/weapon_formula_observations.csv),
with [hashes and limits](../tests/reference/weapon_formula_contract.json).
These expectations were committed before the helper consolidation.
The Rust component tests compare all damage bytes and every spread pair at three
starting angles, then check staged/recorded adapter draw order and RNG indices.
Compact summaries avoid another large trace. Both functions were already present
in historical source families; this is new component input evidence, not globally
new source functions, trace coverage or a full weapon-compatibility claim.

Historical C/encoder and genuine-weapon field/event/full-byte comparisons remain
separate gameplay evidence. Existing weapon, pickup, rollback, provenance and
advance/step tests retain their old expected data. Commands belong to
[TESTING](TESTING.md#shared-weapon-formula-validation); observations and measured
limits belong to the [engineering record](validation/ZSTD_WEAPON_MAINTENANCE.md).

## Focused panic audit

Touched formula inputs are bounded bytes and checked/wrapping integer operations.
The historical hosts' remaining actor/definition/commit assumptions are guarded
by exact registry admission, player initialization, current-generation checks,
immutable metadata and successful preview before commit. No demonstrated
recoverable public-input panic was found in these touched paths. Genuine-player
execution already returns checked errors through staged lifecycle services.
This does not certify every existing production panic path; broader panic,
preflight/allocation and adapter maintenance remains separate roadmap work.
