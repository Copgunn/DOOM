/*
 * M3h9c headless genuine-engine runner: the driver (declared adapter).
 *
 * Mirrors the vanilla -playdemo path (d_main.c:948-958, 1018-1141; d_net.c:742-746)
 * without video, sound, menus or network:
 *   Z_Init (d_main.c:1018); W_InitMultipleFiles over the --wad files, added as D_AddFile
 *   does (d_main.c:543-556, 1021): the IWAD, then the .lmp, which W_AddFile's single-lump
 *   branch names after its upper-cased base name (w_wad.c:172-180, 86-114);
 *   gamemode = registered (the doom.wad branch of IdentifyVersion, d_main.c:697-701);
 *   R_InitTextures and R_InitFlats (the first two calls of R_InitData, r_data.c:654-658,
 *   with the declared adapters; sprites and colormaps are not loaded);
 *   P_InitSwitchList and P_InitPicAnims (P_Init, p_setup.c:700-705, without R_InitSprites);
 *   the D_CheckNetGame single-player values (d_net.c:567-592); singledemo = true and
 *   G_DeferedPlayDemo (d_main.c:1135-1138); G_DoPlayDemo () called directly (the
 *   ga_playdemo case of G_Ticker, g_game.c:617-652), so the loaded level is dumped as
 *   "load" before tic 0: the first G_Ticker then finds gameaction == ga_nothing, the
 *   state vanilla reaches inside that G_Ticker after its gameaction loop; then
 *   { G_Ticker (); gametic++; } as TryRunTics does (d_net.c:742-746), with a dump after
 *   every tic.
 * The demo header selects skill, episode and map (G_DoPlayDemo, g_game.c:1596-1598), so
 * the driver has no map argument.
 *
 * Declared driver behaviour beyond vanilla:
 *   - End of the demo: G_CheckDemoStatus calls I_Quit at the marker (g_game.c:1660-1661),
 *     which in vanilla exits the process (i_system.c:116-124; D_DoomLoop never returns,
 *     d_main.c:1140). The stub I_Quit sets stub_quit_flag and longjmps back to the setjmp
 *     around G_Ticker below, so, as with exit, no later statement of G_CheckDemoStatus
 *     (Z_ChangeTag, the global resets) or of G_Ticker (P_Ticker) runs.
 *   - After every tic the driver calls the genuine Z_CheckHeap (z_zone.c:399-420), a
 *     read-only walk of the zone block list that calls I_Error on corruption; vanilla
 *     calls it only in G_DoLoadLevel (g_game.c:488). --no-checkheap disables it.
 *   - --selftest-extra-draw TIC (test only) calls the public P_Random () once before
 *     G_Ticker of tic TIC; it never touches gameplay code text.
 *   - --zone-mb N sets the zone size (vanilla mb_used = 6, i_system.c:51).
 *
 * Dumps are written only to --out: they hold map-derived values and stay local.
 * tools/reference/m3h9c_dump.py documents and reads the format.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <time.h>
#include "doomdef.h"
#include "doomstat.h"
#include "z_zone.h"
#include "w_wad.h"
#include "g_game.h"
#include "m_random.h"
#include "p_local.h"
#include "p_spec.h"
#include "r_state.h"

/* genuine definitions without header declarations */
void R_InitTextures (void);
void R_InitFlats (void);
void T_FireFlicker (fireflicker_t* flick);   /* p_lights.c:46 */
extern short numlinespecials;
extern boolean singledemo;
extern int rndindex, prndindex;
extern gameaction_t gameaction;
extern thinker_t thinkercap;
extern button_t buttonlist[MAXBUTTONS];
extern plat_t* activeplats[MAXPLATS];
extern ceiling_t* activeceilings[MAXCEILINGS];
extern int bmapwidth, bmapheight;
extern fixed_t bmaporgx, bmaporgy;
extern mobj_t** blocklinks;
/* layout_check.c (generated from the adapted r_data.c text) */
int layout_patchcount_offset (void);
int layout_patches_offset (void);
int layout_mappatch_size (void);
/* stubs.c */
extern int stub_quit_flag, stub_quit_armed, stub_zone_mb;
extern jmp_buf stub_quit_jmp;

#define MAXWADS 20                       /* d_main.c MAXWADFILES */
static char* wadfiles[MAXWADS + 1];
static int numwads;
static void add_file (char* file)        /* d_main.c:543-556 */
{
    char* newfile = malloc (strlen (file) + 1);
    strcpy (newfile, file);
    wadfiles[numwads++] = newfile;
}

/* ---------------- dump ---------------- */
static thinker_t** order;        /* thinker-list order */
static int numth, capth;
typedef struct { void* p; int i; } pidx_t;
static pidx_t* sorted;
static unsigned char* linesel;   /* lines with a nonzero special at load */

static int cmp_pidx (const void* a, const void* b)
{
    const char* x = ((const pidx_t*)a)->p; const char* y = ((const pidx_t*)b)->p;
    return x < y ? -1 : x > y;
}
static void walk_thinkers (void)
{
    thinker_t* t; int i;
    numth = 0;
    for (t = thinkercap.next; t != &thinkercap; t = t->next) {
        if (numth == capth) { capth = capth ? capth * 2 : 1024; order = realloc (order, capth * sizeof *order);
                              sorted = realloc (sorted, capth * sizeof *sorted); }
        order[numth++] = t;
    }
    for (i = 0; i < numth; i++) { sorted[i].p = order[i]; sorted[i].i = i; }
    qsort (sorted, numth, sizeof *sorted, cmp_pidx);
}
/* thinker-order index; -1 NULL; -2 pointer not in the thinker list (dangling) */
static int tidx (void* p)
{
    pidx_t key, *r;
    if (!p) return -1;
    key.p = p;
    r = bsearch (&key, sorted, numth, sizeof *sorted, cmp_pidx);
    return r ? r->i : -2;
}
static long sidx (sector_t* s) { return s ? (long)(s - sectors) : -1L; }
static long stidx (state_t* s) { return s ? (long)(s - states) : -1L; }
static int in_plats (void* p) { int i; for (i = 0; i < MAXPLATS; i++) if (activeplats[i] == p) return 1; return 0; }
static int in_ceils (void* p) { int i; for (i = 0; i < MAXCEILINGS; i++) if (activeceilings[i] == p) return 1; return 0; }

static const char* COLUMNS =
 "#columns,H,tic,leveltime,gametic,prndindex,rndindex,totalkills,totalitems,totalsecret,gameaction,gamestate,numthinkers,numlinespecials\n"
 "#columns,M,tic,idx,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,R,tic,idx\n"
 "#columns,DR,tic,idx,type,sector,topheight,speed,direction,topwait,topcountdown\n"
 "#columns,PL,tic,idx,running,sector,speed,low,high,wait,count,status,oldstatus,crush,tag,type\n"
 "#columns,FL,tic,idx,type,crush,sector,direction,newspecial,texture,floordestheight,speed\n"
 "#columns,CE,tic,idx,running,type,sector,bottomheight,topheight,speed,crush,direction,tag,olddirection\n"
 "#columns,LF,tic,idx,sector,count,maxlight,minlight\n"
 "#columns,LL,tic,idx,sector,count,maxlight,minlight,maxtime,mintime\n"
 "#columns,LS,tic,idx,sector,count,minlight,maxlight,darktime,brighttime\n"
 "#columns,LG,tic,idx,sector,minlight,maxlight,direction\n"
 "#columns,X,tic,idx\n"
 "#columns,S,tic,idx,floorheight,ceilingheight,floorpic,ceilingpic,lightlevel,special,tag,soundtraversed,soundtarget,specialdata,linecount\n"
 "#columns,L,tic,idx,special,tag,flags,s0top,s0mid,s0bottom,s0textureoffset,s0rowoffset,s1top,s1mid,s1bottom\n"
 "#columns,B,tic,idx,line,where,btexture,btimer\n"
 "#columns,P,tic,idx,ingame,mo,playerstate,cmd_forward,cmd_side,cmd_turn,cmd_buttons,viewheight,deltaviewheight,bob,health,armorpoints,armortype,"
 "pw0,pw1,pw2,pw3,pw4,pw5,card0,card1,card2,card3,card4,card5,backpack,frag0,frag1,frag2,frag3,readyweapon,pendingweapon,"
 "wo0,wo1,wo2,wo3,wo4,wo5,wo6,wo7,wo8,ammo0,ammo1,ammo2,ammo3,maxammo0,maxammo1,maxammo2,maxammo3,attackdown,usedown,cheats,refire,"
 "killcount,itemcount,secretcount,damagecount,bonuscount,attacker,extralight,fixedcolormap,colormap,"
 "ps0state,ps0tics,ps0sx,ps0sy,ps1state,ps1tics,ps1sx,ps1sy,didsecret\n";

static void block_pos (mobj_t* m, int* bx, int* by, int* bpos)
{
    int x, y, n; mobj_t* l;
    *bx = *by = *bpos = -1;
    if (m->flags & MF_NOBLOCKMAP) return;
    x = (m->x - bmaporgx) >> MAPBLOCKSHIFT;    /* as P_SetThingPosition, p_maputl.c:510-511 */
    y = (m->y - bmaporgy) >> MAPBLOCKSHIFT;
    if (x < 0 || y < 0 || x >= bmapwidth || y >= bmapheight) return;
    *bx = x; *by = y; *bpos = -2;
    for (n = 0, l = blocklinks[y * bmapwidth + x]; l; l = l->bnext, n++) if (l == m) { *bpos = n; return; }
}
static int sector_pos (mobj_t* m)
{
    int n; mobj_t* l;
    if (m->flags & MF_NOSECTOR) return -1;
    for (n = 0, l = m->subsector->sector->thinglist; l; l = l->snext, n++) if (l == m) return n;
    return -2;
}

static void dump (const char* dir, const char* name, int tic)
{
    char path[1024]; FILE* f; int i, j;
    snprintf (path, sizeof path, "%s/%s", dir, name);
    f = fopen (path, "w");
    if (!f) { perror (path); exit (4); }
    walk_thinkers ();
    fputs (COLUMNS, f);
    fprintf (f, "H,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, leveltime, gametic, prndindex, rndindex,
             totalkills, totalitems, totalsecret, (int)gameaction, (int)gamestate, numth, (int)numlinespecials);
    for (i = 0; i < numth; i++) {
        thinker_t* t = order[i];
        actionf_p1 fn = t->function.acp1;
        if (fn == (actionf_p1)P_MobjThinker) {
            mobj_t* m = (mobj_t*)t; int bx, by, bp;
            block_pos (m, &bx, &by, &bp);
            fprintf (f, "M,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%u,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d\n",
                tic, i, (int)m->type, m->x, m->y, m->z, m->momx, m->momy, m->momz, m->health, stidx (m->state), m->tics,
                m->flags, m->height, m->radius, m->reactiontime, m->threshold, tidx (m->target), m->lastlook,
                m->floorz, m->ceilingz, m->subsector ? (long)(m->subsector - subsectors) : -1L, (unsigned)m->angle,
                (int)m->sprite, m->frame, m->movedir, m->movecount, tidx (m->tracer),
                m->spawnpoint.x, m->spawnpoint.y, m->spawnpoint.angle, m->spawnpoint.type, m->spawnpoint.options,
                m->player ? (int)(m->player - players) : -1, bx, by, bp,
                (m->flags & MF_NOSECTOR) ? -1L : sidx (m->subsector->sector), sector_pos (m));
        } else if (t->function.acv == (actionf_v)(-1)) {
            fprintf (f, "R,%d,%d\n", tic, i);
        } else if (fn == (actionf_p1)T_VerticalDoor) {
            vldoor_t* d = (vldoor_t*)t;
            fprintf (f, "DR,%d,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, (int)d->type, sidx (d->sector), d->topheight, d->speed,
                     d->direction, d->topwait, d->topcountdown);
        } else if (fn == (actionf_p1)T_PlatRaise || (fn == NULL && in_plats (t))) {
            plat_t* p = (plat_t*)t;
            fprintf (f, "PL,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, i, fn != NULL, sidx (p->sector), p->speed, p->low,
                     p->high, p->wait, p->count, (int)p->status, (int)p->oldstatus, (int)p->crush, p->tag, (int)p->type);
        } else if (fn == (actionf_p1)T_MoveFloor) {
            floormove_t* p = (floormove_t*)t;
            fprintf (f, "FL,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, (int)p->type, (int)p->crush, sidx (p->sector),
                     p->direction, p->newspecial, (int)p->texture, p->floordestheight, p->speed);
        } else if (fn == (actionf_p1)T_MoveCeiling || (fn == NULL && in_ceils (t))) {
            ceiling_t* p = (ceiling_t*)t;
            fprintf (f, "CE,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d\n", tic, i, fn != NULL, (int)p->type, sidx (p->sector),
                     p->bottomheight, p->topheight, p->speed, (int)p->crush, p->direction, p->tag, p->olddirection);
        } else if (fn == (actionf_p1)T_FireFlicker) {
            fireflicker_t* p = (fireflicker_t*)t;
            fprintf (f, "LF,%d,%d,%ld,%d,%d,%d\n", tic, i, sidx (p->sector), p->count, p->maxlight, p->minlight);
        } else if (fn == (actionf_p1)T_LightFlash) {
            lightflash_t* p = (lightflash_t*)t;
            fprintf (f, "LL,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, sidx (p->sector), p->count, p->maxlight, p->minlight,
                     p->maxtime, p->mintime);
        } else if (fn == (actionf_p1)T_StrobeFlash) {
            strobe_t* p = (strobe_t*)t;
            fprintf (f, "LS,%d,%d,%ld,%d,%d,%d,%d,%d\n", tic, i, sidx (p->sector), p->count, p->minlight, p->maxlight,
                     p->darktime, p->brighttime);
        } else if (fn == (actionf_p1)T_Glow) {
            glow_t* p = (glow_t*)t;
            fprintf (f, "LG,%d,%d,%ld,%d,%d,%d\n", tic, i, sidx (p->sector), p->minlight, p->maxlight, p->direction);
        } else {
            fprintf (f, "X,%d,%d\n", tic, i);
        }
    }
    for (i = 0; i < numsectors; i++) {
        sector_t* s = &sectors[i];
        fprintf (f, "S,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, i, s->floorheight, s->ceilingheight,
                 (int)s->floorpic, (int)s->ceilingpic, (int)s->lightlevel, (int)s->special, (int)s->tag,
                 s->soundtraversed, tidx (s->soundtarget), tidx (s->specialdata), s->linecount);
    }
    for (i = 0; i < numlines; i++) {
        line_t* l = &lines[i]; side_t* s0; side_t* s1;
        if (!linesel[i]) continue;
        s0 = l->sidenum[0] >= 0 ? &sides[l->sidenum[0]] : NULL;
        s1 = l->sidenum[1] >= 0 ? &sides[l->sidenum[1]] : NULL;
        fprintf (f, "L,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, i, (int)l->special, (int)l->tag, (int)l->flags,
                 s0 ? (int)s0->toptexture : -1, s0 ? (int)s0->midtexture : -1, s0 ? (int)s0->bottomtexture : -1,
                 s0 ? s0->textureoffset : 0, s0 ? s0->rowoffset : 0,
                 s1 ? (int)s1->toptexture : -1, s1 ? (int)s1->midtexture : -1, s1 ? (int)s1->bottomtexture : -1);
    }
    for (i = 0; i < MAXBUTTONS; i++) {
        button_t* b = &buttonlist[i];
        fprintf (f, "B,%d,%d,%ld,%d,%d,%d\n", tic, i, b->line ? (long)(b->line - lines) : -1L, (int)b->where,
                 b->btexture, b->btimer);
    }
    for (i = 0; i < MAXPLAYERS; i++) {
        player_t* p = &players[i];
        fprintf (f, "P,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d", tic, i, (int)playeringame[i], tidx (p->mo),
                 (int)p->playerstate, (int)p->cmd.forwardmove, (int)p->cmd.sidemove, (int)p->cmd.angleturn,
                 (int)p->cmd.buttons, p->viewheight, p->deltaviewheight, p->bob, p->health, p->armorpoints, p->armortype);
        for (j = 0; j < NUMPOWERS; j++) fprintf (f, ",%d", p->powers[j]);
        for (j = 0; j < NUMCARDS; j++) fprintf (f, ",%d", (int)p->cards[j]);
        fprintf (f, ",%d", (int)p->backpack);
        for (j = 0; j < MAXPLAYERS; j++) fprintf (f, ",%d", p->frags[j]);
        fprintf (f, ",%d,%d", (int)p->readyweapon, (int)p->pendingweapon);
        for (j = 0; j < NUMWEAPONS; j++) fprintf (f, ",%d", (int)p->weaponowned[j]);
        for (j = 0; j < NUMAMMO; j++) fprintf (f, ",%d", p->ammo[j]);
        for (j = 0; j < NUMAMMO; j++) fprintf (f, ",%d", p->maxammo[j]);
        fprintf (f, ",%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d", p->attackdown, p->usedown, p->cheats, p->refire,
                 p->killcount, p->itemcount, p->secretcount, p->damagecount, p->bonuscount, tidx (p->attacker),
                 p->extralight, p->fixedcolormap, p->colormap);
        for (j = 0; j < NUMPSPRITES; j++)
            fprintf (f, ",%ld,%d,%d,%d", stidx (p->psprites[j].state), p->psprites[j].tics, p->psprites[j].sx, p->psprites[j].sy);
        fprintf (f, ",%d\n", (int)p->didsecret);
    }
    fclose (f);
}

static double now (void)
{
    struct timespec ts;
    clock_gettime (CLOCK_MONOTONIC, &ts);   /* driver timing only, never read by gameplay */
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main (int argc, char** argv)
{
    char* demo = NULL; char* out = NULL; int extra_draw = -1, i, tic, checkheap = 1, nodump = 0;
    double t0, tick_time = 0, dump_time = 0;
    for (i = 1; i < argc; i++) {
        if (!strcmp (argv[i], "--wad") && i + 1 < argc) { if (numwads == MAXWADS) { fprintf (stderr, "too many --wad\n"); return 2; } add_file (argv[++i]); }
        else if (!strcmp (argv[i], "--demo") && i + 1 < argc) demo = argv[++i];
        else if (!strcmp (argv[i], "--out") && i + 1 < argc) out = argv[++i];
        else if (!strcmp (argv[i], "--zone-mb") && i + 1 < argc) stub_zone_mb = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--selftest-extra-draw") && i + 1 < argc) extra_draw = atoi (argv[++i]);
        else if (!strcmp (argv[i], "--no-checkheap")) checkheap = 0;
        else if (!strcmp (argv[i], "--no-dump")) nodump = 1;
        else { fprintf (stderr, "usage: crunner --wad IWAD --wad DEMO.lmp --demo NAME --out DIR [--zone-mb N] "
                                "[--selftest-extra-draw TIC] [--no-checkheap] [--no-dump]\n"); return 2; }
    }
    if (!numwads || !demo || (!out && !nodump)) { fprintf (stderr, "missing --wad/--demo/--out\n"); return 2; }
    if (layout_patchcount_offset () != 20 || layout_patches_offset () != 22 || layout_mappatch_size () != 10) {
        fprintf (stderr, "maptexture_t layout check FAILED (%d %d %d)\n", layout_patchcount_offset (),
                 layout_patches_offset (), layout_mappatch_size ());
        return 5;
    }
    setvbuf (stdout, NULL, _IOLBF, 0);
    Z_Init ();                                   /* d_main.c:1018 */
    W_InitMultipleFiles (wadfiles);              /* d_main.c:1021 */
    gamemode = registered;                       /* d_main.c:697-701 (doom.wad) */
    R_InitTextures ();                           /* r_data.c:656 */
    R_InitFlats ();                              /* r_data.c:658 */
    P_InitSwitchList ();                         /* p_setup.c:702 */
    P_InitPicAnims ();                           /* p_setup.c:703 */
    consoleplayer = displayplayer = 0;           /* D_CheckNetGame, d_net.c:567-592 */
    playeringame[0] = true;
    singledemo = true;                           /* d_main.c:1137 */
    G_DeferedPlayDemo (demo);                    /* d_main.c:1138 */
    G_DoPlayDemo ();                             /* G_Ticker's ga_playdemo case, g_game.c:634-636 */
    if (gamestate != GS_LEVEL || !demoplayback) { fprintf (stderr, "level did not load\n"); return 6; }
    linesel = calloc (numlines ? numlines : 1, 1);
    for (i = 0; i < numlines; i++) linesel[i] = lines[i].special != 0;
    if (!nodump) dump (out, "load.txt", -1);
    printf ("load: gamestate %d numthinkers %d prndindex %d\n", (int)gamestate, numth, prndindex);
    if (nodump) walk_thinkers ();
    for (tic = 0; ; tic++) {
        if (tic == extra_draw) P_Random ();      /* --selftest-extra-draw: test-only */
        t0 = now ();
        stub_quit_armed = 1;
        if (setjmp (stub_quit_jmp) == 0) G_Ticker ();
        stub_quit_armed = 0;
        if (stub_quit_flag) break;
        gametic++;
        tick_time += now () - t0;
        if (checkheap) Z_CheckHeap ();           /* driver check; read-only walk of the zone */
        if (!nodump) {
            char name[64];
            t0 = now ();
            snprintf (name, sizeof name, "tic_%05d.txt", tic);
            dump (out, name, tic);
            dump_time += now () - t0;
        }
    }
    printf ("summary: tics %d quit_on_marker %d gamestate %d prndindex %d checkheap %s zone_mb %d "
            "tick_seconds %.3f dump_seconds %.3f us_per_tic %.1f\n",
            tic, stub_quit_flag, (int)gamestate, prndindex, checkheap ? "every-tic-ok" : "load-only", stub_zone_mb,
            tick_time, dump_time, tic ? tick_time * 1e6 / tic : 0.0);
    return 0;
}
