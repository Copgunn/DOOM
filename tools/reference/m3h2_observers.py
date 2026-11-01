"""Read-only observation adapters for the M3h2 source harness.

Each row brackets a genuine original statement; source order, arithmetic and
RNG are untouched. MS row codes reuse the historical projectile observation
namespace: 0 explosion, 1 launch velocity, 2 draw (0 spawn tics, 1/2 shadow,
3 contact damage, 4 explosion tics), 4 half-step, 5 validated launch. Tag MS is
new because the frozen harness already uses P for the player capture."""
HELPERS=r'''
static int missile_random(int kind){int value=P_Random();printf("MS,%d,%d,2,%d,%d\n",scenario,operation,kind,value);return value;}
static int missile_spread(void){int a=missile_random(1);int b=missile_random(2);return (a-b)*1048576;}
static void missile_velocity(mobj_t* p){printf("MS,%d,%d,1,%d,%u,%d,%d,%d\n",scenario,operation,ident(p),p->angle,p->momx,p->momy,p->momz);}
'''
def replace_once(text,old,new):
    assert text.count(old)==1,old
    return text.replace(old,new)
def replace_last(text,old,new):
    assert text.count(old)>=1,old
    at=text.rindex(old)
    return text[:at]+new+text[at+len(old):]
def missile_body(name,text,action):
    if name=='P_SpawnMissile':
        text=replace_once(text,'th = P_SpawnMobj (source->x,','printf("MS,%d,%d,5,%d,0,%d,%d\\n",scenario,operation,ident(source),ident(dest),type);\n    th = P_SpawnMobj (source->x,')
        text=replace_once(text,'th->target = source;','th->target = source;observe(6,ident(th),ident(source));')
        text=replace_once(text,'(P_Random()-P_Random())<<20','missile_spread()')
        return replace_once(text,'    P_CheckMissileSpawn (th);','    missile_velocity(th);P_CheckMissileSpawn (th);')
    if name=='P_CheckMissileSpawn':
        text=replace_once(text,'P_Random()','missile_random(0)')
        return replace_once(text,'    if (!P_TryMove','    printf("MS,%d,%d,4,%d,%d,%d,%d,%d\\n",scenario,operation,ident(th),th->x,th->y,th->z,th->tics);\n    if (!P_TryMove')
    if name=='P_ExplodeMissile':
        text=text.replace('{','{\n printf("MS,%d,%d,0,%d\\n",scenario,operation,ident(mo));',1)
        return replace_once(text,'P_Random()','missile_random(4)')
    if name=='PIT_CheckThing':
        # The second textual draw is the MF_MISSILE contact; MF_SKULLFLY stays frozen.
        old='damage = ((P_Random()%8)+1)*tmthing->info->damage;'
        assert text.count(old)==2
        return replace_last(text,old,'damage = ((missile_random(3)%8)+1)*tmthing->info->damage;')
    if name=='A_TroopAttack':
        return replace_once(text,'P_Random()%8','monster_random(actor,4)%8')
    return text
