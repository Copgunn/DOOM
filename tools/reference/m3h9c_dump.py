#!/usr/bin/env python3
"""Reader and comparator for the M3h9c runner's per-tic dumps.

The runner (m3h9c_driver.c) writes one text file per snapshot into its --out directory:
`load.txt` after G_DoPlayDemo (tic -1) and `tic_NNNNN.txt` after G_Ticker number N
(0-based) and gametic++. Each file starts with one `#columns,KIND,name,...` line per row
kind (the contract's `dump_row_kinds`), followed by rows `KIND,tic,...` of raw integers
(fixed_t raw, angles unsigned):

  H   header: leveltime, gametic, prndindex, rndindex, totals, gameaction, gamestate,
      numthinkers (a walk of thinkercap), numlinespecials, gameskill
  one row per thinker in thinker-list order, `idx` = position in that list:
      M mobj_t (the existing A-row order, then tracer, spawnpoint, player, BLOCKMAP cell
        and position from the cell's head, sector thing-list sector and position),
      DR vldoor_t, PL plat_t, FL floormove_t, CE ceiling_t (PL/CE `running` = 0 for a
        mover in stasis, found through activeplats/activeceilings), LF fireflicker_t,
      LL lightflash_t, LS strobe_t, LG glow_t, R removed but not yet freed (function -1),
      X unknown function
  T   per thinker (idx as above): its serial (the order of P_AddThinker calls since the
      start, never reused) and its link serial (the order of P_SetThingPosition calls; the
      global link order is the ascending current link serial; -1 for a non-mobj)
  K   per thinker freed since the previous snapshot, in free order: serial and sequence
      number (seq -1: dropped at a level setup, after Z_FreeTags)
  RM  per mobj removed and freed since the previous snapshot: the M columns with `serial`
      in place of `idx`, copied just before the genuine Z_Free (bpos/spos -1: unlinked by
      P_RemoveMobj)
  S   every sector; L every line whose special was nonzero at load; B the 16 buttons;
  P   the four players; the last column `message` is 0 (NULL), the 1-based index of the
      d_englsh.h pickup string GOTARMOR..GOTSHOTGUN2 in header order, or -1 (another
      string), as HU_Ticker saw it before clearing it.
References are thinker-order indices (-1 NULL, -2 a pointer not in the thinker list);
BLOCKMAP/sector positions are -1 when not linked and -2 when not found where expected.

Dumps of a real IWAD hold map-derived values (coordinates, angles): they stay in local
scratch. `diff` therefore prints the first differing tic, row kind, row ordinal and
column name only; `--values` also prints the two values and is for local use only.

    m3h9c_dump.py diff DIR_A DIR_B [--values]
"""
import argparse, os, re, sys

def snapshot_names(directory):
    """`load.txt` (if present) followed by the tic files in tic order."""
    tics = sorted(n for n in os.listdir(directory) if re.fullmatch(r'tic_\d{5}\.txt', n))
    return (['load.txt'] if os.path.exists(os.path.join(directory, 'load.txt')) else []) + tics

def tic_label(name):
    return 'load' if name == 'load.txt' else str(int(name[4:9]))

def parse(path):
    """Return (columns, rows): columns maps a row kind to its column names after the kind
    (starting with 'tic'); rows is the list of data rows as string lists, kind first."""
    columns, rows = {}, []
    with open(path) as f:
        for line in f:
            fields = line.rstrip('\n').split(',')
            if fields[0] == '#columns': columns[fields[1]] = fields[2:]
            else: rows.append(fields)
    return columns, rows

def records(path):
    """Rows as dictionaries of ints keyed by column name, grouped by kind."""
    columns, rows = parse(path)
    out = {}
    for r in rows:
        out.setdefault(r[0], []).append(dict(zip(columns[r[0]], map(int, r[1:]))))
    return out

def first_difference(dir_a, dir_b, values=False):
    """None if identical, else a one-line description of the first difference."""
    na, nb = snapshot_names(dir_a), snapshot_names(dir_b)
    for name in na:
        if name not in nb: return f'tic {tic_label(name)}: snapshot missing in the second directory'
        ca, ra = parse(os.path.join(dir_a, name)); cb, rb = parse(os.path.join(dir_b, name))
        if ca != cb: return f'tic {tic_label(name)}: column definitions differ'
        ordinal = {}
        for i in range(max(len(ra), len(rb))):
            if i >= len(ra) or i >= len(rb): return f'tic {tic_label(name)}: row count differs at row {i}'
            x, y = ra[i], rb[i]
            if x[0] != y[0]: return f'tic {tic_label(name)}: row {i} kind {x[0]} versus {y[0]}'
            k = ordinal[x[0]] = ordinal.get(x[0], -1) + 1
            if x == y: continue
            names = ['kind'] + ca.get(x[0], [])
            for j in range(max(len(x), len(y))):
                if j >= len(x) or j >= len(y) or x[j] != y[j]:
                    col = names[j] if j < len(names) else f'column {j}'
                    text = f'tic {tic_label(name)}: row kind {x[0]} #{k} column {col}'
                    if values:
                        text += f' ({x[j] if j < len(x) else "-"} versus {y[j] if j < len(y) else "-"})'
                    return text
    if len(nb) != len(na): return f'snapshot count {len(na)} versus {len(nb)}'
    return None

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='command', required=True)
    d = sub.add_parser('diff', help='first difference between two dump directories')
    d.add_argument('a'); d.add_argument('b')
    d.add_argument('--values', action='store_true', help='also print the two values (local use only)')
    a = ap.parse_args()
    for x in (a.a, a.b):
        if not os.path.isdir(x): sys.exit(f'not a directory: {x}')
    diff = first_difference(a.a, a.b, a.values)
    if diff is None:
        print(f'identical: {len(snapshot_names(a.a))} snapshots'); return 0
    print('first difference: ' + diff); return 1

if __name__ == '__main__':
    sys.exit(main())
