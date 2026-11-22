#!/usr/bin/env python3
"""Wire 25 on every scene: the genuine sound stream from the strict M5c runner (with the repository's
synthetic GFX/SFX/MUSIC PWADs and the recommended switches, as m5c_wire25_check.py runs them), the
strict M4b dumps already made by run_scenes.py, sound_records_run --captures, and the repository's
level_compare_m5c.compare (DSIM41, every field and every sound record). Repository modules are
imported read-only (sys.dont_write_bytecode).

    run_w25.py [--bin PATH TAG] [NAME ...]
"""
import json, subprocess, sys
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, '/home/copgunn/src/DOOM/tools/reference')
import m5c_reference as R         # noqa: E402
import level_snapshot as L        # noqa: E402
import level_snapshot_m4b as M    # noqa: E402
import level_snapshot_m5c as S    # noqa: E402
import level_compare_m5c as K     # noqa: E402

C = Path('/home/copgunn/scratch/quirks/followup-b/corner')
M5C = C / 'build5c/strict/crunner'
BIN = Path('/home/copgunn/scratch/quirks/followup-b/target/release/examples/sound_records_run')

def main():
    args = sys.argv[1:]
    binary, tag = BIN, 'rust'
    if args and args[0] == '--bin':
        binary, tag = Path(args[1]), args[2]; args = args[3:]
    scenes = json.loads((C / 'wads/scenes.json').read_text())
    names = args or [n for n, v in scenes.items() if not v['optional']]
    out = {}
    for name in names:
        wad, lmp, cdir = C / f'wads/{name}.wad', C / f'wads/{name}.lmp', C / f'runs/strict_{name}'
        sound = C / f'runs/sound_{name}.txt'
        if not sound.exists():
            sw = [x for k, v in R.RECOMMENDED.items() for x in (k, v)]
            p = subprocess.run([str(M5C), *[x for f in (wad, R.GFX, R.SFX, R.MUSIC, lmp) for x in ('--wad', str(f))], '--demo', name,
                                '--no-dump', '--render', 'none', *sw, '--sound-out', str(sound)], capture_output=True, text=True, env=R.ENV)
            if p.returncode or not sound.exists(): sys.exit(f'{name}: M5c runner exit {p.returncode}: {p.stderr[-300:]}')
        rdir = C / f'{tag}/w25_{name}'
        rdir.mkdir(parents=True, exist_ok=True)
        for f in rdir.glob('*'): f.unlink()
        p = subprocess.run([str(binary), '--wad', str(wad), '--map', 'E1M1', '--demo', str(lmp), '--captures', str(rdir)],
                           capture_output=True, text=True)
        (C / f'{tag}/w25_{name}.stdout').write_text(p.stdout + p.stderr)
        if p.returncode:
            print(f'{name} wire 25: sound_records_run exit {p.returncode}: {(p.stdout + p.stderr)[-300:]}'); continue
        skill, number = M.demo_header(lmp)
        mapdata = L.MapData(str(wad), 'E1M1', L.demo_lumps(str(lmp)))
        result, lines = K.compare(str(cdir), str(rdir), mapdata, skill, number, S.SoundStream(text=sound.read_text()))
        snd = result['sounds']
        out[name] = dict(match=result['match'], tics=result['tics_compared'], c_end=result['c_end'], rust_end=result['rust_end'],
                         map_group_equal=result['map_group_equal'], starts=snd['starts'], stops=snd['stops'],
                         disagreements=snd['disagreements'], first=lines[:4])
        print(f"{name} wire 25: {K.summary_line(result)}")
        if not result['match']: print('   ', '\n    '.join(lines[:6]))
    sp = C / f'{tag}/summary_w25.json'
    old = json.loads(sp.read_text()) if sp.exists() else {}
    old.update(out); sp.write_text(json.dumps(old, indent=1))

if __name__ == '__main__':
    main()
