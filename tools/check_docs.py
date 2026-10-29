#!/usr/bin/env python3
"""Check repository Markdown destinations/anchors and archived evidence digests."""
from pathlib import Path
from urllib.parse import unquote, urlsplit
import hashlib, json, re, subprocess
root=Path(__file__).resolve().parents[1]
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','--','*.md'],cwd=root,text=True).splitlines()
paths=sorted(set(paths));errors=[];links=anchors=0

def prose(path):
    fence=None;out=[]
    for line in path.read_text().splitlines():
        marker=re.match(r'^\s*(`{3,}|~{3,})',line)
        if marker:
            token=marker.group(1)[0]
            if fence is None:fence=token
            elif fence==token:fence=None
            continue
        if fence is None:out.append(line)
    return '\n'.join(out)

def slugs(path):
    seen={};result=set()
    for line in prose(path).splitlines():
        if not re.match(r'^#{1,6}\s+',line):continue
        heading=re.sub(r'^#{1,6}\s+','',line).strip().rstrip('#').strip().lower().replace('`','')
        heading=re.sub(r'\[([^\]]+)\]\([^)]+\)',r'\1',heading)
        slug=re.sub(r'[^\w\- ]','',heading).replace(' ','-')
        n=seen.get(slug,0);seen[slug]=n+1;result.add(slug+(f'-{n}' if n else ''))
    return result

for name in paths:
    path=root/name
    if not path.exists():continue
    for target in re.findall(r'\[[^\]]*\]\(([^)]+)\)',prose(path)):
        target=target.split(' "')[0].strip().strip('<>');u=urlsplit(target)
        if u.scheme or target.startswith('//'):continue
        dest=(path.parent/unquote(u.path)).resolve() if u.path else path.resolve()
        if not dest.exists():errors.append(f'{name}: missing {target}');continue
        links+=1
        if u.fragment:
            anchors+=1
            if not dest.is_file() or unquote(u.fragment) not in slugs(dest):errors.append(f'{name}: missing anchor {target}')
archive=root/'docs/validation/archive';manifest=json.loads((archive/'manifest.json').read_text())
for source,record in manifest.items():
    raw=(archive/record['exact_copy']).read_bytes()
    if len(raw)!=record['bytes'] or hashlib.sha256(raw).hexdigest()!=record['sha256']:errors.append('Changed archived original: '+source)
    nav=(archive/record['navigation_copy']).read_bytes()
    if hashlib.sha256(nav).hexdigest()!=record['navigation_sha256']:errors.append('Changed archived navigation: '+source)
if errors:raise SystemExit('\n'.join(errors))
print(f'{len(paths)} Markdown files: {links} relative links, {anchors} anchors resolved; {len(manifest)} exact archives verified.')
