# M3h9b evidence record

Observed results of the M3h9b milestone (`PlayerMonstersBarrelsV1`, wire 21, DSIM37), the
second part of the E1M1 split recommended in the
[M3h9 investigation](../M3H9_REFERENCE_INVESTIGATION.md) §8. Only observations appear
here. The [M3h9b investigation](../M3H9B_REFERENCE_INVESTIGATION.md) owns the source
facts, the admitted domain, the budgets (§5.3), the decisions (§6) and the plan (§9);
[format](../STATE_FORMAT.md) owns DSIM37 and [support](../COMPATIBILITY.md) the admitted
behavior. Exact review state belongs to GitHub. No full-player, whole-ticker, DOS-demo or
whole-game equivalence is claimed; the real-E1M1 run is M3h9c.

## IWAD counts

Read locally and read-only on 2026-11-04 from the owner's two IWADs (68 map slots) with
the counts-only modes added to `tools/iwad_counts.py` for the investigation's §3
(`--spawn-rule [SLOT]`, `--spawn-rule-markdown`, `--things-budget [SLOT]`,
`--spawn-fit [SLOT]`; local only, never run by CI, a no-op without the IWAD directory).
The `--things-markdown` tables were re-run first and are byte-identical to
[M3H9_IWAD_COUNTS.md](M3H9_IWAD_COUNTS.md) (182 table lines). Every mode prints counts,
pass/fail facts, type names and DoomEd numbers only; two consecutive runs of each mode
were byte-identical. No IWAD content enters the repository, CI or logs.

### E1M1

The inherited closed-box spawn rule (M3h5 §4.1 item 4, every spawned kind, the player
body included) and the proposed rule S′ (investigation decision F3):

| Skill | Spawned | Closed-box violations | Edge contact only | Open-box overlap [movers, statics] | Overlap depth 0 / 1–4 / 5–8 / 9–12 / 13+ | S′ violations | Minimum embedded-mover margin |
| --- | --- | --- | --- | --- | --- | --- | --- |
| easy | 90 | 16 | 6 | 10 [1, 9] | 2 / 8 / 0 / 0 / 0 | **0** | 16 |
| medium | 92 | 16 | 6 | 10 [1, 9] | 2 / 8 / 0 / 0 / 0 | **0** | 16 |
| hard | 116 | 17 | 6 | 11 [2, 9] | 2 / 9 / 0 / 0 / 0 | **0** | 16 |

Every closed-box violation is a one-sided wall contact (even parity 0, outside the
rectangle 0, centre outside the BLOCKMAP 0; things touching any line, one- or two-sided:
21/21/25). By kind on medium (violations/spawned): monster 1/6, pickup 9/52, solid
decoration 6/12, non-solid decoration 0/15, barrel 0/6, player 0/1; by DoomEd number:
35 (`MT_MISC50`) 2, 2014 (`MT_MISC2`) 4, 2015 (`MT_MISC3`) 5, 2028 (`MT_MISC31`) 4, 3001
(`MT_TROOP`) 1; hard adds 9 (`MT_SHOTGUY`) 1. The six edge contacts are all solid
decorations; the margin column is `⌊cross / (|dx| + |dy|)⌋` over the embedded movers,
against the S′ requirement `31 − r` (11 for radius 20).

Spawn height fit: pass at every skill, 0 failures, minimum height of a sector containing a
spawned thing 72.

Budget facts (`--things-budget E1M1`): BLOCKMAP 36 × 23 cells; most lines in one cell 13;
distinct lines in a 2 × 2 window 28; most things in one cell 3/4/4 (e/m/h); lines plus
things over a whole row at most 150/150/151; over any 18 consecutive cells of a row,
column or diagonal at most **103/103/104**.

### All 68 slots

| Rule | Easy | Medium | Hard |
| --- | --- | --- | --- |
| Closed-box spawn rule: slots passing | 4 | 4 | 4 |
| Closed-box violations (things) | 1,388 | 1,447 | 1,487 |
| of which wall contacts: edge only / overlap [movers, statics] | 430 / 933 [28, 905] | 443 / 972 [62, 910] | 452 / 996 [93, 903] |
| S′: slots passing | 53 | 52 | 51 |
| S′ violations (things): parity or rectangle / mover depth | 48 / 5 | 56 / 6 | 64 / 7 |
| Spawn height fit: slots passing | 65 | 65 | 65 |
| Fit failures (monster 4, pickup 10, non-solid decoration 2) | 16 | 16 | 16 |

The four slots passing the closed-box rule at medium are E3M8, MAP07, MAP21 and MAP32; the
three failing the height fit are E2M1, E4M2 and MAP32. Spawned things over all slots:
11,559 / 13,923 / 15,878.

Per-slot closed-box table (`--spawn-rule-markdown`):

| Slot | Spawned e/m/h | Violations e/m/h | Wall touch e/m/h | Even parity e/m/h | Outside rectangle e/m/h | Touching any line e/m/h | Centre outside BLOCKMAP e/m/h | Pass e/m/h |
|---|---|---|---|---|---|---|---|---|
| E1M1 | 90/92/116 | 16/16/17 | 16/16/17 | 0/0/0 | 0/0/0 | 21/21/25 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M2 | 180/200/239 | 17/17/17 | 17/17/17 | 0/0/0 | 1/1/1 | 20/22/24 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M3 | 257/310/365 | 33/33/32 | 33/33/32 | 0/0/0 | 0/0/0 | 45/47/47 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M4 | 170/201/229 | 20/21/20 | 20/21/20 | 0/0/0 | 0/0/0 | 21/23/22 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M5 | 152/216/257 | 24/34/36 | 24/34/36 | 0/0/0 | 0/0/0 | 25/36/41 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M6 | 287/370/440 | 65/71/73 | 65/71/73 | 0/0/0 | 0/0/0 | 88/102/107 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M7 | 212/267/330 | 69/71/76 | 69/71/76 | 0/0/0 | 0/0/0 | 91/100/110 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M8 | 94/99/113 | 11/13/13 | 11/13/13 | 0/0/0 | 0/0/0 | 15/21/25 | 0/0/0 | FAIL/FAIL/FAIL |
| E1M9 | 153/192/223 | 21/21/21 | 21/21/21 | 0/0/0 | 0/0/0 | 42/43/45 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M1 | 78/100/115 | 2/2/2 | 2/2/2 | 0/0/0 | 1/1/1 | 8/9/10 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M2 | 230/261/286 | 20/19/19 | 20/19/19 | 0/0/0 | 0/0/0 | 25/24/26 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M3 | 155/197/208 | 5/5/5 | 5/5/5 | 0/0/0 | 0/0/0 | 13/18/19 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M4 | 200/226/232 | 9/9/9 | 9/9/9 | 0/0/0 | 0/0/0 | 15/17/17 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M5 | 250/278/313 | 15/14/14 | 15/14/14 | 0/0/0 | 0/0/0 | 29/28/30 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M6 | 224/266/305 | 16/15/15 | 16/15/15 | 0/0/0 | 0/0/0 | 22/21/23 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M7 | 188/202/219 | 10/10/10 | 10/10/10 | 0/0/0 | 1/1/1 | 25/25/25 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M8 | 36/40/56 | 2/2/2 | 2/2/2 | 0/0/0 | 0/0/0 | 4/4/4 | 0/0/0 | FAIL/FAIL/FAIL |
| E2M9 | 41/41/41 | 3/3/3 | 3/3/3 | 0/0/0 | 1/1/1 | 3/3/3 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M1 | 65/71/81 | 3/3/3 | 3/3/3 | 0/0/0 | 0/0/0 | 3/3/3 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M2 | 162/177/187 | 24/24/23 | 24/24/23 | 0/0/0 | 0/0/0 | 30/31/31 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M3 | 140/165/183 | 30/29/29 | 30/29/29 | 0/0/0 | 0/0/0 | 43/45/45 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M4 | 231/298/358 | 32/33/34 | 32/33/34 | 0/0/0 | 0/0/0 | 37/39/43 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M5 | 214/245/265 | 18/18/18 | 18/18/18 | 0/0/0 | 2/2/2 | 22/22/22 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M6 | 242/281/312 | 10/10/11 | 10/10/11 | 0/0/0 | 0/0/0 | 26/26/29 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M7 | 115/126/127 | 7/7/7 | 7/7/7 | 0/0/0 | 0/0/0 | 9/10/10 | 0/0/0 | FAIL/FAIL/FAIL |
| E3M8 | 31/32/32 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | pass/pass/pass |
| E3M9 | 170/176/184 | 5/5/4 | 5/5/4 | 0/0/0 | 0/0/0 | 5/5/4 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M1 | 98/119/124 | 16/17/19 | 16/17/19 | 1/2/3 | 1/2/3 | 32/38/38 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M2 | 159/193/214 | 40/39/38 | 40/39/38 | 0/0/0 | 0/0/0 | 60/60/64 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M3 | 228/259/294 | 36/38/37 | 34/36/35 | 9/9/9 | 0/0/0 | 39/41/41 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M4 | 87/102/116 | 16/17/18 | 16/17/18 | 0/0/0 | 0/0/0 | 28/30/32 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M5 | 210/239/252 | 34/35/35 | 34/35/35 | 0/0/0 | 0/0/0 | 71/72/74 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M6 | 233/267/274 | 66/68/66 | 66/68/66 | 0/0/0 | 0/0/0 | 91/95/93 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M7 | 203/209/240 | 25/26/29 | 25/26/29 | 0/0/0 | 1/1/1 | 47/50/56 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M8 | 210/246/279 | 20/21/21 | 20/21/21 | 0/0/0 | 0/0/0 | 45/48/54 | 0/0/0 | FAIL/FAIL/FAIL |
| E4M9 | 188/207/229 | 14/14/14 | 14/14/14 | 0/0/0 | 0/0/0 | 35/34/34 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP01 | 34/44/52 | 2/2/2 | 2/2/2 | 0/0/0 | 0/0/0 | 4/4/4 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP02 | 108/131/151 | 11/11/11 | 11/11/11 | 0/0/0 | 1/1/1 | 37/40/48 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP03 | 75/102/124 | 4/4/6 | 4/4/6 | 0/0/0 | 0/0/0 | 15/15/17 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP04 | 91/103/120 | 11/12/12 | 11/12/12 | 0/0/0 | 3/3/3 | 27/28/29 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP05 | 118/158/189 | 22/22/22 | 22/22/22 | 0/0/0 | 0/0/0 | 32/33/35 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP06 | 164/201/222 | 37/37/39 | 37/37/39 | 0/0/0 | 0/0/0 | 44/45/47 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP07 | 64/70/77 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | pass/pass/pass |
| MAP08 | 153/197/229 | 16/16/16 | 16/16/16 | 0/0/0 | 0/0/0 | 29/29/29 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP09 | 162/274/371 | 1/2/2 | 1/2/2 | 0/0/0 | 0/0/0 | 15/16/16 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP10 | 302/361/484 | 51/53/55 | 51/53/55 | 0/0/0 | 0/0/0 | 57/59/63 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP11 | 162/194/221 | 46/47/47 | 46/47/47 | 0/0/0 | 0/0/0 | 54/56/56 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP12 | 102/141/179 | 12/13/13 | 12/13/13 | 0/0/0 | 0/0/0 | 12/13/13 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP13 | 224/264/296 | 5/12/18 | 5/12/18 | 0/0/0 | 0/0/0 | 14/25/35 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP14 | 111/134/141 | 25/27/27 | 23/25/25 | 5/5/5 | 0/0/0 | 33/34/35 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP15 | 274/342/361 | 70/74/78 | 70/74/78 | 0/0/0 | 0/0/0 | 92/97/101 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP16 | 125/193/236 | 21/28/35 | 4/4/4 | 17/24/31 | 0/0/0 | 13/20/20 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP17 | 320/369/406 | 72/74/76 | 72/74/76 | 0/0/0 | 0/0/0 | 90/97/101 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP18 | 353/408/459 | 27/27/28 | 27/27/28 | 0/0/0 | 0/0/0 | 44/44/45 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP19 | 301/386/421 | 9/10/10 | 9/10/10 | 0/0/0 | 0/0/0 | 11/16/17 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP20 | 182/215/234 | 29/29/29 | 29/29/29 | 0/0/0 | 0/0/0 | 54/56/56 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP21 | 113/162/202 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 2/2/2 | 0/0/0 | pass/pass/pass |
| MAP22 | 72/94/109 | 20/20/20 | 20/20/20 | 0/0/0 | 0/0/0 | 32/37/39 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP23 | 340/384/416 | 1/1/1 | 1/1/1 | 0/0/0 | 0/0/0 | 8/9/9 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP24 | 233/292/351 | 7/8/9 | 7/8/9 | 0/0/0 | 0/0/0 | 10/16/18 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP25 | 172/237/252 | 7/8/8 | 7/8/8 | 0/0/0 | 0/0/0 | 21/24/24 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP26 | 196/243/272 | 26/24/22 | 26/24/22 | 0/0/0 | 0/0/0 | 37/36/35 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP27 | 339/416/468 | 24/25/25 | 24/25/25 | 0/0/0 | 0/0/0 | 37/39/39 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP28 | 149/215/255 | 9/12/16 | 9/12/16 | 0/0/0 | 0/0/0 | 14/20/27 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP29 | 197/240/265 | 64/64/65 | 64/64/65 | 0/0/0 | 0/0/0 | 74/80/82 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP30 | 75/71/72 | 4/4/4 | 0/0/0 | 4/4/4 | 0/0/0 | 1/1/1 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP31 | 190/227/302 | 1/1/1 | 1/1/1 | 0/0/0 | 1/1/1 | 2/2/2 | 0/0/0 | FAIL/FAIL/FAIL |
| MAP32 | 75/85/103 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | 0/0/0 | pass/pass/pass |

## Original-C evidence

`tools/reference/m3h9b_reference.py --check --overflow-check --address-check` (with
`ASAN_OPTIONS=detect_leaks=0`, so LeakSanitizer was disabled, not passed) compiles 154
genuine bodies: the 151 of the M3h9 contract plus `PIT_RadiusAttack`, `P_RadiusAttack`
and `A_Explode`, whose abort stub is removed; the globals `bombsource`, `bombspot` and
`bombdamage` are copied verbatim from p_map.c. It runs 26 scenarios over the ten rooms
`range`, `cluster`, `player_range`, `monsters`, `walls`, `door`, `fireball`, `dense64`,
`angles` and `embedded` (22 distinct maps), 2,671 scripted tics of which 2,661 execute
(`player_killed_by_barrel` ends at its reborn request), all at medium skill. The strict
build and both sanitizer builds produced byte-identical raw traces, SHA-256
`e02df372792a81948a517d1f4982c168dbc16dfed323b9473eec7ee965dee031` (the contract's
`sanitizer_comparison.raw_trace_sha256`), in 14.7 s with a 398 MB peak. No sanitizer
report was produced. Observed totals: 65 barrel spawns (each asserted against the info.c
record), 27 explosions, 79 radius candidates (43 not shootable, 3 out of range, 1 sight
blocked, 32 damaged, 0 boss), 28 barrel deaths (1 gibbed), 37 kills, 596 gameplay draws.
Per explosion the generator asserts the wrapped distance (`128 << 16`, always a 3 × 3
window), the `y` outer, `x` inner walk, that every damaged candidate's `P_DamageMobj` call
matches its row, that the bomb source is the barrel's last target assignment, that every
draw inside the explosion is accounted for, and the death-chain timing `T + d + t0 − 1 /
+4 / +9 / +19 / +29`.

Two declared adapters apply in the sanitizer builds only: the unsigned rewrite of
p_mobj.c:794 is needed (without it both builds abort with `signed integer overflow: 4 *
536870912`); the rewrite of p_map.c:1221 is declared but observed not needed under
`-std=gnu89`, where GCC checks `shift-base` only for C99 and later. Controlled hits
(`hit_*`, inflictor NULL) are used in two scenarios only (`radius_blocked_by_step`,
`fireball_on_dying_barrel`), with the reason in each case; five scenarios start at a
declared seed. The harness's `intercepts[]` carries the M3d3 instrumentation's abort before
a 129th entry, so the source's unchecked overflow is never observed in C; the Rust-only
lane `m3h9b_intercepts129.bin` (≥ 135 intercepts by geometry) is never run in C.

The two fist scenarios first gave the player the fist alone; the DSIM weapon layout cannot
encode that, so they were regenerated with fist and pistol owned and the fist ready, every
observed outcome unchanged (the first traces had raw digest
`83598233039c559a21638524c82e9e4c082f9cc400d49893ff1971b0c2c35877`).

Headline observations: a pistol wound costs exactly 6 draws (damage, 4 puff, pain) and
fixes the barrel's target; `A_Explode` runs at `T + t0 + 9`; in `chain_reaction` a barrel
at Chebyshev distance 136 (dist 125 < 128) in a fourth block is never visited; in
`monster_wakes_mid_walk` an Imp whose pain draw fails relinks inside the walk and another
Imp is damaged twice (39 + 39) and dies; in `barrel_infight` the Zombieman that wounded
the barrel is the bomb source, is hurt 16 by its own explosion and is retargeted by the
Imp; `fireball_kills_barrel` is the `d = 1` timing case; `door_gibs_dying_barrel` turns the
dying barrel to `S_GIBS` under the door; the densest pistol trace collects 92 intercepts;
all 17 test angles equal the wrapped product; the embedded Imp walks out and never through.

## Independent DSIM37 projection

`tools/reference/m3h9b_snapshot_goldens.py --check` projects DSIM37 from the C rows alone
(`barrels_snapshot.py`, extending the frozen DSIM36 projection without editing it): 454
captures of the 26 scenarios in `tests/reference/m3h9b_snapshots.csv`, 2,687 ordered
event frames (45,529 fields) in `m3h9b_events.dref`, 16 full-byte `m3h9b_state_*.bin`
fixtures (a live barrel, a wounded one with a target, each `S_BEXP` state, the `A_Explode`
op, `S_NULL` pending removal, the first cluster explosion, the mid-walk relink, the
`S_GIBS` crunch, the fist kill's explosion, `dense64`, the end of the intercept-heavy trace,
`angles` and `embedded`) and `m3h9b_snapshot_contract.json`. The generator asserts that
every capture equals the DSIM36 projection of the same rows except `version`, `profile`,
`context.schema` and `store.max_slots` (384 against 320; each capture 3 bytes longer, equal
field counts, identical event stream); for `dense_pickups` and `intercept_heavy_trace`,
whose 45 counted items the frozen encoder refuses, the baseline is the frozen build with
only its 16-item and 96-bonus ranges lifted, and that refusal is asserted. 875 barrel
records (661 alive, 188 dying, 22 pending at `S_NULL`, 4 gibbed, 217 with a target), at
most 16 barrels and 114 occupied slots in any capture; 27 explosions with 33 sight records
and 32 damage records. These bytes were committed before the Rust writer and are never
derived from Rust output.

## Rust source replay

`barrels_replay` runs the shared M3h9 comparison engine with `M3H9B_SPEC` (version 37)
over the M3h9b rows and goldens, then a second pass over the barrel rows (spawn idents,
every state entry and action row, every `RA` row, every `RC` candidate in walk order with
the damage row after each damaged one, the contract's death chains), through
`tools/reference/check_player_barrels.py`. **All 26 scenarios matched on the first run; no
simulation bug was found and no fixture, encoder or golden was questioned.** The summary
line, identical in debug and release:

```text
player_barrels full replay: cases=26 ops=2687 event_frames=2687 event_records=45529 captures=454 fixtures=16 thinker_rows=14369 crossings=2 uses=6 blocked=0 explosions=27 candidates=79 not_shootable=43 boss=0 out_of_range=3 sight_blocked=1 damaged=32 explosion_damage=32 spawns=375 state_entries=1802 actions=497 explode_actions=27 barrel_deaths=28 gibbed=1 digest=f421ecf6f6b0c1a3
```

The walk is observed through a test-only, thread-local `barrels::walk_trace` observer that
reads values the walk already computes. The Rust-only lanes: `m3h9b_embedded_barrel.bin`
loads and `spawn_map_things` refuses it with `MonsterError::Arithmetic` (snapshot, RNG
and actor list unchanged; the same barrel moved 15 units out of the wall spawns);
`m3h9b_intercepts129.bin` refuses tic 19 with `MapScaleError::Intercepts` through both
`advance` and `step` with the snapshot and RNG unchanged, the tic on which the same lane
without its 64 bonuses fires its first 2,048-unit shot and is admitted. The example
`player_barrels` replays the 19 public-input cases over 1,621 ops (the controlled-damage
cases 14 and 21 and the 200-health cases 2, 10, 18, 20 and 25 excluded), matches 328
capture digests, asserts the C headline counts and that all 26 rooms are refused by the
E1M1 policy; its `--measure` workloads `cluster_chain` (7 explosions), `dense64_fight` (16
explosions), `range_patrol` (no barrel damage) and `liftroom_ride` (32 `P_ChangeSector`
calls) give the same final digest in every mode (`measure_barrels.py --check`); these runs
were not on an idle machine and are not timing evidence.

## Rust-only policy tests

Rust-only safety and policy tests, not source evidence (listed in
[TESTING](../TESTING.md#original-exploding-barrels-pickup-scale-and-every-thing-angle-c-evidence)):
the plumbing tests (every M3h9 room under wires 20 and 21 with 1,384 captures equal except
the four identity fields; every older reader rejecting 37; a relabelled 36 failing
`Schema`; `max_slots` 385 failing `Limit`), the bounds, angle and S′ tests (an Imp with
margin 16 and 11 admitted and 10 refused; a player body with margin 12 and 14 refused and
15 admitted; a lamp on its box edge and a bonus overlapping a wall admitted; even parity
and out-of-rectangle centres refused for a THING and a dropped clip; the 17 angles; 64
pickups and the 65th `WorkLimit`, 16 and the 17th under wire 20), the 23 barrel combat
tests with expected values derived from the C formulas (the two spawn draws, the pistol
wound's 6 draws and thrust, the kill's `t0 = 5 − (r & 3)` and chain timing, `A_Explode` at
`T + t0 + 9`, the player at 60/80/127/129/150 units taking 84/64/17/none/none with the
fourth-cell quirk at 129, armor, attacker, pain and death with the reborn tic refused,
kill credit with a None and a player source, infighting, the chain reaction with the
137-unit survivor, the 10.5-unit window against 12- and 16-unit ledges, the mid-walk
relink on two seeds, a fireball killing a wounded barrel and passing over a dying one on a
common floor, a fireball absorbed by a dying barrel on a 24-unit pedestal, a pushed barrel
stopping at a wall and crossing the lift line, the door crunch to `S_GIBS`, the
stale-source witness, the API refusals, `A_Explode` admitted for `S_BEXP4` only), 4 × 2,400-tic
soaks with a DSIM37 round trip every 100 tics, every barrel state round-tripping and 26
forged-field negatives of `decode_barrels`. Test totals after plan step 8: `cargo test
--workspace --locked` 1,058 passed, 0 failed, 3 ignored (85 suites); the release run 1,025
passed, 0 failed, 1 ignored (73 suites).

## Fuzzing

Target `player_barrels` compiled: all 48 fuzz targets built with `--sanitizer address`
(cargo-fuzz 0.13.2). The target starts from the 26 scenario starts and applies the bounded
operations listed in
[TESTING](../TESTING.md#original-exploding-barrels-pickup-scale-and-every-thing-angle-c-evidence)
(`-max_len=362`: 2 header bytes plus 40 nine-byte records, at most 1,024 tics an input);
`tools/seed_player_barrels_fuzz.py` writes 3,172 seeds (26 starts × 2 start bytes × 61
patterns). Every AddressSanitizer run set `ASAN_OPTIONS=detect_leaks=0`, so LeakSanitizer
was disabled, not passed. A seed replay is not a campaign.

| Run | Observed |
| --- | --- |
| Seed replay at `ef20de8`, all 3,172 seeds, `-runs=0`, AddressSanitizer | stopped by finding 3 (seed 1367) after 1,463 units in 266 s, peak RSS 757 MB, cov 13,512, ft 54,395 |
| Seed replay at `ef20de8`, the 3,166 seeds without finding 3's six reproducers | 3,168 runs in 830 s (3 exec/s), cov 14,217, ft 62,024, peak RSS 796 MB; no crash, timeout, assertion or sanitizer report |
| Seed replay at `2a3d2cf` (the Escape witness), all 3,172 seeds, `-runs=0`, AddressSanitizer | 3,174 runs in 1,186 s, cov 14,355, ft 63,102, peak RSS 803 MB; no crash, timeout, assertion or sanitizer report; the six finding-3 seeds end in the declared `Escape` refusal; two slow units (over 10 s, under the 15 s timeout) saved; the machine was not idle |
| Coverage-guided and sanitizer campaigns on the final target | recorded under [fuzz campaigns](#fuzz-campaigns) |

Findings of the seed replay, in the order found:

1. **Simulation (fixed in `ebfaa77`).** `damage_actor` accepted a barrel as the damage
   *source*; the wounded victim then targeted the barrel, a monster hurt by its explosion
   targeted it too, and the next valid tic was refused with `PerceptionError::Roster`, not
   a declared witness. The source never passes a barrel as a source (it is only ever an
   inflictor), so the API now refuses it with `PlayerBodyError::Unsupported`, state
   unchanged; pinning test `a_barrel_is_refused_as_a_damage_source`.
2. **Target.** The embedded room failed the target's Ω box clause at spawn, which the
   spawn rule S′ admits for an Imp overlapping a wall; the target now applies the box
   clause once an actor has left its spawn position.
3. **Policy (the barrel corner jump).** Six seeds, each a lethal 1,000-damage hit on the
   corridor-hall barrel, left the barrel at (10, −30) with momentum (−27.19, −27.19) and
   moved it in one step to (−17.19, −57.19), behind the corner at (0, −32): the source
   tests only the destination box and does not halve negative moves, and the Rust move
   matched it, but the barrel's centre left Ω, which the M3h5 proof assumed every mover
   keeps (its step "`30·w < 2r·w`" fails for radius 10). Resolved as the player's corner jump
   was: an exact parity test after every accepted barrel move refuses the tic with
   `MapScaleError::Escape` (note [§13](../M3H9B_REFERENCE_INVESTIGATION.md#13-corrections-and-findings-from-phase-2-kept-with-the-original-text-above));
   fixed in `2a3d2cf` (`BarrelMotion` in barrels.rs through the new `after_move` hook of the
   movement path, barrels under wire 21 only), with the reproducer, a stopped-short control
   and a fast-barrel-along-a-wall control as tests. Test totals after the fix: 1,062 passed,
   0 failed, 3 ignored (85 suites); release 1,029 passed, 0 failed, 1 ignored.

**The 121-thing lift room.** `player_barrels --measure dense64_lift` builds the M3h9
liftroom in memory with 1 player, 8 monsters, 64 pickups, 32 decorations and 16 barrels
(121 things linked at spawn, 120 inside the lift sector's blockbox; the bound of 129 is
reached only through drops after kills, and nothing is killed here) and walks the player
across the lift's WR 88 line for 1,200 tics: 272 `P_ChangeSector` calls (at most 1 per
tic), 32,640 thing clips (at most 120 per tic), digest `775002081fb13dd7` in `advance`,
`step` and `capture` alike (`measure_barrels.py --check`); smoke runs on a busy machine,
not timings.

## Fuzz campaigns

Recorded after the independent review, on the final target.

## Matched performance

Recorded after plan step 13.

## Independent review

Plan step 14, 2026-11-04: an independent reviewer (a fresh agent with the branch, the C
source and the committed documents only, no access to the implementing sessions' notes)
reviewed the full diff against `origin/master` (96 files, +15,593 / −209) against the
original C, re-ran the M3h9b evidence, projection and replay checks and the M3h8 and M3h9
replays (all passing, digests unchanged). **No blocker.** It verified the explosion walk,
`PIT_RadiusAttack`, the barrel as a damage target, the `A_Explode` admission, the spawn
classification and bounds, S′ (including an independent check of the proof and of the
"step below 2r" condition for every admitted mover: player 16, monsters 20, sergeant 30,
fireball 6; only the barrel fails it and is covered by the Escape witness), the C harness
bodies, observers and adapters, the encoder's honesty and the reader's record rules
against the C lines. Findings and their resolution:

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | should fix | COMPATIBILITY and the note's §5.1 said `A_Explode` runs 9 tics after the death state's first tic; C gives `T + d + t0 + 9`, 11–14 tics after the kill | corrected; note §13 |
| 2 | should fix | STATE_FORMAT said a barrel's spawnpoint is at the actor's position; a thrust barrel leaves it | corrected |
| 3 | should fix | ROADMAP named one new witness; there are two (stale source, corner jump) | corrected |
| 4 | should fix | README overclaimed "every feature it needs now works" | reworded: accepted and checked; loading and matching the run is next |
| 5 | should fix | COMPATIBILITY's API table lacked the refusal of a barrel as a damage *source* | row added |
| 6 | should fix | four stale code comments (barrels "land in a later step", sight bound 24, `has_e1m1` "every other policy rejects") | fixed in code |
| 7 | should fix | a documentation-only change skipped the historical-evidence guard (`evidence_storage.py verify-git`), so a protected `docs/validation` record could change unguarded | paths under `docs/validation/` are never documentation-only |
| 8 | note | a failing detect-changes job left the required jobs skipped or pending | both jobs run with `!cancelled()`; an empty output means a full run |
| 9 | note | rename detection could make a code file renamed to `.md` look documentation-only | `--no-renames` |
| 10 | note | the readers accepted threshold 100 with reaction time 8, which C never produces (p_inter.c:902 then 904–911) | both validators require reaction time 0 with threshold 100; negative test |
| 11 | note | README "Which kind of Doom?" and the "Where it's (maybe) going" edits need the owner's approval cited; TESTING claimed "about a minute" for an unobserved run | approvals cited in the pull request; the claim removed |

Verified and left untouched: everything in the reviewer's "found correct" list (the
report is `review_report.md` in the milestone's scratch, summarised here).

## Local gates

Recorded after plan step 15.

## CI

Recorded when the pull request's checks have run on the exact head, including the
docs-only fast path proof.
