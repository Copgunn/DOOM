# M3h8 IWAD mover counts

Output of `tools/iwad_counts.py` over the owner's two classic IWADs (68 map slots), read
locally and read-only on 2026-11-03 for the M3h8 Stage B (doors, lifts, floors, switches,
exit) load rules of the [M3h7 investigation](../M3H7_REFERENCE_INVESTIGATION.md) §4.3,
§4.4, §4.6 and §5.6. The IWADs never enter the repository, CI or logs; the script is
local-only, is never run by CI, and is a no-op when the IWAD directory is absent. This
page holds counts and pass/fail facts only: a map is named by its slot, line and sector
special type numbers are game-rule facts, heights are plain integers, and no lump name
beyond the slot, texture or flat name, or coordinate of any object appears. It
complements the geometry table of [M3h6](M3H6_IWAD_COUNTS.md), which is unchanged.

```sh
python3 tools/iwad_counts.py --movers E1M1        # every measurement of one slot
python3 tools/iwad_counts.py --movers-markdown    # the movers table below
python3 tools/iwad_counts.py --specials-markdown  # the M3h7 §5.6 specials table below
```

## How the counts are defined

The definitions follow the original C source (`linuxdoom-1.10/`); the script's comments
name each function and line it follows.

- **Line kinds** come from `P_CrossSpecialLine`, `P_ShootSpecialLine` (p_spec.c) and
  `P_UseSpecialLine` (p_switch.c). A *tag-using* line is every special that walks
  `P_FindSectorFromLineTag`: everything but manual doors (DR/D1 read the back side),
  exits and the scroller 48. Its *targets* are the sectors whose tag equals the line's
  tag (tag 0 selects every untagged sector). Stairs and donuts are counted on their
  tagged sectors only.
- **Mover families** are the `specialdata` structures: door (DR/D1 back sectors and
  tagged door targets), plat, floor (floors, stairs, donuts) and ceiling; line 40 is
  floor and ceiling. "Every special" columns use all vanilla types; "admitted set"
  figures use E1M1's seven types {1, 63, 109, 88, 36, 38, 11} only, as the load rules do.
- **Neighbors** are `getNextSector` results (p_spec.c:250–261): `ML_TWOSIDED` lines only,
  the sector itself for a line with the same sector on both sides.
- **Reachable heights** (§4.4 rule 4) are an exact-integer interval fixpoint over every
  floor and ceiling, using only the admitted lines of each map: doors 1 and 63 move the
  ceiling over [floor, lowest neighbor ceiling − 4], 109 (`blazeOpen`) raises it to that
  top, lift 88 moves the floor over [min(lowest neighbor floor, floor), floor], 38 lowers
  it to the lowest neighbor floor, 36 (`turboLower`) moves it to the highest neighbor
  floor + 8 (−500 with no neighbor; no + 8 when equal to the floor, p_floor.c:307–315).
  DR, SR and WR rules repeat to a fixpoint; each W1 line (109, 36, 38) fires at most once,
  so after the first closure there are at most as many rounds as W1 lines, each letting
  every W1 line fire from the current intervals. A door *inverts* when its smallest
  possible top is below its floor (taken at its highest reachable value). The *turbo
  jump-up* is the largest one-tic rise of a 36 target (destination above the floor), the
  *door top drop* the largest one-tic drop of a door ceiling above its top
  (`T_MovePlane` sets the ceiling to the destination at once, p_floor.c:167–181), the
  *max step* the largest one-tic plane movement of any admitted mover (109 moves 8 a
  tic), and *B2 Δ* the largest one-tic floor rise or ceiling drop (closing door 2, rising
  lift 4, turbo jump-up, door top drop), the Δ of §4.3 (B2).
- The **`spechit` window** is the largest number of distinct two-sided lines with a
  nonzero special listed in any 2 × 2 cell window of the map's own BLOCKMAP lump.
- **Stage B load rules** (column *Stage B*): M3h6 geometry admission, sector specials in
  the Stage A set, every scroller one-sided (Stage A), and §4.4 rules 1–6 over the
  admitted set. The failing rules are named; a map using any other line special fails
  rule 1 regardless, so the remaining rules are informative only there.

## E1M1 in full

```text
slot E1M1
line special types: 1:8 11:1 36:1 38:1 48:8 63:1 88:1 109:1
sector special types: 1:1 7:4 8:2 9:3 12:1
tag/use/cross line types (all but 48): 1:8 11:1 36:1 38:1 63:1 88:1 109:1
  equal to {1, 63, 109, 88, 36, 38, 11}: True
tagged lines 5; tag-using lines 5; tagged lines whose special ignores the tag 0
  line 36 (W1 floor): 1 target sectors; target sector special types 0:1; target families ['floor']; sky targets 0
  line 38 (W1 floor): 1 target sectors; target sector special types 0:1; target families ['floor']; sky targets 0
  line 63 (SR door): 1 target sectors; target sector special types 0:1; target families ['door']; sky targets 0
  line 88 (WR plat): 1 target sectors; target sector special types 9:1; target families ['plat']; sky targets 0
  line 109 (W1 door): 1 target sectors; target sector special types 0:1; target families ['door']; sky targets 0
manual door lines by type 1:8; one-sided none
scrollers 8; two-sided 8
mover sectors (every special) 9; by family {'door': 6, 'floor': 2, 'plat': 1}
  (admitted set) 9: door 6, plat 1, floor 2
sectors in more than one mover family: every special 0, admitted set 0; DR back sectors that are non-door targets 0
plat sectors 1 (≤ 30 pass); reusable switch lines 1 (≤ 16 pass)
sky mover sectors: every special 0, admitted set 0
self-referencing lines of door sectors 0; door sectors with one 0
door sectors without a two-sided neighbor 0; mover sectors 0
tag-0 lines with tag-using specials: every special 0, admitted set 0
2x2 window max two-sided special lines 8 (≤ 8 pass)
admitted-set fixpoint: door sectors 6; reachable [-136, 264]; within ±512 pass; sectors widened 9; inverting doors 0; turbo jump-up 0; door top drop 0; largest one-tic plane step 8; B2 Δ (largest floor rise or ceiling drop) 4
line specials outside the Stage B set: none
stage B load rules FAIL: two-sided scroller (Stage A)
```

Against the assumptions of the M3h7 note:

- **Confirmed**: E1M1's tag/use/cross line types are exactly {1, 63, 109, 88, 36, 38,
  11}; its 5 tagged lines are its 5 tag-using lines and each hits exactly one sector (two
  floor, two door, one plat target; the plat target is a secret sector, special 9).
  Every DR line is two-sided, no sector is in two mover families, no door sector has a
  self-referencing line or lacks a two-sided neighbor, no mover sector has an `F_SKY1`
  ceiling, no tag-0 line uses a tag, 1 plat sector (≤ 30) and 1 reusable switch (≤ 16).
- **Reachable heights** stay at E1M1's static range (−136 to 264, within ±512); no door
  inverts, the turbo lower never jumps up (its destination is below its floor) and no
  door ceiling drops to its top, so B2 Δ is 4 (the lift) and the largest plane step is 8
  (109 `blazeOpen` raising its ceiling).
- **Surprise: all 8 of E1M1's scrollers (48) are two-sided** and non-blocking. M3h7
  Stage A admits 48 on one-sided lines only, so E1M1 fails Stage A and therefore Stage B
  on that rule alone. They also enter `spechit` (§4.6 assumed scrollers usually
  one-sided): E1M1's 2 × 2 window maximum is **exactly 8**, at the bound, and all 8
  lines of that window are scrollers (the largest window of non-scroller two-sided
  special lines is 4). Admitting E1M1 needs an owner decision on two-sided scrollers.

## Movers table

| Slot | Outside Stage B set | Tag-using lines | Targets | Tag 0 | Manual door one-sided | Mover sectors | Families | Multi-family | Plat sectors | Reusable switches | Scrollers (two-sided) | Sky movers | Self-ref door lines | Doors w/o neighbor | 2x2 two-sided specials | Reachable (admitted) | Inverting doors | Turbo jump-up | Door top drop | Max step | B2 Δ | Stage B | Failing rules |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1M1 | none | 5 | 5 | 0 | 0 | 9 | door 6, floor 2, plat 1 | 0 | 1 | 1 | 8 (8) | 0 | 0 | 0 | 8 | [-136, 264] | 0 | 0 | 0 | 8 | 4 | FAIL | two-sided scroller (Stage A) |
| E1M2 | 9:1 28:2 31:2 46:1 62:6 103:3 | 22 | 23 | 0 | 0 | 19 | door 13, floor 1, plat 5 | 0 | 5 | 7 | 0 (0) | 0 | 0 | 0 | 4 | [-232, 512] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| E1M3 | 2:9 8:1 20:2 26:2 27:2 31:4 35:3 51:1 62:4 90:2 103:1 | 30 | 41 | 0 | 0 | 30 | door 20, floor 1, plat 9 | 0 | 9 | 5 | 0 (0) | 3 | 0 | 0 | 6 | [-32, 528] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512 |
| E1M4 | 2:1 5:1 18:2 26:2 27:2 31:2 62:3 86:4 90:12 | 28 | 80 | 0 | 0 | 17 | door 11, floor 4, plat 3 | 1 | 3 | 3 | 0 (0) | 0 | 0 | 0 | 6 | [-64, 400] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set |
| E1M5 | 2:2 22:1 26:3 27:2 31:6 62:3 70:2 90:1 91:10 97:10 98:3 103:2 | 39 | 60 | 0 | 0 | 27 | door 21, floor 2, plat 4 | 0 | 4 | 5 | 0 (0) | 0 | 0 | 0 | 14 | [-216, 248] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; 6: spechit window |
| E1M6 | 2:8 16:1 26:3 27:6 28:4 62:4 76:2 103:3 | 27 | 45 | 0 | 0 | 34 | door 30, floor 1, plat 3 | 0 | 3 | 4 | 0 (0) | 2 | 0 | 0 | 6 | [-80, 304] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set |
| E1M7 | 2:3 22:1 23:1 26:2 27:6 28:2 31:3 34:3 62:7 70:1 90:3 91:7 98:1 103:1 | 39 | 41 | 0 | 0 | 26 | door 18, floor 2, plat 6 | 0 | 6 | 10 | 4 (4) | 0 | 0 | 0 | 8 | [-32, 320] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; two-sided scroller (Stage A) |
| E1M8 | 2:3 7:1 23:1 31:4 62:2 82:1 91:1 97:8 | 17 | 20 | 0 | 0 | 9 | door 5, floor 3, plat 1 | 0 | 1 | 2 | 0 (0) | 2 | 0 | 0 | 8 | [-136, 240] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); sector specials; 1: line specials outside the admitted set |
| E1M9 | 2:17 20:1 31:2 32:2 33:4 34:4 62:6 90:2 97:2 103:1 | 41 | 47 | 0 | 0 | 23 | door 14, floor 3, plat 6 | 0 | 6 | 6 | 0 (0) | 4 | 0 | 0 | 9 | [-168, 304] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; 6: spechit window |
| E2M1 | 7:1 14:1 26:2 28:2 29:1 37:6 97:20 102:3 103:1 | 35 | 36 | 0 | 0 | 14 | door 6, floor 7, plat 1 | 0 | 1 | 1 | 0 (0) | 0 | 0 | 0 | 5 | [-88, 384] | 0 | 0 | 0 | 4 | 2 | FAIL | 1: line specials outside the admitted set |
| E2M2 | 2:6 8:1 9:1 10:1 13:4 16:1 19:3 20:1 21:1 23:4 26:2 27:2 30:1 31:2 32:1 33:2 59:5 73:5 74:3 77:2 87:3 89:5 102:2 103:4 | 54 | 191 | 0 | 0 | 51 | ceiling 5, door 19, floor 14, plat 15 | 2 | 15 | 0 | 0 (0) | 0 | 0 | 0 | 5 | [-48, 320] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| E2M3 | 26:2 87:3 89:5 | 8 | 96 | 0 | 0 | 19 | door 7, plat 12 | 0 | 12 | 0 | 12 (12) | 0 | 0 | 0 | 10 | [-48, 256] | 0 | 0 | 0 | 2 | 2 | FAIL | 1: line specials outside the admitted set; two-sided scroller (Stage A); 6: spechit window |
| E2M4 | 2:7 3:1 7:3 19:2 20:1 22:1 23:1 24:1 31:7 32:1 34:1 46:6 56:1 58:5 62:3 73:2 74:7 77:4 97:10 102:1 | 60 | 85 | 0 | 0 | 47 | ceiling 4, door 17, floor 22, plat 5 | 1 | 5 | 9 | 1 (0) | 0 | 0 | 0 | 6 | [-112, 624] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512 |
| E2M5 | 2:10 7:1 20:1 31:1 51:1 62:2 97:14 103:1 104:1 | 63 | 69 | 0 | 0 | 42 | door 38, floor 4, plat 3 | 3 | 3 | 34 | 0 (0) | 1 | 0 | 0 | 6 | [-152, 320] | 0 | 0 | 0 | 4 | 2 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| E2M6 | 2:20 20:1 23:1 27:5 31:2 32:1 33:1 37:1 61:1 73:1 74:1 103:4 | 30 | 55 | 0 | 0 | 43 | ceiling 1, door 39, floor 2, plat 1 | 0 | 1 | 1 | 6 (0) | 2 | 0 | 0 | 6 | [-24, 288] | 0 | 0 | 0 | 2 | 2 | FAIL | 1: line specials outside the admitted set |
| E2M7 | 16:1 20:1 32:2 33:2 34:2 97:3 102:3 103:6 65535:1 | 14 | 16 | 0 | 0 | 25 | door 22, floor 2, plat 1 | 0 | 1 | 0 | 0 (0) | 0 | 0 | 0 | 4 | [-72, 384] | 0 | 0 | 0 | 2 | 2 | FAIL | 1: line specials outside the admitted set |
| E2M8 | 7:4 75:4 86:4 | 12 | 12 | 0 | 0 | 8 | door 4, floor 4 | 0 | 0 | 0 | 0 (0) | 0 | 0 | 0 | 6 | [0, 280] | 0 | 0 | 0 | 0 | 0 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| E2M9 | 31:3 32:1 33:1 34:1 52:1 103:1 | 1 | 1 | 0 | 0 | 6 | door 6 | 0 | 0 | 0 | 0 (0) | 0 | 0 | 0 | 4 | [-48, 168] | 0 | 0 | 0 | 0 | 0 | FAIL | 1: line specials outside the admitted set |
| E3M1 | 18:1 31:2 37:6 42:1 52:4 61:1 62:4 | 13 | 13 | 0 | 0 | 15 | door 6, floor 7, plat 2 | 0 | 2 | 6 | 0 (0) | 7 | 0 | 0 | 5 | [-128, 192] | 0 | 0 | 0 | 2 | 2 | FAIL | 1: line specials outside the admitted set |
| E3M2 | 2:6 3:1 23:1 30:1 31:1 32:1 46:6 52:4 103:4 | 26 | 46 | 0 | 0 | 22 | door 10, floor 12 | 0 | 0 | 6 | 0 (0) | 3 | 0 | 0 | 5 | [0, 272] | 0 | 0 | 0 | 4 | 0 | FAIL | 1: line specials outside the admitted set |
| E3M3 | 7:1 18:3 19:2 26:2 52:3 62:2 102:1 | 11 | 13 | 0 | 0 | 12 | door 3, floor 6, plat 3 | 0 | 3 | 2 | 63 (1) | 6 | 0 | 0 | 5 | [-120, 360] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; two-sided scroller (Stage A) |
| E3M4 | 2:4 18:3 27:1 31:14 32:2 33:1 42:4 52:1 61:4 73:8 74:4 75:1 86:1 97:6 102:1 103:3 | 39 | 46 | 0 | 0 | 39 | ceiling 2, door 35, floor 4 | 2 | 0 | 8 | 0 (0) | 0 | 0 | 0 | 6 | [-16, 472] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| E3M5 | 2:11 23:6 31:3 32:2 34:1 40:4 44:1 52:3 61:1 73:1 74:9 82:2 90:1 97:26 102:2 103:2 | 70 | 1528 | 6 | 4 | 264 | ceiling 13, door 47, floor 243 | 39 | 0 | 1 | 0 (0) | 10 | 0 | 0 | 13 | [-48, 384] | 0 | 0 | 0 | 2 | 2 | FAIL | 1: line specials outside the admitted set; 1: one-sided DR line; 6: spechit window |
| E3M6 | 19:4 31:3 32:1 37:1 39:2 51:1 52:1 56:4 61:1 75:1 82:3 97:8 102:1 103:2 | 35 | 42 | 0 | 0 | 23 | door 10, floor 13 | 0 | 0 | 3 | 5 (5) | 2 | 0 | 0 | 6 | [-128, 256] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; two-sided scroller (Stage A) |
| E3M7 | 2:3 20:5 26:2 31:7 33:7 34:2 37:11 52:1 62:1 97:18 103:2 | 41 | 55 | 0 | 0 | 26 | door 19, floor 2, plat 8 | 3 | 8 | 1 | 0 (0) | 0 | 0 | 0 | 5 | [-96, 256] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; 2: shared mover families |
| E3M8 | 2:4 31:6 | 4 | 16 | 0 | 0 | 5 | door 5 | 0 | 0 | 0 | 0 (0) | 0 | 0 | 0 | 4 | [0, 256] | 0 | 0 | 0 | 0 | 0 | FAIL | 1: line specials outside the admitted set |
| E3M9 | 18:1 23:1 31:2 32:1 33:1 37:6 39:1 42:1 52:1 61:1 62:4 104:1 | 22 | 51 | 0 | 0 | 28 | door 7, floor 19, plat 2 | 0 | 2 | 6 | 0 (0) | 15 | 0 | 0 | 5 | [-136, 184] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| E4M1 | 19:1 23:1 31:1 32:1 33:1 52:1 56:1 60:2 97:4 102:2 117:2 123:6 126:9 | 26 | 27 | 0 | 0 | 11 | door 4, floor 5, plat 2 | 0 | 2 | 8 | 24 (24) | 1 | 0 | 0 | 25 | [-64, 152] | 0 | 0 | 0 | 0 | 0 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; two-sided scroller (Stage A); 6: spechit window |
| E4M2 | 2:1 20:2 31:1 51:1 62:3 97:8 105:1 112:2 117:3 118:1 133:2 137:1 | 24 | 31 | 0 | 0 | 18 | door 12, floor 2, plat 4 | 0 | 4 | 3 | 0 (0) | 1 | 0 | 0 | 6 | [-128, 440] | 0 | 0 | 0 | 8 | 0 | FAIL | 1: line specials outside the admitted set |
| E4M3 | 2:1 20:1 28:2 31:4 32:2 45:1 47:1 52:4 62:4 65:1 71:1 83:1 97:10 114:4 120:2 123:11 135:8 | 49 | 58 | 0 | 0 | 18 | door 8, floor 3, plat 7 | 0 | 7 | 21 | 0 (0) | 6 | 0 | 0 | 8 | [-216, 304] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| E4M4 | 19:4 31:6 33:2 52:3 62:4 97:7 102:2 114:1 123:1 | 20 | 57 | 0 | 0 | 32 | door 5, floor 20, plat 7 | 0 | 7 | 6 | 0 (0) | 2 | 0 | 0 | 4 | [-24, 224] | 0 | 0 | 0 | 1 | 0 | FAIL | 1: line specials outside the admitted set |
| E4M5 | 26:2 27:2 28:6 62:20 97:13 | 43 | 37 | 0 | 0 | 19 | door 12, plat 7 | 0 | 7 | 20 | 0 (0) | 0 | 0 | 0 | 10 | [-448, 256] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| E4M6 | 2:1 22:3 23:3 31:1 97:14 112:1 117:7 118:2 123:5 133:1 134:2 136:2 | 42 | 78 | 0 | 0 | 48 | door 19, floor 23, plat 6 | 0 | 6 | 9 | 0 (0) | 6 | 0 | 0 | 8 | [-304, 256] | 0 | 0 | 0 | 8 | 0 | FAIL | 1: line specials outside the admitted set |
| E4M7 | 5:5 20:1 22:1 23:2 25:1 26:2 27:12 28:2 30:2 59:1 62:6 73:2 74:2 97:17 101:6 103:2 114:7 115:8 120:1 123:1 125:1 135:3 | 74 | 106 | 0 | 0 | 47 | ceiling 4, door 17, floor 19, plat 7 | 0 | 7 | 22 | 24 (24) | 2 | 0 | 0 | 11 | [-256, 264] | 0 | 0 | 0 | 8 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; two-sided scroller (Stage A); 6: spechit window |
| E4M8 | 20:1 33:1 34:1 52:8 97:11 117:10 118:2 123:135 127:1 | 149 | 224 | 0 | 0 | 34 | door 11, floor 1, plat 22 | 0 | 22 | 136 | 0 (0) | 20 | 0 | 0 | 22 | [-336, 304] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| E4M9 | 27:4 62:35 97:30 102:2 | 75 | 83 | 0 | 0 | 31 | door 17, floor 2, plat 12 | 0 | 12 | 35 | 0 (0) | 2 | 0 | 0 | 10 | [-64, 384] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| MAP01 | 2:1 31:2 62:1 102:1 103:2 | 5 | 8 | 0 | 0 | 12 | door 7, floor 4, plat 1 | 0 | 1 | 1 | 0 (0) | 0 | 0 | 0 | 2 | [-40, 264] | 0 | 0 | 0 | 2 | 2 | FAIL | 1: line specials outside the admitted set |
| MAP02 | 32:3 97:1 102:2 114:6 117:2 135:16 | 28 | 77 | 0 | 0 | 16 | door 14, floor 2 | 0 | 0 | 6 | 0 (0) | 0 | 0 | 0 | 17 | [-96, 176] | 0 | 0 | 0 | 8 | 0 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 6: spechit window |
| MAP03 | 19:9 20:1 26:8 33:1 97:12 103:1 114:6 120:3 123:2 | 36 | 41 | 0 | 0 | 19 | door 13, floor 3, plat 3 | 0 | 3 | 8 | 0 (0) | 4 | 0 | 0 | 5 | [-144, 264] | 0 | 0 | 0 | 8 | 0 | FAIL | 1: line specials outside the admitted set |
| MAP04 | 18:1 97:5 99:2 102:1 114:13 134:4 136:2 138:1 139:1 141:1 | 40 | 79 | 0 | 0 | 29 | ceiling 2, door 24, floor 2, plat 1 | 0 | 1 | 23 | 0 (0) | 0 | 0 | 0 | 6 | [-120, 168] | 0 | 0 | 0 | 8 | 4 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| MAP05 | 3:1 22:2 28:4 32:2 34:2 62:1 86:1 97:2 103:3 106:2 114:6 115:7 117:2 120:18 123:5 | 59 | 78 | 0 | 0 | 36 | door 25, plat 11 | 0 | 11 | 19 | 0 (0) | 0 | 0 | 0 | 8 | [80, 320] | 0 | 0 | 0 | 8 | 0 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| MAP06 | 2:2 19:1 20:1 23:1 32:2 33:2 49:1 61:1 62:2 74:1 97:24 114:6 115:1 117:2 120:1 123:6 136:2 | 57 | 80 | 0 | 2 | 30 | ceiling 1, door 21, floor 2, plat 6 | 0 | 6 | 18 | 0 (0) | 0 | 0 | 0 | 5 | [-568, 328] | 0 | 0 | 0 | 8 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| MAP07 | 23:1 31:1 71:4 | 6 | 10 | 0 | 0 | 10 | door 1, floor 10 | 1 | 0 | 0 | 0 (0) | 4 | 0 | 0 | 4 | [0, 128] | 0 | 0 | 0 | 1 | 0 | FAIL | 1: line specials outside the admitted set |
| MAP08 | 2:2 3:1 8:1 20:1 31:2 33:1 34:1 37:6 46:4 61:3 62:2 97:10 103:3 125:4 | 41 | 48 | 0 | 0 | 34 | door 20, floor 14, plat 2 | 2 | 2 | 9 | 8 (0) | 0 | 0 | 0 | 5 | [-80, 224] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set |
| MAP09 | 2:5 3:1 15:1 18:1 22:1 23:1 31:3 32:1 34:4 61:2 62:2 86:1 97:14 100:1 102:1 103:2 112:1 120:9 123:14 | 73 | 102 | 0 | 0 | 47 | door 23, floor 5, plat 19 | 0 | 19 | 22 | 0 (0) | 0 | 0 | 0 | 5 | [-304, 448] | 0 | 0 | 0 | 8 | 4 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| MAP10 | 2:4 4:1 27:1 32:2 46:5 62:3 71:4 90:6 97:1 114:1 126:4 | 31 | 38 | 0 | 0 | 34 | door 30, floor 1, plat 3 | 0 | 3 | 10 | 0 (0) | 0 | 0 | 0 | 5 | [-72, 304] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set |
| MAP11 | 20:2 26:2 28:2 62:1 97:8 102:2 105:2 117:2 120:5 123:3 | 25 | 39 | 0 | 0 | 27 | door 11, floor 7, plat 9 | 0 | 9 | 4 | 4 (4) | 4 | 0 | 0 | 6 | [-232, 544] | 0 | 0 | 0 | 8 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512; two-sided scroller (Stage A) |
| MAP12 | 2:2 18:2 22:2 26:1 27:1 32:1 33:1 37:4 39:1 52:4 59:7 61:2 62:8 71:1 87:1 89:1 97:6 103:6 119:8 | 55 | 80 | 0 | 0 | 49 | door 14, floor 22, plat 17 | 4 | 17 | 10 | 0 (0) | 2 | 0 | 0 | 7 | [-80, 296] | 0 | 0 | 0 | 4 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP13 | 2:1 3:1 7:1 19:1 23:1 26:1 31:6 32:1 33:1 34:1 52:4 62:2 73:3 74:6 77:3 83:1 87:1 89:1 97:20 102:2 103:1 | 52 | 84 | 0 | 0 | 35 | ceiling 2, door 15, floor 16, plat 3 | 1 | 3 | 2 | 8 (8) | 2 | 0 | 0 | 12 | [-72, 640] | 0 | 0 | 0 | 4 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512; two-sided scroller (Stage A); 6: spechit window |
| MAP14 | 26:2 33:2 42:1 60:1 61:1 62:1 73:2 74:1 105:11 114:7 117:8 120:3 123:75 | 105 | 194 | 0 | 0 | 58 | ceiling 1, door 19, floor 17, plat 21 | 0 | 21 | 88 | 0 (0) | 32 | 0 | 0 | 14 | [64, 296] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| MAP15 | 2:4 13:1 23:1 27:8 30:1 31:1 32:6 33:2 35:3 52:4 62:7 71:5 97:15 103:2 105:3 106:1 117:1 119:1 120:7 123:9 124:4 127:2 137:6 | 73 | 112 | 0 | 0 | 49 | door 25, floor 7, plat 19 | 2 | 19 | 16 | 0 (0) | 6 | 0 | 0 | 6 | [-128, 600] | 0 | 0 | 0 | 8 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512 |
| MAP16 | 31:5 32:1 33:2 46:2 52:2 62:7 97:1 103:1 106:3 123:12 126:12 | 38 | 54 | 0 | 0 | 25 | door 18, plat 8 | 1 | 8 | 21 | 0 (0) | 2 | 0 | 0 | 10 | [-24, 264] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| MAP17 | 2:1 16:1 19:1 20:2 30:1 31:4 32:1 33:2 34:1 52:1 62:6 97:13 102:2 103:3 105:1 112:1 117:6 118:1 120:5 123:7 | 64 | 80 | 0 | 0 | 51 | door 30, floor 8, plat 13 | 0 | 13 | 14 | 20 (20) | 8 | 0 | 0 | 12 | [-232, 728] | 0 | 0 | 0 | 8 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512; two-sided scroller (Stage A); 6: spechit window |
| MAP18 | 3:1 5:2 7:1 19:7 23:1 32:1 34:1 35:1 45:2 46:2 52:1 62:1 97:13 102:3 103:1 | 39 | 42 | 0 | 0 | 27 | door 10, floor 16, plat 1 | 0 | 1 | 5 | 0 (0) | 5 | 0 | 0 | 5 | [-96, 352] | 0 | 0 | 0 | 4 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP19 | 7:1 18:1 19:2 23:1 31:4 32:1 33:1 34:1 39:1 47:2 52:4 61:1 62:6 67:1 68:1 71:1 73:1 74:5 87:2 89:3 90:1 97:23 102:2 103:1 | 62 | 72 | 0 | 0 | 48 | ceiling 1, door 28, floor 11, plat 8 | 0 | 8 | 13 | 29 (7) | 6 | 0 | 0 | 6 | [-64, 368] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; two-sided scroller (Stage A) |
| MAP20 | 2:4 18:1 22:5 31:1 46:3 52:4 62:8 70:1 83:6 90:1 91:1 94:1 97:33 103:1 117:2 126:1 | 90 | 96 | 0 | 0 | 33 | door 14, floor 10, plat 9 | 0 | 9 | 12 | 0 (0) | 8 | 0 | 0 | 10 | [-280, 648] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512; 6: spechit window |
| MAP21 | 23:1 32:10 33:7 34:6 52:2 62:41 71:4 97:36 103:1 127:4 | 93 | 94 | 0 | 0 | 22 | door 9, floor 4, plat 10 | 1 | 10 | 41 | 0 (0) | 1 | 0 | 0 | 16 | [0, 256] | 0 | 0 | 0 | 8 | 0 | FAIL | 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| MAP22 | 32:1 33:2 62:6 68:1 97:4 114:5 117:6 118:2 123:8 | 25 | 41 | 0 | 0 | 21 | door 13, plat 8 | 0 | 8 | 20 | 0 (0) | 5 | 0 | 0 | 4 | [-240, 256] | 0 | 0 | 0 | 8 | 0 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS |
| MAP23 | 2:3 3:1 16:1 18:1 31:2 34:1 46:1 52:4 61:1 62:2 71:1 97:26 118:1 | 43 | 44 | 0 | 0 | 13 | door 11, floor 2, plat 1 | 1 | 1 | 4 | 0 (0) | 0 | 0 | 0 | 6 | [-128, 320] | 0 | 0 | 0 | 8 | 0 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP24 | 2:2 5:1 7:2 19:1 20:1 22:6 23:3 28:2 32:2 35:1 52:5 62:7 67:8 97:63 102:1 103:1 | 105 | 116 | 0 | 0 | 37 | door 13, floor 10, plat 14 | 0 | 14 | 15 | 0 (0) | 0 | 0 | 0 | 6 | [-512, 384] | 0 | 0 | 0 | 4 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP25 | 32:2 52:4 60:1 62:8 97:10 105:2 117:4 120:3 123:5 | 31 | 35 | 0 | 0 | 10 | door 4, floor 1, plat 5 | 0 | 5 | 16 | 0 (0) | 3 | 0 | 0 | 8 | [-120, 368] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP26 | 2:2 22:2 32:2 33:2 34:4 35:1 37:1 52:4 62:4 90:9 97:32 103:1 105:4 114:2 115:2 116:1 117:11 118:4 119:1 123:1 | 75 | 93 | 0 | 0 | 27 | door 21, floor 2, plat 7 | 3 | 7 | 11 | 0 (0) | 0 | 0 | 0 | 7 | [-312, 296] | 0 | 0 | 0 | 8 | 4 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP27 | 2:22 7:1 23:1 27:2 28:1 31:1 32:1 35:6 39:1 46:2 52:1 60:2 61:2 62:10 71:2 97:6 103:1 110:1 112:2 114:1 115:1 117:1 120:6 | 123 | 151 | 0 | 0 | 54 | door 29, floor 16, plat 9 | 0 | 9 | 18 | 0 (0) | 1 | 0 | 0 | 9 | [-144, 256] | 0 | 0 | 0 | 8 | 4 | FAIL | geometry (M3h6); sector specials; 1: line specials outside the admitted set; 5: MAXPLATS/MAXBUTTONS/MAXLINEANIMS; 6: spechit window |
| MAP28 | 2:8 3:1 31:5 33:1 34:2 35:2 46:3 52:7 61:1 62:5 70:1 97:1 119:2 138:1 139:1 | 29 | 44 | 0 | 0 | 34 | door 19, floor 13, plat 3 | 1 | 3 | 12 | 33 (26) | 0 | 0 | 0 | 7 | [-232, 280] | 0 | 0 | 0 | 4 | 4 | FAIL | 1: line specials outside the admitted set; two-sided scroller (Stage A) |
| MAP29 | 2:1 18:1 20:4 52:4 71:2 86:1 97:32 102:5 103:1 119:1 123:15 126:3 131:2 | 73 | 78 | 0 | 0 | 28 | door 9, floor 12, plat 8 | 1 | 8 | 15 | 0 (0) | 0 | 0 | 0 | 8 | [104, 968] | 0 | 0 | 0 | 8 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512 |
| MAP30 | 62:9 97:1 140:1 | 11 | 11 | 0 | 0 | 4 | floor 1, plat 4 | 1 | 4 | 9 | 1 (0) | 0 | 0 | 0 | 4 | [-128, 640] | 0 | 0 | 0 | 0 | 0 | FAIL | geometry (M3h6); 1: line specials outside the admitted set; 4: reachable height beyond ±512 |
| MAP31 | 31:6 51:1 | 0 | 0 | 0 | 0 | 28 | door 28 | 0 | 0 | 0 | 0 (0) | 0 | 0 | 0 | 4 | [-24, 184] | 0 | 0 | 0 | 2 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |
| MAP32 | 31:2 | 1 | 1 | 0 | 0 | 9 | door 9 | 0 | 0 | 0 | 0 (0) | 0 | 0 | 0 | 3 | [-24, 256] | 0 | 0 | 0 | 8 | 2 | FAIL | geometry (M3h6); 1: line specials outside the admitted set |

- stage B: maps using only line specials in {0, 48, 1, 63, 109, 88, 36, 38, 11}: 1 of 68 (E1M1)
- stage B: geometry admitted (M3h6): 1 of 1 admitted-set maps, 33 of 68 maps
- stage B: sector specials in the Stage A set: 1 of 1 admitted-set maps, 66 of 68 maps
- stage B: rule 1 every DR line two-sided: 1 of 1 admitted-set maps, 67 of 68 maps
- stage B: rule 2 mover families disjoint: 1 of 1 admitted-set maps, 67 of 68 maps
- stage B: rule 3 every mover sector has a two-sided neighbor: 1 of 1 admitted-set maps, 68 of 68 maps
- stage B: rule 4 reachable heights within ±512: 1 of 1 admitted-set maps, 58 of 68 maps
- stage B: rule 4 no inverting door: 1 of 1 admitted-set maps, 68 of 68 maps
- stage B: rule 5 plats, buttons, scrollers: 1 of 1 admitted-set maps, 53 of 68 maps
- stage B: scrollers one-sided (Stage A): 0 of 1 admitted-set maps, 56 of 68 maps
- stage B: rule 6 spechit window ≤ 8: 1 of 1 admitted-set maps, 51 of 68 maps
- stage B: every Stage B load rule: 0 of 1 admitted-set maps, 0 of 68 maps
- stage B: admitted-set reachable range over all maps [-568, 968]; within ±512 for 58 of 68; diverged 0
- stage B: inverting doors (admitted set) 0 in all, none for 68 of 68; maximum turbo jump-up 0; maximum door top drop 0; maximum one-tic plane step 8; maximum B2 Δ 4
- stage B: every special: maxima mover sectors 264, plat sectors 22, tag-using lines 149, targets of one line 228; plat sectors ≤ 30 for 68 of 68
- stage B: every special: sectors in more than one mover family 70 in all; none for 49 of 68
- stage B: every special: DR back sectors that are non-door targets 35 in all; none for 63 of 68
- stage B: every special: self-referencing lines of door sectors 0 in all; none for 68 of 68
- stage B: every special: door sectors without a two-sided neighbor 0 in all; none for 68 of 68
- stage B: every special: tag-0 lines with tag-using specials 6 in all; none for 67 of 68
- stage B: every special: tag-using lines with no target sector 10 in all; none for 63 of 68
- stage B: every special: sky mover sectors 190 in all; none for 32 of 68
- stage B: every special: one-sided manual door lines by type 1:4 117:2
- stage B: 2x2 window two-sided special lines: maximum 25; ≤ 8 for 51 of 68

## M3h7 §5.6 specials table

The pure-count table of the M3h7 note (§5.6), as printed by `--specials-markdown`. Its
Stage B columns use E1M1's seven types only.

| Slot | Line special types | Tagged lines | Sector special types | Tagged sectors | Sector specials A | Line specials A | Scrollers | Two-sided scrollers | Scrollers ≤ 64 | Light sectors | Sky ceilings | Sky floors | Stage A all | Mover sectors | Plat sectors | Reusable switches | One-sided DR | Multi-family sectors | Self-ref door sectors | Tag-0 tag-using lines | Sky mover sectors | 2x2 two-sided specials |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1M1 | 1:8 11:1 36:1 38:1 48:8 63:1 88:1 109:1 | 5 | 1:1 7:4 8:2 9:3 12:1 | 5 | pass | FAIL | 8 | 8 | pass | 4 | 7 | 0 | FAIL | 9 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 8 |
| E1M2 | 1:15 9:1 11:1 28:2 31:2 46:1 62:6 88:11 103:3 | 22 | 1:3 2:1 3:5 7:5 8:13 9:6 12:1 13:3 | 13 | pass | FAIL | 0 | 0 | pass | 26 | 8 | 0 | FAIL | 13 | 5 | 7 | 0 | 0 | 0 | 0 | 0 | 4 |
| E1M3 | 1:12 2:9 8:1 11:1 20:2 26:2 27:2 31:4 35:3 51:1 62:4 63:1 88:7 90:2 103:1 | 30 | 1:8 7:8 8:4 9:7 16:5 | 28 | pass | FAIL | 0 | 0 | pass | 12 | 10 | 0 | FAIL | 12 | 5 | 5 | 0 | 0 | 0 | 0 | 1 | 6 |
| E1M4 | 1:6 2:1 5:1 11:1 18:2 26:2 27:2 31:2 36:2 62:3 86:4 88:3 90:12 | 30 | 1:4 2:1 7:8 8:4 9:3 12:1 13:2 | 12 | pass | FAIL | 0 | 0 | pass | 12 | 1 | 0 | FAIL | 7 | 3 | 3 | 0 | 0 | 0 | 0 | 0 | 6 |
| E1M5 | 1:22 2:2 11:1 22:1 26:3 27:2 31:6 62:3 70:2 88:5 90:1 91:10 97:10 98:3 103:2 | 39 | 1:2 2:4 7:7 8:8 9:9 | 12 | pass | FAIL | 0 | 0 | pass | 14 | 9 | 0 | FAIL | 14 | 3 | 5 | 0 | 0 | 0 | 0 | 0 | 14 |
| E1M6 | 1:12 2:8 11:1 16:1 26:3 27:6 28:4 36:1 62:4 76:2 88:8 103:3 | 27 | 1:5 2:1 3:1 7:30 8:2 9:4 12:3 | 20 | pass | FAIL | 0 | 0 | pass | 12 | 17 | 0 | FAIL | 12 | 3 | 4 | 0 | 0 | 0 | 0 | 2 | 6 |
| E1M7 | 1:7 2:3 11:1 22:1 23:1 26:2 27:6 28:2 31:3 34:3 48:4 62:7 63:2 70:1 88:12 90:3 91:7 98:1 103:1 | 39 | 1:2 2:1 3:3 7:8 8:7 9:4 12:3 | 16 | pass | FAIL | 4 | 4 | pass | 16 | 6 | 0 | FAIL | 11 | 5 | 10 | 0 | 0 | 0 | 0 | 0 | 8 |
| E1M8 | 1:2 2:3 7:1 23:1 31:4 62:2 82:1 91:1 97:8 | 17 | 1:4 7:1 8:4 9:1 11:1 | 17 | FAIL | FAIL | 0 | 0 | pass | 8 | 29 | 0 | FAIL | 1 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 8 |
| E1M9 | 1:3 2:17 11:1 20:1 31:2 32:2 33:4 34:4 36:1 62:6 88:11 90:2 97:2 103:1 | 41 | 7:10 8:4 9:2 16:1 | 17 | pass | FAIL | 0 | 0 | pass | 4 | 37 | 0 | FAIL | 9 | 4 | 6 | 0 | 0 | 0 | 0 | 4 | 9 |
| E2M1 | 1:2 7:1 11:1 14:1 26:2 28:2 29:1 36:1 37:6 63:1 97:20 102:3 103:1 | 35 | 5:10 7:3 8:1 9:4 | 24 | pass | FAIL | 0 | 0 | pass | 1 | 3 | 0 | FAIL | 3 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 5 |
| E2M2 | 1:10 2:6 8:1 9:1 10:1 11:1 13:4 16:1 19:3 20:1 21:1 23:4 26:2 27:2 30:1 31:2 32:1 33:2 38:1 59:5 73:5 74:3 77:2 87:3 89:5 102:2 103:4 | 54 | 1:8 2:3 4:3 5:3 7:5 9:12 16:6 | 44 | pass | FAIL | 0 | 0 | pass | 14 | 0 | 0 | FAIL | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5 |
| E2M3 | 1:10 11:1 26:2 48:12 87:3 89:5 | 8 | 1:14 2:1 3:2 5:1 7:9 8:2 9:6 | 12 | pass | FAIL | 12 | 12 | pass | 19 | 4 | 0 | FAIL | 6 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 10 |
| E2M4 | 1:10 2:7 3:1 7:3 11:1 19:2 20:1 22:1 23:1 24:1 31:7 32:1 34:1 46:6 48:1 56:1 58:5 62:3 73:2 74:7 77:4 88:4 97:10 102:1 | 63 | 1:6 3:1 5:1 7:11 8:1 9:10 12:3 13:1 16:3 | 43 | pass | FAIL | 1 | 0 | pass | 12 | 0 | 0 | FAIL | 9 | 3 | 9 | 0 | 0 | 0 | 0 | 0 | 6 |
| E2M5 | 1:24 2:10 7:1 11:1 20:1 31:1 36:1 51:1 62:2 63:32 97:14 103:1 104:1 | 66 | 1:15 2:1 5:1 7:8 8:1 9:10 | 36 | pass | FAIL | 0 | 0 | pass | 17 | 7 | 0 | FAIL | 32 | 0 | 34 | 0 | 0 | 0 | 0 | 0 | 6 |
| E2M6 | 1:24 2:20 11:1 20:1 23:1 27:5 31:2 32:1 33:1 37:1 48:6 61:1 73:1 74:1 103:4 | 35 | 1:4 2:2 3:7 7:5 9:3 12:2 | 28 | pass | FAIL | 6 | 0 | pass | 15 | 6 | 0 | FAIL | 13 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 6 |
| E2M7 | 1:18 11:1 16:1 20:1 32:2 33:2 34:2 97:3 102:3 103:6 65535:1 | 14 | 1:5 2:1 3:5 4:2 5:2 7:10 8:2 9:6 13:3 16:3 | 14 | pass | FAIL | 0 | 0 | pass | 18 | 4 | 0 | FAIL | 9 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| E2M8 | 7:4 75:4 86:4 | 12 | none | 12 | pass | FAIL | 0 | 0 | pass | 0 | 1 | 0 | FAIL | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 6 |
| E2M9 | 31:3 32:1 33:1 34:1 52:1 103:1 | 1 | 8:2 9:1 16:1 | 1 | pass | FAIL | 0 | 0 | pass | 2 | 0 | 0 | FAIL | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| E3M1 | 1:4 18:1 31:2 37:6 42:1 52:4 61:1 62:4 | 13 | 1:1 5:2 9:1 | 10 | pass | FAIL | 0 | 0 | pass | 1 | 19 | 0 | FAIL | 3 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 5 |
| E3M2 | 2:6 3:1 23:1 30:1 31:1 32:1 36:1 38:6 46:6 52:4 103:4 | 26 | 8:1 9:3 16:1 | 20 | pass | FAIL | 0 | 0 | pass | 1 | 22 | 0 | FAIL | 10 | 0 | 6 | 0 | 0 | 0 | 0 | 2 | 5 |
| E3M3 | 1:4 7:1 18:3 19:2 26:2 48:63 52:3 62:2 88:2 102:1 | 12 | 3:1 5:3 7:2 8:6 9:6 12:3 16:6 | 11 | pass | FAIL | 63 | 1 | pass | 10 | 31 | 0 | FAIL | 5 | 3 | 2 | 0 | 0 | 0 | 0 | 2 | 5 |
| E3M4 | 1:26 2:4 18:3 27:1 31:14 32:2 33:1 42:4 52:1 61:4 73:8 74:4 75:1 86:1 97:6 102:1 103:3 | 40 | 5:5 7:5 9:4 16:1 | 22 | pass | FAIL | 0 | 0 | pass | 0 | 3 | 0 | FAIL | 17 | 0 | 8 | 0 | 0 | 0 | 0 | 0 | 6 |
| E3M5 | 1:49 2:11 23:6 31:3 32:2 34:1 38:4 40:4 44:1 52:3 61:1 73:1 74:9 82:2 90:1 97:26 102:2 103:2 | 64 | 7:3 8:4 9:10 16:3 | 40 | pass | FAIL | 0 | 0 | pass | 4 | 10 | 0 | FAIL | 36 | 0 | 1 | 4 | 0 | 0 | 0 | 0 | 13 |
| E3M6 | 1:4 19:4 31:3 32:1 37:1 38:6 39:2 48:5 51:1 52:1 56:4 61:1 63:2 75:1 82:3 97:8 102:1 103:2 | 35 | 7:5 9:4 12:2 16:20 | 25 | pass | FAIL | 5 | 5 | pass | 2 | 32 | 0 | FAIL | 8 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 6 |
| E3M7 | 1:4 2:3 20:5 26:2 31:7 33:7 34:2 37:11 52:1 62:1 88:1 97:18 103:2 | 41 | 2:1 5:32 8:1 9:4 | 35 | pass | FAIL | 0 | 0 | pass | 2 | 0 | 0 | FAIL | 4 | 3 | 1 | 0 | 2 | 0 | 0 | 0 | 5 |
| E3M8 | 2:4 31:6 | 4 | none | 5 | pass | FAIL | 0 | 0 | pass | 0 | 4 | 0 | FAIL | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| E3M9 | 1:5 18:1 23:1 31:2 32:1 33:1 37:6 38:6 39:1 42:1 52:1 61:1 62:4 104:1 | 22 | 1:2 5:6 8:1 9:1 | 24 | pass | FAIL | 0 | 0 | pass | 3 | 35 | 0 | FAIL | 13 | 0 | 6 | 0 | 0 | 0 | 0 | 6 | 5 |
| E4M1 | 19:1 23:1 31:1 32:1 33:1 48:24 52:1 56:1 60:2 97:4 102:2 117:2 123:6 126:9 | 26 | 1:1 7:6 9:2 17:12 | 14 | pass | FAIL | 24 | 24 | pass | 13 | 17 | 0 | FAIL | 0 | 0 | 8 | 0 | 0 | 0 | 0 | 0 | 25 |
| E4M2 | 2:1 11:1 20:2 31:1 38:2 51:1 62:3 97:8 105:1 109:2 112:2 117:3 118:1 133:2 137:1 | 24 | 2:1 5:16 8:8 9:3 13:2 17:4 | 18 | pass | FAIL | 0 | 0 | pass | 15 | 3 | 0 | FAIL | 5 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 6 |
| E4M3 | 2:1 20:1 28:2 31:4 32:2 45:1 47:1 52:4 62:4 65:1 71:1 83:1 88:3 97:10 114:4 120:2 123:11 135:8 | 56 | 5:3 8:4 9:22 | 18 | pass | FAIL | 0 | 0 | pass | 4 | 42 | 0 | FAIL | 2 | 2 | 21 | 0 | 0 | 0 | 0 | 2 | 8 |
| E4M4 | 19:4 31:6 33:2 38:1 52:3 62:4 97:7 102:2 114:1 123:1 | 20 | 1:1 2:5 5:4 9:2 16:6 | 41 | pass | FAIL | 0 | 0 | pass | 6 | 12 | 1 | FAIL | 2 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 4 |
| E4M5 | 1:16 11:1 26:2 27:2 28:6 62:20 88:10 97:13 | 43 | 5:10 7:8 9:2 17:2 | 11 | pass | FAIL | 0 | 0 | pass | 2 | 3 | 0 | FAIL | 13 | 6 | 20 | 0 | 0 | 0 | 0 | 0 | 10 |
| E4M6 | 2:1 11:1 22:3 23:3 31:1 38:3 97:14 109:7 112:1 117:7 118:2 123:5 133:1 134:2 136:2 | 42 | 5:14 8:4 9:3 17:10 | 51 | pass | FAIL | 0 | 0 | pass | 14 | 53 | 0 | FAIL | 3 | 0 | 9 | 0 | 0 | 0 | 0 | 0 | 8 |
| E4M7 | 1:2 5:5 11:1 20:1 22:1 23:2 25:1 26:2 27:12 28:2 30:2 48:24 59:1 62:6 73:2 74:2 88:4 97:17 101:6 103:2 109:1 114:7 115:8 120:1 123:1 125:1 135:3 | 74 | 1:4 2:1 5:4 7:17 8:5 9:4 17:1 | 51 | pass | FAIL | 24 | 24 | pass | 11 | 25 | 0 | FAIL | 4 | 2 | 22 | 0 | 0 | 0 | 0 | 0 | 11 |
| E4M8 | 1:2 20:1 33:1 34:1 52:8 63:1 97:11 117:10 118:2 123:135 127:1 | 149 | 5:1 8:4 9:1 16:2 17:15 | 69 | pass | FAIL | 0 | 0 | pass | 19 | 71 | 1 | FAIL | 2 | 0 | 136 | 0 | 0 | 0 | 0 | 0 | 22 |
| E4M9 | 1:30 11:1 27:4 62:35 88:8 97:30 102:2 | 75 | 8:1 9:2 | 19 | pass | FAIL | 0 | 0 | pass | 1 | 48 | 0 | FAIL | 23 | 8 | 35 | 0 | 0 | 0 | 0 | 2 | 10 |
| MAP01 | 1:4 2:1 11:1 31:2 62:1 102:1 103:2 | 5 | 9:5 | 8 | pass | FAIL | 0 | 0 | pass | 0 | 10 | 0 | FAIL | 2 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 2 |
| MAP02 | 11:1 32:3 97:1 102:2 109:3 114:6 117:2 135:16 | 33 | 3:2 9:1 | 17 | pass | FAIL | 0 | 0 | pass | 2 | 1 | 0 | FAIL | 4 | 0 | 6 | 0 | 0 | 0 | 0 | 0 | 17 |
| MAP03 | 11:1 19:9 20:1 26:8 33:1 97:12 103:1 109:2 114:6 120:3 123:2 | 36 | 2:1 9:1 | 23 | pass | FAIL | 0 | 0 | pass | 1 | 31 | 0 | FAIL | 4 | 0 | 8 | 0 | 0 | 0 | 0 | 0 | 5 |
| MAP04 | 11:1 18:1 88:2 97:5 99:2 102:1 109:7 114:13 134:4 136:2 138:1 139:1 141:1 | 40 | 8:4 9:3 16:3 | 35 | pass | FAIL | 0 | 0 | pass | 4 | 0 | 0 | FAIL | 14 | 1 | 23 | 0 | 0 | 0 | 0 | 0 | 6 |
| MAP05 | 3:1 11:1 22:2 28:4 32:2 34:2 62:1 86:1 97:2 103:3 106:2 109:11 114:6 115:7 117:2 120:18 123:5 | 69 | 2:14 9:3 | 37 | pass | FAIL | 0 | 0 | pass | 14 | 6 | 0 | FAIL | 3 | 0 | 19 | 0 | 0 | 0 | 0 | 0 | 8 |
| MAP06 | 2:2 11:1 19:1 20:1 23:1 32:2 33:2 49:1 61:1 62:2 74:1 88:1 97:24 109:6 114:6 115:1 117:2 120:1 123:6 136:2 | 60 | 1:1 7:3 8:8 9:3 16:3 | 38 | pass | FAIL | 0 | 0 | pass | 9 | 7 | 0 | FAIL | 12 | 1 | 18 | 0 | 0 | 0 | 0 | 0 | 5 |
| MAP07 | 11:5 23:1 31:1 38:1 71:4 | 6 | 9:1 | 11 | pass | FAIL | 0 | 0 | pass | 0 | 17 | 0 | FAIL | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| MAP08 | 1:8 2:2 3:1 8:1 11:1 20:1 31:2 33:1 34:1 37:6 38:3 46:4 48:8 61:3 62:2 88:1 97:10 103:3 125:4 | 53 | 1:12 5:5 8:2 9:7 16:2 | 41 | pass | FAIL | 8 | 0 | pass | 14 | 1 | 0 | FAIL | 13 | 1 | 9 | 0 | 0 | 0 | 0 | 0 | 5 |
| MAP09 | 1:6 2:5 3:1 11:1 15:1 18:1 22:1 23:1 31:3 32:1 34:4 38:1 61:2 62:2 63:4 86:1 88:6 97:14 100:1 102:1 103:2 109:5 112:1 120:9 123:14 | 74 | 1:2 2:3 5:2 7:3 9:6 16:7 | 43 | pass | FAIL | 0 | 0 | pass | 5 | 2 | 0 | FAIL | 12 | 1 | 22 | 0 | 0 | 0 | 0 | 0 | 5 |
| MAP10 | 1:37 2:4 4:1 11:1 27:1 32:2 46:5 62:3 63:1 71:4 88:1 90:6 97:1 114:1 126:4 | 31 | 1:4 2:3 3:2 7:7 9:18 13:1 17:2 | 26 | pass | FAIL | 0 | 0 | pass | 12 | 9 | 0 | FAIL | 21 | 1 | 10 | 0 | 0 | 0 | 0 | 0 | 5 |
| MAP11 | 1:1 11:1 20:2 26:2 28:2 48:4 62:1 97:8 102:2 105:2 109:2 117:2 120:5 123:3 | 25 | 3:1 7:11 8:4 9:3 17:6 | 26 | pass | FAIL | 4 | 4 | pass | 11 | 39 | 0 | FAIL | 4 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 6 |
| MAP12 | 1:5 2:2 18:2 22:2 26:1 27:1 32:1 33:1 36:4 37:4 39:1 52:4 59:7 61:2 62:8 71:1 87:1 89:1 97:6 103:6 119:8 | 55 | 1:8 5:1 8:2 9:4 12:7 16:9 | 46 | pass | FAIL | 0 | 0 | pass | 17 | 11 | 0 | FAIL | 5 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 7 |
| MAP13 | 1:10 2:1 3:1 7:1 19:1 23:1 26:1 31:6 32:1 33:1 34:1 36:4 38:3 48:8 52:4 62:2 73:3 74:6 77:3 83:1 87:1 89:1 97:20 102:2 103:1 | 52 | 1:5 8:1 9:8 12:3 16:1 | 40 | pass | FAIL | 8 | 8 | pass | 9 | 48 | 0 | FAIL | 16 | 0 | 2 | 0 | 0 | 0 | 0 | 1 | 12 |
| MAP14 | 11:1 26:2 33:2 42:1 60:1 61:1 62:1 63:2 73:2 74:1 105:11 114:7 117:8 120:3 123:75 | 117 | 17:5 | 58 | pass | FAIL | 0 | 0 | pass | 5 | 84 | 0 | FAIL | 1 | 0 | 88 | 0 | 0 | 0 | 0 | 0 | 14 |
| MAP15 | 1:5 2:4 13:1 23:1 27:8 30:1 31:1 32:6 33:2 35:3 52:4 62:7 71:5 97:15 103:2 105:3 106:1 109:5 117:1 119:1 120:7 123:9 124:4 127:2 137:6 | 73 | 2:1 3:1 5:10 7:12 8:8 9:11 16:4 17:10 | 58 | pass | FAIL | 0 | 0 | pass | 20 | 58 | 0 | FAIL | 9 | 0 | 16 | 0 | 0 | 0 | 0 | 0 | 6 |
| MAP16 | 1:12 31:5 32:1 33:2 46:2 52:2 62:7 97:1 103:1 106:3 123:12 126:12 | 38 | 5:1 7:1 9:4 17:3 | 25 | pass | FAIL | 0 | 0 | pass | 3 | 43 | 0 | FAIL | 6 | 0 | 21 | 0 | 0 | 0 | 0 | 0 | 10 |
| MAP17 | 1:1 2:1 16:1 19:1 20:2 30:1 31:4 32:1 33:2 34:1 48:20 52:1 62:6 63:1 97:13 102:2 103:3 105:1 109:19 112:1 117:6 118:1 120:5 123:7 | 65 | 1:1 5:2 7:15 8:6 9:3 16:1 17:12 | 50 | pass | FAIL | 20 | 20 | pass | 19 | 19 | 0 | FAIL | 16 | 0 | 14 | 0 | 0 | 0 | 0 | 0 | 12 |
| MAP18 | 1:8 3:1 5:2 7:1 19:7 23:1 32:1 34:1 35:1 36:4 45:2 46:2 52:1 62:1 97:13 102:3 103:1 | 41 | 1:5 2:1 3:2 9:4 12:2 16:3 | 31 | pass | FAIL | 0 | 0 | pass | 10 | 30 | 0 | FAIL | 10 | 0 | 5 | 0 | 0 | 0 | 0 | 1 | 5 |
| MAP19 | 1:23 7:1 18:1 19:2 23:1 31:4 32:1 33:1 34:1 36:1 39:1 47:2 48:29 52:4 61:1 62:6 63:4 67:1 68:1 71:1 73:1 74:5 87:2 88:1 89:3 90:1 97:23 102:2 103:1 | 62 | 4:1 8:4 9:9 | 35 | pass | FAIL | 29 | 7 | pass | 5 | 54 | 0 | FAIL | 18 | 1 | 13 | 0 | 0 | 0 | 0 | 0 | 6 |
| MAP20 | 1:12 2:4 18:1 22:5 31:1 36:2 38:2 46:3 52:4 62:8 70:1 83:6 88:20 90:1 91:1 94:1 97:33 103:1 117:2 126:1 | 90 | 2:3 3:1 5:8 8:14 9:7 16:2 | 37 | pass | FAIL | 0 | 0 | pass | 18 | 50 | 1 | FAIL | 19 | 6 | 12 | 0 | 0 | 0 | 0 | 6 | 10 |
| MAP21 | 23:1 32:10 33:7 34:6 36:4 52:2 62:41 71:4 97:36 103:1 109:2 127:4 | 93 | 7:1 8:3 | 27 | pass | FAIL | 0 | 0 | pass | 3 | 4 | 1 | FAIL | 2 | 0 | 41 | 0 | 0 | 0 | 0 | 0 | 16 |
| MAP22 | 11:1 32:1 33:2 62:6 68:1 97:4 109:1 114:5 117:6 118:2 123:8 | 33 | 3:1 7:14 9:3 16:1 17:11 | 31 | pass | FAIL | 0 | 0 | pass | 12 | 18 | 0 | FAIL | 1 | 0 | 20 | 0 | 0 | 0 | 0 | 0 | 4 |
| MAP23 | 2:3 3:1 16:1 18:1 31:2 34:1 46:1 52:4 61:1 62:2 71:1 97:26 109:6 118:1 | 43 | 1:1 2:1 5:3 8:2 9:2 17:1 | 16 | pass | FAIL | 0 | 0 | pass | 5 | 5 | 1 | FAIL | 4 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 6 |
| MAP24 | 1:8 2:2 5:1 7:2 19:1 20:1 22:6 23:3 28:2 32:2 35:1 36:1 52:5 62:7 67:8 88:7 97:63 102:1 103:1 | 105 | 1:1 2:2 7:7 8:22 9:4 16:5 | 42 | pass | FAIL | 0 | 0 | pass | 25 | 0 | 1 | FAIL | 10 | 5 | 15 | 0 | 0 | 0 | 0 | 0 | 6 |
| MAP25 | 32:2 52:4 60:1 62:8 63:2 97:10 105:2 117:4 120:3 123:5 | 39 | 3:2 7:13 8:1 9:2 13:1 17:1 | 13 | pass | FAIL | 0 | 0 | pass | 5 | 21 | 0 | FAIL | 1 | 0 | 16 | 0 | 0 | 0 | 0 | 0 | 8 |
| MAP26 | 2:2 22:2 32:2 33:2 34:4 35:1 37:1 52:4 62:4 63:1 88:1 90:9 97:32 103:1 105:4 109:10 114:2 115:2 116:1 117:11 118:4 119:1 123:1 | 75 | 1:2 3:1 5:3 7:4 8:6 9:4 16:3 17:5 | 24 | pass | FAIL | 0 | 0 | pass | 14 | 3 | 1 | FAIL | 6 | 2 | 11 | 0 | 0 | 0 | 0 | 0 | 7 |
| MAP27 | 1:11 2:22 7:1 23:1 27:2 28:1 31:1 32:1 35:6 36:5 38:26 39:1 46:2 52:1 60:2 61:2 62:10 71:2 88:10 97:6 103:1 109:15 110:1 112:2 114:1 115:1 117:1 120:6 | 123 | 4:1 7:1 8:5 9:8 10:1 17:17 | 52 | FAIL | FAIL | 0 | 0 | pass | 23 | 31 | 1 | FAIL | 25 | 4 | 18 | 0 | 0 | 0 | 0 | 0 | 9 |
| MAP28 | 1:2 2:8 3:1 31:5 33:1 34:2 35:2 36:2 46:3 48:33 52:7 61:1 62:5 70:1 88:1 97:1 119:2 138:1 139:1 | 29 | 5:4 9:7 17:13 | 28 | pass | FAIL | 33 | 26 | pass | 13 | 0 | 1 | FAIL | 5 | 1 | 12 | 0 | 0 | 0 | 0 | 0 | 7 |
| MAP29 | 1:3 2:1 18:1 20:4 38:1 52:4 71:2 86:1 97:32 102:5 103:1 109:4 119:1 123:15 126:3 131:2 | 73 | 1:1 3:2 7:6 8:6 16:9 17:28 | 37 | pass | FAIL | 0 | 0 | pass | 37 | 1 | 1 | FAIL | 8 | 0 | 15 | 0 | 0 | 0 | 0 | 0 | 8 |
| MAP30 | 48:1 62:9 97:1 140:1 | 11 | 7:8 8:1 | 4 | pass | FAIL | 1 | 0 | pass | 1 | 0 | 0 | FAIL | 0 | 0 | 9 | 0 | 0 | 0 | 0 | 0 | 4 |
| MAP31 | 1:44 11:1 31:6 51:1 | 0 | 1:1 9:4 17:2 | 0 | pass | FAIL | 0 | 0 | pass | 3 | 0 | 0 | FAIL | 22 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| MAP32 | 1:20 11:1 31:2 109:1 | 1 | 9:6 | 2 | pass | FAIL | 0 | 0 | pass | 0 | 0 | 1 | FAIL | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3 |

- line special types over all maps 1:551 2:181 3:9 4:1 5:9 7:17 8:3 9:2 10:1 11:39 13:5 14:1 15:1 16:5 18:18 19:38 20:30 21:1 22:25 23:34 24:1 25:1 26:40 27:56 28:29 29:1 30:6 31:134 32:58 33:50 34:39 35:17 36:35 37:42 38:67 39:6 40:4 42:7 44:1 45:3 46:35 47:3 48:250 49:1 51:5 52:90 56:6 58:5 59:13 60:6 61:23 62:273 63:55 65:1 67:9 68:2 70:5 71:25 73:25 74:39 75:6 76:2 77:9 82:6 83:8 86:12 87:10 88:141 89:15 90:38 91:19 94:1 97:651 98:4 99:2 100:1 101:6 102:42 103:75 104:2 105:24 106:6 109:110 110:1 112:7 114:65 115:19 116:1 117:69 118:13 119:13 120:63 123:321 124:4 125:5 126:29 127:7 131:2 133:3 134:6 135:27 136:6 137:7 138:2 139:2 140:1 141:1 65535:1
- sector special types over all maps 1:134 2:53 3:40 4:7 5:157 7:284 8:195 9:306 10:1 11:1 12:31 13:13 16:111 17:160
- stage A: sector specials in the Stage A set for 66 of 68
- stage A: line specials in {0, 48} for 0 of 68
- stage A: every scroller one-sided for 56 of 68; two-sided scrollers 143
- stage A: scrollers ≤ 64 for 68 of 68; maximum 63
- stage A: maxima: light-thinker sectors 37, sky ceilings 84, sky floors 1
- stage A all (with geometry admitted) for 0 of 68
- stage B: maxima: mover sectors 36, plat sectors 8, reusable switch lines 136, 2x2 window two-sided special lines 25
- stage B: plat sectors ≤ 30 for 68 of 68; reusable switch lines ≤ 16 for 53 of 68; 2x2 window ≤ 8 two-sided special lines for 51 of 68
- stage B: one-sided DR lines 4 in all; none for 67 of 68
- stage B: sectors in more than one mover family 2 in all; none for 67 of 68
- stage B: door sectors with a self-referencing line 0 in all; none for 68 of 68
- stage B: tag-0 lines with tag-using specials 0 in all; none for 68 of 68
- stage B: sky sectors that are mover sectors 29 in all; none for 57 of 68
- stage B: reachable heights and the load rules: see --movers-markdown
