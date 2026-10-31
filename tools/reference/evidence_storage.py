#!/usr/bin/env python3
"""Lossless zstd maintenance, with independent immutable Git-object proof.

Ordinary tests never invoke Git or fetch history. This explicit audit does.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
BASELINE = '138c66b6c6b6d57d0b18cf945d8661722741bd66'
MANIFEST = ROOT/'tests/reference/zstd_manifest.json'
INVENTORY = ROOT/'docs/validation/zstd-baseline.json'

def checked(command, **kw):
    return subprocess.run(command, cwd=ROOT, check=True, **kw)

def codec():
    checked(['cargo','build','-q','-p','doom-test-evidence','--bin','evidence','--locked'])
    return ROOT/'target/debug'/('evidence.exe' if os.name=='nt' else 'evidence')

def original(path):
    return checked(['git','show',f'{BASELINE}:{path}'],capture_output=True).stdout

def replacement(match):
    prefix,name=match.group(1),match.group(2)
    # Both include paths have the same parent as their old CSV include.
    return (f'doom_test_evidence::text("tests/reference/{name}", '
            f'include_bytes!("{prefix}{name}.zst"), '
            f'include_str!("{prefix}zstd_manifest.json")).expect("verified historical zstd evidence")')

def adapt(source, names):
    pattern=r'include_str!\("((?:\.\./)+(?:tests/)?reference/)([^"/]+\.csv)"\)'
    return re.sub(pattern, lambda m: replacement(m) if m.group(2) in names else m.group(0),source)

def verify_git(binary):
    m=json.loads(MANIFEST.read_text()); entries=m['entries']; paths={e['original_path'] for e in entries}
    inventory=json.loads(INVENTORY.read_text())
    assert m['baseline']==BASELINE==inventory['baseline']
    assert inventory['tree']==checked(['git','rev-parse',BASELINE+'^{tree}'],capture_output=True).stdout.decode().strip()
    assert paths=={e['path'] for e in inventory['files']}, 'migration allowlist mismatch'
    with tempfile.TemporaryDirectory(prefix='doom-byte-proof-') as directory:
        out=Path(directory)
        checked([str(binary),'extract',str(ROOT),str(MANIFEST),str(out)])
        for e in entries:
            before=original(e['original_path']); after=(out/e['original_path']).read_bytes()
            recorded=next(x for x in inventory['files'] if x['path']==e['original_path'])
            assert len(before)==recorded['bytes'] and hashlib.sha256(before).hexdigest()==recorded['sha256']
            blob=hashlib.sha1(b'blob '+str(len(before)).encode()+b'\0'+before).hexdigest()
            assert e['original_git_blob']==recorded['git_blob']==blob
            assert before==after, e['original_path']
    # Check every historical source/evidence/helper/archive path against Git.
    names={Path(p).name for p in paths}; protected=0; adapters=[]
    rows=checked(['git','ls-tree','-r',BASELINE,'--','ipx','sersrc','sndserv','linuxdoom-1.10','tests/reference','tests/support','tools/reference','docs/validation','README.TXT','LICENSE.TXT'],capture_output=True).stdout.decode().splitlines()
    for row in rows:
        header,path=row.split('\t')
        if path in paths:
            assert not (ROOT/path).exists(), 'raw counterpart should be absent after migration'
            continue
        after=(ROOT/path).read_bytes()
        if path.startswith('tests/support/') and path.endswith('.rs'):
            before=original(path)
            transformed=adapt(before.decode(),names)
            expected=checked(['rustup','run','stable','rustfmt','--edition','2024','--emit','stdout'],input=transformed.encode(),capture_output=True).stdout if transformed!=before.decode() else before
            if expected!=before: adapters.append(path)
            assert after==expected, path
        elif path=='tools/reference/README.md':
            continue # active human guide, not archived evidence or provenance
        else:
            blob=hashlib.sha1(b'blob '+str(len(after)).encode()+b'\0'+after).hexdigest()
            assert blob==header.split()[2], path
        protected+=1
    # Preserve exact original 65 CI source commands, including sanitizer flags.
    workflow=original('.github/workflows/rust.yml').decode()
    block=workflow.split('      - name: Reproduce isolated reference vectors and snapshot goldens\n',1)[1].split('      - name:',1)[0]
    expected=[line.strip() for line in block.splitlines() if line.strip().startswith(('python3 ','ASAN_OPTIONS='))]
    assert expected==json.loads((ROOT/'tools/reference/source_suite.json').read_text())
    for path in ['crates/doom-sim/tests/m3d2_reference.rs','crates/doom-sim/tests/m3d3i_reference.rs','crates/doom-sim/tests/m3f1_reference.rs','crates/doom-sim/tests/m3f2_reference.rs']:
        before=original(path).decode(); transformed=adapt(before,names)
        assert transformed!=before
        formatted=checked(['rustup','run','stable','rustfmt','--edition','2024','--emit','stdout'],input=transformed.encode(),capture_output=True).stdout
        assert (ROOT/path).read_bytes()==formatted, path
        adapters.append(path)
    upstream=sum(row.split('\t')[1].split('/')[0] in ['linuxdoom-1.10','ipx','sersrc','sndserv','LICENSE.TXT','README.TXT'] for row in rows)
    assert upstream==165
    print(json.dumps({'decoded_actual_byte_equal':len(entries),'protected_paths_checked':protected,'unchanged_paths':protected-(len(adapters)-4),'loading_only_adapters':adapters,'upstream_files':upstream,'source_commands':len(expected)}))

def migrate(binary):
    inventory=json.loads(INVENTORY.read_text()); entries=[]; proofs=[]
    with tempfile.TemporaryDirectory(prefix='doom-migration-') as directory:
        for e in inventory['files']:
            path=e['path']; raw=(ROOT/path).read_bytes(); baseline=original(path)
            assert raw==baseline and len(raw)==e['bytes'] and hashlib.sha256(raw).hexdigest()==e['sha256']
            packed=ROOT/(path+'.zst')
            checked([str(binary),'encode',str(ROOT/path),str(packed)])
            decoded=Path(directory)/Path(path).name
            checked([str(binary),'decode',str(packed),str(decoded),str(len(raw))])
            assert decoded.read_bytes()==raw==baseline, path
            data=packed.read_bytes()
            entries.append(dict(original_path=path,compressed_path=path+'.zst',original_length=len(raw),original_sha256=e['sha256'],original_git_blob=e['git_blob'],compressed_length=len(data),compressed_sha256=hashlib.sha256(data).hexdigest(),decode_budget=len(raw),family=e['family']))
            proofs.append(dict(path=path,raw_equals_git=True,decoded_equals_raw=True,decoded_equals_git=True))
        manifest=dict(schema='DOOM-ZSTD-EVIDENCE-v1',baseline=BASELINE,encoder='zstd crate 0.13.3; zstd-safe 7.3.0; libzstd 1.5.7',parameters='level=9,window_log=23,checksum=true,content_size=true,workers=0,dictionary=none',entries=sorted(entries,key=lambda e:e['original_path']))
        MANIFEST.write_text(json.dumps(manifest,indent=2)+'\n')
        checked([str(binary),'verify',str(ROOT),str(MANIFEST)])
        # Removal occurs ONLY after every file passed all exact comparisons.
        for e in entries: (ROOT/e['original_path']).unlink()
    (ROOT/'docs/validation/zstd-migration-proof.json').write_text(json.dumps(dict(baseline=BASELINE,proofs=proofs,original_bytes=sum(e['original_length']for e in entries),compressed_bytes=sum(e['compressed_length']for e in entries)),indent=2)+'\n')
    names={Path(e['original_path']).name for e in entries}; consumers=[]
    for folder in ['tests/support','crates/doom-sim/tests']:
        for path in (ROOT/folder).glob('*.rs'):
            before=path.read_text();after=adapt(before,names)
            if before!=after: path.write_text(after);consumers.append(str(path.relative_to(ROOT)))
    print('Updated loading-only consumers:',consumers)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('operation',choices=['migrate','verify-git','verify']);a=p.parse_args();binary=codec()
    if a.operation=='migrate': migrate(binary)
    elif a.operation=='verify-git': verify_git(binary)
    else: checked([str(binary),'verify',str(ROOT),str(MANIFEST)])
if __name__=='__main__': main()
