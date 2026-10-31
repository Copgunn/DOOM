# Zstd evidence and shared weapon formula maintenance

Observed on 2026-10-31 (America/Los_Angeles). Exact review commits, merge ancestry and CI belong to GitHub; this record owns the detailed engineering observations. No new gameplay capability or DSIM version is admitted.

## Scope and independent checkpoints

The branch first committed the untouched baseline inventory and design, then the bounded codec, then the verified migration. A separate compiled-C component expectation commit preceded the weapon-helper refactor. Final review added an early direct-entry budget guard before hashing; affected codec tests, sanitizer/fuzz checks and measurements were refreshed. The initial after measurements and interleaved follow-up remain retained, rather than replaced.

The [storage contract](../EVIDENCE_STORAGE.md) owns framing/resource limits and tooling architecture; [weapon formulas](../WEAPON_FORMULAS.md) owns shared caller and source-arithmetic semantics. [TESTING](../TESTING.md#historical-zstd-evidence) owns commands.

## Package A: exact lossless migration

19 raw traces: **88,469,558 → 4,820,823 bytes**, saving **83,648,735 bytes**, **18.351547:1**, **94.550868%** payload reduction. Manifest overhead is 10,143 bytes. This is current-checkout payload storage, not Git-history shrinkage.

Original lengths, SHA-256 and Git blob IDs were captured [before transformation](zstd-baseline.json). Every file was read completely, checked against its actual baseline Git bytes, encoded, decoded, length/hash checked, and actually compared byte for byte before any raw counterpart was removed. The [migration record](zstd-migration-proof.json) is not trusted alone: the separate [Git proof](maintenance-zstd/preservation.json) reruns comparisons against the immutable Git object. The [manifest](../../tests/reference/zstd_manifest.json) contains both byte identities, exact encoder/settings, budgets and source family.

| Original CSV basename | Original bytes | Standard zstd bytes |
| --- | ---: | ---: |
| `m3d2_trace.csv` | 4,944,681 | 241,191 |
| `m3d2b_trace.csv` | 1,477,220 | 77,926 |
| `m3d3b_trace.csv` | 1,077,576 | 73,405 |
| `m3d3c_trace.csv` | 1,851,813 | 90,439 |
| `m3d3d_trace.csv` | 1,791,165 | 116,870 |
| `m3d3e_trace.csv` | 3,970,619 | 257,705 |
| `m3d3f_trace.csv` | 1,285,282 | 94,412 |
| `m3d3g_trace.csv` | 4,188,957 | 236,319 |
| `m3d3h_trace.csv` | 7,056,084 | 397,500 |
| `m3d3i_spawn.csv` | 1,330,657 | 101,817 |
| `m3d3i_trace.csv` | 4,961,898 | 269,120 |
| `m3d3j_spawn.csv` | 1,363,363 | 109,036 |
| `m3d3j_trace.csv` | 7,007,769 | 322,567 |
| `m3f1_movement.csv` | 1,399,006 | 15,885 |
| `m3f2_sight.csv` | 5,005,831 | 27,485 |
| `m3f3_clipping_trace.csv` | 1,933,895 | 112,921 |
| `m3f3_trace.csv` | 28,923,662 | 1,631,081 |
| `m3f5_trace.csv` | 7,224,524 | 440,991 |
| `m3g7_trace.csv` | 1,675,556 | 204,153 |

The encoder is zstd0.13.3 / zstd-safe7.3.0 / bundled libzstd1.5.7: level9, window_log23, checksum/content-size enabled, workers0, no dictionary. All19 committed frames recompress identically in the codec tests. Nine codec tests cover valid frames/cache, every truncation, corruption, framing/trailing/extra frames, hashes, sizes/windows, overflow/total budgets, schema/path/duplicate/unknown fields and missing files. Direct Entry construction now checks compressed limits before SHA work; this is Rust/tool safety policy.

Only19 raw paths are removed. Existing DREFLZ1 helpers/streams, full DSIM fixtures, original source, encoders, provenance, maps/scenario inputs and archives remain intact. The frozen observers run through one copied temporary workspace with decoded raw paths; no hardlinks or Rust-generated expectations. Source commands/flags retain their exact baseline inventory.

The delivery proof checks 6,342 baseline protected paths, including 6,327 unchanged paths and15 support-file loading substitutions, plus4 separately checked integration-test substitutions:19 consumers/21 include sites in total. All165 upstream files were also actually compared against the original upstream object;20 archive records remain exact. The active reference-tool README is an explicit human-guide exception. No global preservation exemption was introduced.

## Package B: shared source operations

Historical Preview::fire and Commit::fire now call gun_shot / punch_shot, as does genuine-player fire_current. One bullet formula, one unpowered-punch formula and one named spread operation remain. Damage draws precede ordered left/right draws; accurate pistol/chaingun bullets omit spread, shotgun resolves seven inaccurate pellets, and fist keeps its aim/impact path. No player fields, weapon ownership/ammo, psprite timing, trace/effects, damage/retaliation, RNG/event order, profile contract or actor bound changed.

Three component tests check all256 damage bytes; all65,536 spread pairs at three starting angles for each operation; and staged versus recorded DoomRng adapters, indices, classifications and pellet ordering. One additional private stage test validates preview nonmutation and supplies opt-in cost instrumentation; it shares the existing fixture module without duplicate loading.

The new compact C oracle executes inherited P_GunShot and A_Punch bodies:256 damage inputs plus196,608 spread cases per body,393,728 total body entries. Two inherited compiled bodies, zero globally new source bodies, zero new gameplay scenarios/full DSIM fixtures. Minimal named-field records, unpowered/no-hit conditions and aim/line/sound stubs are explicit component adapters. Negative signed spread shifts use the already declared ordered-draw/multiplication BAM interpretation; portable ISO-C equivalence for undefined expressions is not claimed. All existing source-backed gameplay evidence remains separate and reused.

No demonstrated recoverable public-input panic was found in the touched weapon arithmetic/host paths; existing immutable-registry/preflight invariants remain documented. This does not certify all historical production panic paths. No unsafe Rust or engine framework was added.

## Validation observations

| Validation | Observed result |
| --- | --- |
| Stable workspace | 598 passed,0 failed,2 intentionally ignored asset tests |
| Rust1.85 workspace | 598 passed,0 failed,2 intentionally ignored asset tests |
| Release simulation/type | 565 passed,0 failed |
| Codec stable/MSRV/release | Nine tests pass in each applicable focused run |
| Build/format/fuzz-format/Clippy/rustdoc/examples/parser/seed generation | All43 established local commands passed; affected guard/doc checks rerun |
| Simulation examples | All35 pass debug/release comparison |
| Frozen source/encoder suite | All65 commands pass, including all21 historical pairs and every later family |
| Original metadata | 137 actor types,967 states,138 sprites; sanitized check passes |
| New C formula observer | Normal and ASan/UBSan observations agree with committed component expectations |
| Historical diagnostics | Existing versions including v20–v25, readers, full bytes/events and advance/step contracts preserved |

[Local results](maintenance-zstd/local-results.json), [source command results](maintenance-zstd/source-results.json) and the unchanged independent encoders distinguish gameplay equivalence from Rust/tool safety. Existing M3g9 source observations and every earlier family are reused regression evidence, not new source coverage. One original signed spawn-angle overflow is intentionally observed by the unchanged historical probe; it is not admitted gameplay or a newly hidden sanitizer error.

### Fuzzing and sanitizers

36 distinct fuzz targets build. Twelve distinct targets execute13 completed final campaigns:53,248 executions (4,096 each), with no findings/artifacts. A preliminary completed codec campaign adds4,096 executions separately; an interrupted incomplete scheduler attempt is excluded. [Exact per-target seeds/budgets/durations](maintenance-zstd/fuzz-results.json) and [built executable identities](maintenance-zstd/fuzz-built.json) keep build and execution claims separate. The additional codec campaign exercises the final direct-entry guard.

Ordinary campaigns use Rust ASan. The dedicated codec campaigns additionally instrument vendor zstd C with GCC ASan/UBSan; nine codec unit/round-trip tests pass with that C instrumentation. [Native archive proof](maintenance-zstd/native-codec-sanitizer-proof.json) verifies actual sanitizer references, not coverage percentages. All campaigns use ASAN_OPTIONS=detect_leaks=0. LeakSanitizer failed under the environment tracing restriction when unintentionally enabled; no leak coverage is claimed. An initial Clang sanitizer attempt could not build because Clang was absent; the supported GCC run passed. Bounded campaigns are not exhaustive.

## Performance and memory

Matched release compiler: Rust1.99.1 on Linux x86_64, shared host with a four-CPU quota and16GiB memory limit. No local source/test/fuzz work overlaps quiet measurement batches. Each established driver retains three independent timing/RSS processes and separate allocation processes. No CPU isolation, page-cache reset or universal performance claim; all samples/outliers are retained.

The final281 matched rows have identical deterministic observations; all241 recorded allocation arrays are exactly unchanged. Private preview/commit stage digests and allocation arrays also match using the identical harness. The only altered historical driver is measure_legacy_weapons.py linker/build glue for the test codec; its timed PROGRAM and workload remain unchanged. Other16 driver hashes match.

| Driver | Before samples/method | Final after samples/method |
| --- | --- | --- |
| stepping | [raw JSON](maintenance-zstd/before-measure/stepping.json) | [raw JSON](maintenance-zstd/after-measure/stepping.json) |
| demon_expanded_map | [raw JSON](maintenance-zstd/before-measure/demon_expanded_map.json) | [raw JSON](maintenance-zstd/after-measure/demon_expanded_map.json) |
| canonical_demon | [raw JSON](maintenance-zstd/before-measure/canonical_demon.json) | [raw JSON](maintenance-zstd/after-measure/canonical_demon.json) |
| original_registry | [raw JSON](maintenance-zstd/before-measure/original_registry.json) | [raw JSON](maintenance-zstd/after-measure/original_registry.json) |
| context_ownership | [raw JSON](maintenance-zstd/before-measure/context_ownership.json) | [raw JSON](maintenance-zstd/after-measure/context_ownership.json) |
| game_instance | [raw JSON](maintenance-zstd/before-measure/game_instance.json) | [raw JSON](maintenance-zstd/after-measure/game_instance.json) |
| player_body | [raw JSON](maintenance-zstd/before-measure/player_body.json) | [raw JSON](maintenance-zstd/after-measure/player_body.json) |
| player_knockback | [raw JSON](maintenance-zstd/before-measure/player_knockback.json) | [raw JSON](maintenance-zstd/after-measure/player_knockback.json) |
| player_commands | [raw JSON](maintenance-zstd/before-measure/player_commands.json) | [raw JSON](maintenance-zstd/after-measure/player_commands.json) |
| player_demon | [raw JSON](maintenance-zstd/before-measure/player_demon.json) | [raw JSON](maintenance-zstd/after-measure/player_demon.json) |
| player_demon_melee | [raw JSON](maintenance-zstd/before-measure/player_demon_melee.json) | [raw JSON](maintenance-zstd/after-measure/player_demon_melee.json) |
| legacy_weapons | [raw JSON](maintenance-zstd/before-measure/legacy_weapons.json) | [raw JSON](maintenance-zstd/after-measure/legacy_weapons.json) |
| player_pistol | [raw JSON](maintenance-zstd/before-measure/player_pistol.json) | [raw JSON](maintenance-zstd/after-measure/player_pistol.json) |
| player_weapons | [raw JSON](maintenance-zstd/before-measure/player_weapons.json) | [raw JSON](maintenance-zstd/after-measure/player_weapons.json) |
| map_scale | [raw JSON](maintenance-zstd/before-measure/map_scale.json) | [raw JSON](maintenance-zstd/after-measure/map_scale.json) |
| map_extent | [raw JSON](maintenance-zstd/before-measure/map_extent.json) | [raw JSON](maintenance-zstd/after-measure/map_extent.json) |
| player_world_pickups | [raw JSON](maintenance-zstd/before-measure/player_world_pickups.json) | [raw JSON](maintenance-zstd/after-measure/player_world_pickups.json) |

Representative final medians follow; these rows time100 production advances. Whole-process peak RSS includes setup and fixture data, not just live engine allocations. Every individual timing/RSS/allocation sample remains in the linked data.

| Workload | Before ms/100 | Final after ms/100 | Before/after median RSS KiB |
| --- | ---: | ---: | ---: |
| stepping:small_control/20/advance | 10.530520 | 10.680173 | 2916 / 2832 |
| stepping:long_lifecycle/20/advance | 11.240882 | 11.183027 | 3672 / 3584 |
| stepping:four_owners/20/advance | 48.933683 | 47.134052 | 3728 / 3728 |
| stepping:wide_map_lifecycle/20/advance | 11.403463 | 11.953978 | 3928 / 3728 |
| stepping:expanded_sight_dag/20/advance | 10.877945 | 10.989591 | 2856 / 3040 |
| stepping:line_table/20/advance | 10.976121 | 11.021727 | 3672 / 3500 |
| stepping:high_cell_demon/20/advance | 11.676927 | 11.788133 | 4352 / 4356 |
| stepping:canonical_small/original/advance | 10.718830 | 10.667023 | 2572 / 2520 |
| legacy_weapons:historical_pistol_held/advance | 0.143513 | 0.163281 | 2636 / 2644 |
| player_pistol:held_refire/advance | 5.992937 | 6.045469 | 18664 / 19304 |
| player_weapons:fist_hit/advance | 5.722089 | 5.490365 | 47692 / 47704 |
| player_weapons:shotgun_held/advance | 7.337099 | 7.329498 | 47676 / 47692 |
| player_weapons:chaingun/advance | 7.450671 | 7.449076 | 47692 / 47772 |
| player_weapons:full_encounter/advance | 25.991441 | 24.738291 | 47748 / 47720 |
| player_world_pickups:full_encounter/advance | 35.298686 | 30.524425 | 35776 / 35752 |

Timing is mixed. The initial after batch showed +7.1% small-control advance and +6.9% sight-step medians; [interleaved follow-up](maintenance-zstd/interleaved.json) retained approximately +4.8%/+5.6%. These modest control increases remain observed and unprofiled; no causal attribution or universal equivalence is established. The large initial pickup improvement shrank from17.9% to1.1% in the paired follow-up. The tiny historical-pistol control changed direction between batches. These inconsistencies preclude an optimization claim. The [initial281-row comparison](maintenance-zstd/comparison-initial.json), [initial raw measurements](maintenance-zstd/after-initial-measure/results.json) and [final comparison](maintenance-zstd/comparison.json) retain all observations; no slow sample was discarded.

### Separate preview and commit costs

Three processes per phase;100 independent fixture worlds prepared outside each interval. Preview alone versus already-preflighted weapon commit is not a whole gameplay tic. Before baseline instrumentation adds only the same cfg(test) harness/fixture visibility.

| Source fixture / stage | Before samples ms/100 | After samples ms/100 | Before/after median ms |
| --- | --- | --- | --- |
| 0:preview | 0.695300 / 0.615711 / 0.606748 | 0.677273 / 0.671133 / 0.619638 | 0.615711 / 0.671133 |
| 0:commit | 0.975844 / 0.912871 / 0.902986 | 1.030185 / 0.970196 / 1.021011 | 0.912871 / 1.021011 |
| 5:preview | 1.628000 / 1.564445 / 1.690552 | 1.595892 / 1.624344 / 1.611426 | 1.628000 / 1.611426 |
| 5:commit | 2.895124 / 2.973570 / 2.979520 | 3.066048 / 2.883478 / 2.803810 | 2.973570 / 2.883478 |
| 23:preview | 0.529654 / 0.546920 / 0.560139 | 0.562131 / 0.547751 / 0.592176 | 0.546920 / 0.562131 |
| 23:commit | 0.711923 / 0.816238 / 0.780905 | 0.727687 / 0.758092 / 0.704453 | 0.780905 / 0.727687 |
| 24:preview | 0.766164 / 0.745102 / 0.661258 | 0.665936 / 0.663372 / 0.648500 | 0.745102 / 0.663372 |
| 24:commit | 0.975534 / 0.977797 / 0.939631 | 0.889587 / 0.906872 / 0.910297 | 0.975534 / 0.906872 |

[Before](maintenance-zstd/before-measure/weapon-stages.json) and [after](maintenance-zstd/after-measure/weapon-stages.json) include all stage allocation arrays, per-process peak RSS, calibration and the identical harness hash.

### Reference loading and end-to-end checks

The [filesystem microbenchmark](maintenance-zstd/evidence-loading.json) uses one common release executable for raw reads versus uncached zstd/frame/integrity work. It processes all19 traces. It is not the original include_str test architecture; OS page cache was not reset.

| Path | Three elapsed samples ms | Median ms | Peak RSS samples KiB | Allocation calls/reallocs/requested bytes per batch |
| --- | --- | ---: | --- | --- |
| raw | 129.433332 / 125.886893 / 125.075863 | 125.886893 | 36404 / 36340 / 36400 | 76 / 19 / 88,475,657 |
| zstd | 218.899016 / 217.577299 / 214.547915 | 217.577299 | 36668 / 36664 / 36728 | 228 / 38 / 95,123,359 |

Actual M3f3 reference test binaries separately include startup, evidence decode/parsing and every comparison assertion; build time is excluded.

| Test build | Before elapsed samples s | After elapsed samples s | Before/after median s |
| --- | --- | --- | --- |
| debug | 17.434876 / 17.730036 / 17.615554 | 19.139649 / 19.247929 / 18.731084 | 17.615554 / 19.139649 |
| release | 2.816895 / 2.806471 / 2.814011 | 2.909258 / 2.777855 / 2.857346 | 2.814011 / 2.857346 |

[Before](maintenance-zstd/before-measure/reference-checks.json) and [after](maintenance-zstd/after-measure/reference-checks.json) preserve all RSS samples. For the final paired record, maximum debug RSS changes273,924→276,064KiB; release272,388→274,196KiB. This test-only loading cost is separate from production updates. Allocation requests are cumulative, not peak live heap or leak measurements.

## Storage/dependency accounting and limitations

The manifest adds10,143 bytes. [Actual dependency storage](maintenance-zstd/dependency-storage.json):31 external locked packages,3,247,555 registry archive bytes and17,453,490 unpacked registry file bytes; zero vendored dependency bytes. Cache/build outputs are not repository checkout payload and are not universal disk budgets. [Declared versions/licenses/MSRVs](zstd-dependencies.json) match the actual lock resolution; Rust1.85 compilation and cross-platform CI are separate execution evidence. The evaluated ruzstd0.8.2 failed its Rust1.85 probe; this does not imply every older pure-Rust decoder is incompatible.

New tooling/metadata and all raw measurement samples are retained compactly; current tracked-byte overhead is reported in [checkout accounting](maintenance-zstd/storage-accounting.json). Git still contains the old raw traces. Temporary source-reproduction copies require extra disk space. Frozen direct generator interfaces expect raw paths, so current users invoke their wrapper; archived commands describe old checkouts and remain unchanged.

No armor, powers, new weapons/monsters, player-thinker branch, rendering or ownership redesign is implemented. Existing production panic paths, repeated preflight/staging allocations and future evidence growth remain bounded follow-up maintenance priorities.

## Recommended next gameplay milestone

M3g10: independently observe and admit bounded original green/blue armor pickup and damage absorption through the existing world-item, OriginalPlayer and DamagePlan services. Keep other powers, new weapons, respawn and generalized specials separate.
