"""Read-only observation adapters for the M3h1 source harness.

Strict audits only: A_PosAttack/A_SPosAttack/A_CPosAttack keep BAM angles in
`int` locals, so `angle += spread` is signed overflow near 180 degrees. The
audited build retypes exactly those locals to angle_t (explicit BAM); the run
asserts its trace equals the unmodified -fwrapv build.

A_Scream's grouped death-sound draw is otherwise an unattributed gameplay
draw; SR rows attribute the same single P_Random value to its actor.

The inherited m3d3d shotgunner instrumentation prints W rows that collide with
the genuine-player weapon W namespace; only those A_SPosAttack rows are renamed
SG. Source statements, ordering and RNG are untouched."""
import re
def retag_shotgunner(source):
    start=source.index('void A_SPosAttack (mobj_t* actor)')
    end=source.index('\nvoid ',start+10)
    body=source[start:end]
    assert body.count('printf("W,%d,%d,')==5
    return source[:start]+body.replace('printf("W,%d,%d,','printf("SG,%d,%d,')+source[end:]
def attribute_scream(source):
    start=source.index('void A_Scream')
    end=source.index('\nvoid ',start+10)
    body=source[start:end]
    for group,modulus in [('sfx_podth1',3),('sfx_bgdth1',2)]:
        old=f'sound = {group} + P_Random ()%{modulus};'
        assert body.count(old)==1
        body=body.replace(old,f'{{int scream_draw=P_Random();printf("SR,%d,%d,%d,%d\\n",scenario,operation,ident(actor),scream_draw);sound = {group} + scream_draw%{modulus};}}')
    return source[:start]+body+source[end:]
import re
def bam_attack_angles(source):
    for name,count in [('A_PosAttack',1),('A_SPosAttack',2),('A_CPosAttack',2)]:
        start=source.index('void '+name+' (mobj_t* actor)')
        end=source.index('\nvoid ',start+10)
        body,n=re.subn(r'\bint(\s+)(b?angle);',r'angle_t\1\2;',source[start:end])
        assert n==count,(name,n)
        source=source[:start]+body+source[end:]
    return source
