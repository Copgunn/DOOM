# M3h9 IWAD THING census

Output of `tools/iwad_counts.py --things-markdown` over the owner's two classic IWADs (68
map slots), read locally and read-only on 2026-11-03 for the
[M3h9 investigation](../M3H9_REFERENCE_INVESTIGATION.md) §3. The IWADs never enter the
repository, CI or logs; the script is local-only, is never run by CI, and is a no-op when
the IWAD directory is absent. This page holds counts and type facts only: a map is named
by its slot, a THING by its DoomEd number and `mobjinfo` entry (game-rule facts from
`info.c`), and no coordinate, angle or lump datum of any object appears. It complements
the geometry table of [M3h6](M3H6_IWAD_COUNTS.md) and the movers table of
[M3h8](M3H8_IWAD_COUNTS.md), both unchanged.

```sh
python3 tools/iwad_counts.py --things E1M1        # the per-doomednum census of one slot
python3 tools/iwad_counts.py --things-markdown    # the two tables below
```

## How the counts are defined

The script follows `P_SpawnMapThing` (p_mobj.c:708–795) in single player: deathmatch
starts (type 11) are stored, up to `MAX_DEATHMATCH_STARTS` (10); player starts 1–4 are
stored and only start 1 spawns a body; a record with `options & 16` is skipped; the skill
bit is 1 for baby and easy, 2 for medium and 4 for hard and nightmare (p_mobj.c:740–751);
a type without a `mobjinfo` DoomEd number reaches `I_Error`. "Spawned" counts every
`P_SpawnMobj` of that skill (monsters, pickups, decorations, barrels); "Monsters" is
`MF_COUNTKILL` or `MT_SKULL` (the `-nomonsters` test); "Pickups" is `MF_SPECIAL`;
decorations are every other type, split on `MF_SOLID` (corpses included); barrels are
DoomEd 2035. Columns written `e/m/h` are the easy, medium and hard single-player values.

## Census tables

| Slot | THINGs | Spawned e/m/h | Monsters e/m/h | Pickups e/m/h | Solid deco e/m/h | Non-solid deco e/m/h | Barrels e/m/h | P1 start | P2-4 starts | DM starts | Unknown types |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1M1 | 143 | 89/91/115 | 4/6/29 | 52/52/53 | 12/12/12 | 15/15/15 | 6/6/6 | 1 | 3 | 5 | 0 |
| E1M2 | 262 | 179/199/238 | 20/41/79 | 107/106/107 | 3/3/3 | 25/25/25 | 24/24/24 | 1 | 3 | 10 | 0 |
| E1M3 | 380 | 256/309/364 | 34/74/131 | 164/175/175 | 17/17/15 | 15/15/15 | 26/28/28 | 1 | 3 | 9 | 0 |
| E1M4 | 254 | 169/200/228 | 21/54/85 | 91/89/86 | 4/4/4 | 21/21/21 | 32/32/32 | 1 | 3 | 9 | 0 |
| E1M5 | 293 | 151/215/256 | 24/91/131 | 96/93/94 | 2/2/2 | 1/1/1 | 28/28/28 | 1 | 3 | 8 | 0 |
| E1M6 | 463 | 286/369/439 | 50/118/177 | 199/214/223 | 15/15/15 | 0/0/0 | 22/22/24 | 1 | 3 | 11 | 0 |
| E1M7 | 358 | 211/266/329 | 32/84/150 | 143/146/143 | 20/20/20 | 0/0/0 | 16/16/16 | 1 | 3 | 10 | 0 |
| E1M8 | 126 | 93/98/112 | 21/27/41 | 37/36/36 | 6/6/6 | 1/1/1 | 28/28/28 | 1 | 3 | 9 | 0 |
| E1M9 | 237 | 152/191/222 | 33/72/103 | 79/79/79 | 12/12/12 | 8/8/8 | 20/20/20 | 1 | 3 | 10 | 0 |
| E2M1 | 127 | 77/99/114 | 15/39/54 | 42/40/40 | 0/0/0 | 20/20/20 | 0/0/0 | 1 | 3 | 4 | 0 |
| E2M2 | 311 | 229/260/285 | 52/86/115 | 150/147/143 | 15/15/15 | 9/9/9 | 3/3/3 | 1 | 3 | 7 | 0 |
| E2M3 | 229 | 154/196/207 | 28/66/86 | 78/82/73 | 14/14/14 | 24/24/24 | 10/10/10 | 1 | 3 | 4 | 0 |
| E2M4 | 265 | 199/225/231 | 46/73/80 | 104/103/102 | 28/28/28 | 18/18/18 | 3/3/3 | 1 | 3 | 7 | 0 |
| E2M5 | 326 | 249/277/312 | 70/103/138 | 107/102/102 | 38/38/38 | 34/34/34 | 0/0/0 | 1 | 3 | 5 | 0 |
| E2M6 | 326 | 223/265/304 | 60/101/141 | 123/123/122 | 30/31/31 | 10/10/10 | 0/0/0 | 1 | 3 | 5 | 0 |
| E2M7 | 235 | 187/201/218 | 64/78/95 | 96/96/96 | 12/12/12 | 14/14/14 | 1/1/1 | 1 | 3 | 5 | 0 |
| E2M8 | 66 | 35/39/55 | 1/5/21 | 34/34/34 | 0/0/0 | 0/0/0 | 0/0/0 | 1 | 3 | 7 | 0 |
| E2M9 | 53 | 40/40/40 | 14/14/14 | 22/22/22 | 0/0/0 | 4/4/4 | 0/0/0 | 1 | 3 | 8 | 0 |
| E3M1 | 92 | 64/70/80 | 8/16/26 | 25/23/23 | 23/23/23 | 8/8/8 | 0/0/0 | 1 | 3 | 4 | 0 |
| E3M2 | 208 | 161/176/186 | 48/65/80 | 75/73/68 | 28/28/28 | 10/10/10 | 0/0/0 | 1 | 3 | 5 | 0 |
| E3M3 | 209 | 139/164/182 | 25/51/71 | 79/78/76 | 24/24/24 | 11/11/11 | 0/0/0 | 1 | 3 | 7 | 0 |
| E3M4 | 374 | 230/297/357 | 37/100/161 | 145/149/148 | 21/21/21 | 12/12/12 | 15/15/15 | 1 | 3 | 4 | 0 |
| E3M5 | 291 | 213/244/264 | 43/74/94 | 120/120/120 | 29/29/29 | 21/21/21 | 0/0/0 | 1 | 3 | 5 | 0 |
| E3M6 | 335 | 241/280/311 | 60/98/126 | 91/91/94 | 75/75/75 | 15/16/16 | 0/0/0 | 1 | 3 | 6 | 0 |
| E3M7 | 154 | 114/125/126 | 21/36/47 | 76/72/62 | 0/0/0 | 17/17/17 | 0/0/0 | 1 | 3 | 4 | 0 |
| E3M8 | 58 | 30/31/31 | 3/4/4 | 26/26/26 | 0/0/0 | 1/1/1 | 0/0/0 | 1 | 3 | 4 | 0 |
| E3M9 | 197 | 169/175/183 | 70/78/88 | 67/65/63 | 23/23/23 | 9/9/9 | 0/0/0 | 1 | 3 | 4 | 0 |
| E4M1 | 141 | 97/118/123 | 33/54/63 | 37/37/33 | 7/7/7 | 20/20/20 | 0/0/0 | 1 | 3 | 4 | 0 |
| E4M2 | 227 | 158/192/213 | 33/69/91 | 86/84/83 | 33/33/33 | 4/4/4 | 2/2/2 | 1 | 3 | 5 | 0 |
| E4M3 | 384 | 227/258/293 | 79/115/150 | 119/114/114 | 26/26/26 | 3/3/3 | 0/0/0 | 1 | 3 | 4 | 0 |
| E4M4 | 153 | 86/101/115 | 27/45/60 | 42/39/38 | 12/12/12 | 5/5/5 | 0/0/0 | 1 | 3 | 6 | 0 |
| E4M5 | 267 | 209/238/251 | 54/82/90 | 87/88/93 | 17/17/17 | 10/10/10 | 41/41/41 | 1 | 3 | 6 | 0 |
| E4M6 | 295 | 232/266/273 | 51/92/104 | 97/90/85 | 50/50/50 | 34/34/34 | 0/0/0 | 1 | 3 | 5 | 0 |
| E4M7 | 258 | 202/208/239 | 87/88/104 | 63/68/83 | 41/41/41 | 11/11/11 | 0/0/0 | 1 | 3 | 5 | 0 |
| E4M8 | 315 | 209/245/278 | 53/87/120 | 86/88/88 | 64/64/64 | 6/6/6 | 0/0/0 | 1 | 3 | 4 | 0 |
| E4M9 | 247 | 187/206/228 | 57/76/96 | 66/66/68 | 15/15/15 | 9/9/9 | 40/40/40 | 1 | 3 | 6 | 0 |
| MAP01 | 69 | 33/43/51 | 9/19/27 | 24/24/24 | 0/0/0 | 0/0/0 | 0/0/0 | 1 | 3 | 4 | 0 |
| MAP02 | 173 | 107/130/150 | 47/70/90 | 41/41/41 | 0/0/0 | 1/1/1 | 18/18/18 | 1 | 3 | 6 | 0 |
| MAP03 | 148 | 74/101/123 | 29/56/78 | 30/30/30 | 4/4/4 | 11/11/11 | 0/0/0 | 1 | 3 | 4 | 0 |
| MAP04 | 138 | 90/102/119 | 26/38/55 | 52/52/52 | 4/4/4 | 4/4/4 | 4/4/4 | 1 | 3 | 5 | 0 |
| MAP05 | 217 | 117/157/188 | 53/92/124 | 41/42/41 | 18/18/18 | 5/5/5 | 0/0/0 | 1 | 3 | 5 | 0 |
| MAP06 | 247 | 163/200/221 | 54/92/114 | 59/58/57 | 27/27/27 | 14/14/14 | 9/9/9 | 1 | 3 | 6 | 0 |
| MAP07 | 98 | 63/69/76 | 6/12/19 | 57/57/57 | 0/0/0 | 0/0/0 | 0/0/0 | 1 | 3 | 6 | 0 |
| MAP08 | 242 | 152/196/228 | 66/111/144 | 70/69/68 | 2/2/2 | 14/14/14 | 0/0/0 | 1 | 3 | 6 | 0 |
| MAP09 | 392 | 161/273/370 | 55/141/238 | 99/125/125 | 5/5/5 | 2/2/2 | 0/0/0 | 1 | 3 | 7 | 0 |
| MAP10 | 509 | 301/360/483 | 103/156/279 | 190/196/196 | 4/4/4 | 4/4/4 | 0/0/0 | 1 | 3 | 9 | 0 |
| MAP11 | 238 | 161/193/220 | 21/53/80 | 92/92/92 | 42/42/42 | 6/6/6 | 0/0/0 | 1 | 3 | 7 | 0 |
| MAP12 | 195 | 101/140/178 | 36/71/109 | 61/65/65 | 2/2/2 | 2/2/2 | 0/0/0 | 1 | 3 | 6 | 0 |
| MAP13 | 312 | 223/263/295 | 59/94/126 | 149/152/152 | 0/2/2 | 15/15/15 | 0/0/0 | 1 | 3 | 8 | 0 |
| MAP14 | 173 | 110/133/140 | 54/76/84 | 29/30/29 | 23/23/23 | 4/4/4 | 0/0/0 | 1 | 3 | 7 | 0 |
| MAP15 | 376 | 273/341/360 | 90/158/177 | 119/119/119 | 48/48/48 | 16/16/16 | 0/0/0 | 1 | 3 | 8 | 0 |
| MAP16 | 249 | 124/192/235 | 53/118/161 | 58/61/61 | 5/5/5 | 8/8/8 | 0/0/0 | 1 | 3 | 7 | 0 |
| MAP17 | 421 | 319/368/405 | 38/87/125 | 211/211/210 | 47/47/47 | 7/7/7 | 16/16/16 | 1 | 3 | 7 | 0 |
| MAP18 | 498 | 352/407/458 | 72/127/178 | 272/272/272 | 0/0/0 | 8/8/8 | 0/0/0 | 1 | 3 | 5 | 0 |
| MAP19 | 437 | 300/385/420 | 103/174/208 | 151/164/165 | 35/35/35 | 11/12/12 | 0/0/0 | 1 | 3 | 6 | 0 |
| MAP20 | 256 | 181/214/233 | 20/53/80 | 115/115/107 | 34/34/34 | 12/12/12 | 0/0/0 | 1 | 3 | 9 | 0 |
| MAP21 | 217 | 112/161/201 | 35/84/124 | 59/59/59 | 1/1/1 | 11/11/11 | 6/6/6 | 1 | 3 | 6 | 0 |
| MAP22 | 121 | 71/93/108 | 21/41/56 | 27/29/29 | 18/18/18 | 5/5/5 | 0/0/0 | 1 | 3 | 5 | 0 |
| MAP23 | 431 | 339/383/415 | 33/77/106 | 76/76/79 | 15/15/15 | 7/7/7 | 208/208/208 | 1 | 3 | 7 | 0 |
| MAP24 | 364 | 232/291/350 | 51/108/167 | 168/170/170 | 4/4/4 | 9/9/9 | 0/0/0 | 1 | 3 | 6 | 0 |
| MAP25 | 276 | 171/236/251 | 47/93/104 | 61/71/70 | 35/35/35 | 18/27/32 | 10/10/10 | 1 | 3 | 6 | 0 |
| MAP26 | 296 | 195/242/271 | 18/67/101 | 140/138/133 | 11/11/11 | 15/15/15 | 11/11/11 | 1 | 3 | 7 | 0 |
| MAP27 | 490 | 338/415/467 | 53/118/170 | 206/218/218 | 34/34/34 | 45/45/45 | 0/0/0 | 1 | 3 | 8 | 0 |
| MAP28 | 266 | 148/214/254 | 29/83/123 | 90/102/102 | 25/25/25 | 3/3/3 | 1/1/1 | 1 | 3 | 5 | 0 |
| MAP29 | 276 | 196/239/264 | 41/86/107 | 85/83/87 | 54/54/54 | 16/16/16 | 0/0/0 | 1 | 3 | 4 | 0 |
| MAP30 | 95 | 74/70/71 | 0/0/1 | 54/50/50 | 3/3/3 | 17/17/17 | 0/0/0 | 1 | 3 | 5 | 0 |
| MAP31 | 317 | 189/226/301 | 37/74/149 | 107/107/107 | 29/29/29 | 11/11/11 | 5/5/5 | 1 | 3 | 7 | 0 |
| MAP32 | 111 | 74/84/102 | 5/15/33 | 43/43/43 | 26/26/26 | 0/0/0 | 0/0/0 | 1 | 3 | 5 | 0 |

things: maps 68; THINGS records in all 17307, maximum 509
things: maxima per skill e/m/h: spawned 352/415/483, monsters 103/174/279, pickups 272/272/272, solid decorations 75/75/75, non-solid decorations 45/45/45, barrels 208/208/208, totalkills 99/164/261, totalitems 182/182/182
things: maxima: player 2-4 starts 3, deathmatch starts 11 (beyond MAX_DM_STARTS on 1 of 68)
things: exactly one player 1 start for 68 of 68; no unknown type for 68 of 68; no non-positive type for 68 of 68; barrels present on 27 of 68; corpses present on 47 of 68
things: distinct doomednums over all maps 123; unmatched none

E1M1: one row per doomednum

| Doomednum | Type | Name | Kind | Flags | Radius | Height | Total | Easy | Medium | Hard | Ambush | Bit 16 | Handled e/m/h |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | - | - | player start | - | - | - | 1 | 1 | 1 | 1 | 0 | 0 | 1/1/1 |
| 2 | - | - | player start | - | - | - | 1 | 1 | 1 | 1 | 0 | 0 | 1/1/1 |
| 3 | - | - | player start | - | - | - | 1 | 1 | 1 | 1 | 0 | 0 | 1/1/1 |
| 4 | - | - | player start | - | - | - | 1 | 1 | 1 | 1 | 0 | 0 | 1/1/1 |
| 9 | MT_SHOTGUY | shotgun guy | monster | COUNTKILL SHOOTABLE SOLID | 20 | 56 | 16 | 0 | 0 | 16 | 15 | 0 | 0/0/16 |
| 10 | MT_MISC68 | bloody mess | corpse | none | 20 | 16 | 2 | 2 | 2 | 2 | 0 | 0 | 2/2/2 |
| 11 | - | - | dm start | - | - | - | 5 | 5 | 5 | 5 | 0 | 0 | 5/5/5 |
| 12 | MT_MISC69 | bloody mess | corpse | none | 20 | 16 | 2 | 2 | 2 | 2 | 0 | 0 | 2/2/2 |
| 15 | MT_MISC62 | dead player | corpse | none | 20 | 16 | 4 | 4 | 4 | 4 | 0 | 0 | 4/4/4 |
| 24 | MT_MISC71 | pool of blood and flesh | corpse | none | 20 | 16 | 7 | 7 | 7 | 7 | 0 | 0 | 7/7/7 |
| 35 | MT_MISC50 | candelabra | obstacle | SOLID | 16 | 16 | 2 | 2 | 2 | 2 | 0 | 0 | 2/2/2 |
| 48 | MT_MISC48 | tall techno pillar | obstacle | SOLID | 16 | 16 | 2 | 2 | 2 | 2 | 0 | 0 | 2/2/2 |
| 2001 | MT_SHOTGUN | shotgun | pickup | SPECIAL | 20 | 16 | 2 | 2 | 2 | 2 | 0 | 1 | 1/1/1 |
| 2002 | MT_CHAINGUN | chaingun | pickup | SPECIAL | 20 | 16 | 1 | 1 | 1 | 1 | 0 | 1 | 0/0/0 |
| 2003 | MT_MISC27 | rocket launcher | pickup | SPECIAL | 20 | 16 | 1 | 1 | 1 | 1 | 0 | 1 | 0/0/0 |
| 2007 | MT_CLIP | ammo clip | pickup | SPECIAL | 20 | 16 | 2 | 2 | 2 | 2 | 2 | 0 | 2/2/2 |
| 2008 | MT_MISC22 | shotgun shells | pickup | SPECIAL | 20 | 16 | 3 | 3 | 3 | 3 | 0 | 1 | 2/2/2 |
| 2011 | MT_MISC10 | stimpack | pickup | SPECIAL | 20 | 16 | 1 | 1 | 1 | 1 | 1 | 0 | 1/1/1 |
| 2012 | MT_MISC11 | medikit | pickup | SPECIAL | 20 | 16 | 3 | 3 | 3 | 3 | 2 | 0 | 3/3/3 |
| 2014 | MT_MISC2 | health bonus | pickup | COUNTITEM SPECIAL | 20 | 16 | 13 | 12 | 12 | 13 | 5 | 0 | 12/12/13 |
| 2015 | MT_MISC3 | armor bonus | pickup | COUNTITEM SPECIAL | 20 | 16 | 25 | 25 | 25 | 25 | 1 | 0 | 25/25/25 |
| 2018 | MT_MISC0 | green armor | pickup | SPECIAL | 20 | 16 | 1 | 1 | 1 | 1 | 0 | 0 | 1/1/1 |
| 2019 | MT_MISC1 | blue armor | pickup | SPECIAL | 20 | 16 | 1 | 1 | 1 | 1 | 0 | 0 | 1/1/1 |
| 2023 | MT_MISC13 | berserk | pickup | COUNTITEM SPECIAL | 20 | 16 | 1 | 1 | 1 | 0 | 0 | 1 | 0/0/0 |
| 2028 | MT_MISC31 | floor lamp | obstacle | SOLID | 16 | 16 | 8 | 8 | 8 | 8 | 4 | 0 | 8/8/8 |
| 2035 | MT_BARREL | exploding barrel | barrel | NOBLOOD SHOOTABLE SOLID | 10 | 42 | 9 | 9 | 9 | 9 | 3 | 3 | 6/6/6 |
| 2046 | MT_MISC19 | box of rockets | pickup | SPECIAL | 20 | 16 | 3 | 3 | 3 | 3 | 0 | 3 | 0/0/0 |
| 2048 | MT_MISC17 | box of bullets | pickup | SPECIAL | 20 | 16 | 6 | 6 | 6 | 6 | 1 | 5 | 1/1/1 |
| 2049 | MT_MISC23 | box of shells | pickup | SPECIAL | 20 | 16 | 6 | 6 | 6 | 6 | 1 | 3 | 3/3/3 |
| 3001 | MT_TROOP | imp | monster | COUNTKILL SHOOTABLE SOLID | 20 | 56 | 4 | 2 | 2 | 4 | 4 | 0 | 2/2/4 |
| 3004 | MT_POSSESSED | zombieman | monster | COUNTKILL SHOOTABLE SOLID | 20 | 56 | 9 | 2 | 4 | 9 | 9 | 0 | 2/4/9 |
