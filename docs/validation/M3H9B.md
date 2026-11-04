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
