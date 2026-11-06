/*
 * M4a headless genuine-engine runner: the driver (declared adapter).
 *
 * This is the M4a fork of tools/reference/m3h9c_driver.c, which stays byte-for-byte as
 * merged (frozen wire-22 evidence; tools/local/e1m1_reference.py and
 * tools/reference/m3h9c_reference.py keep building it). tools/reference/m4a_build.py
 * builds this file with the same genuine sources, adapters, stubs and flags as
 * m3h9c_build.py, plus -Wl,--wrap=P_RemoveThinker, into the contract
 * tests/reference/m4a_runner_contract.json. The fork adds exactly four things to the
 * M3h9c driver text (each appended so every earlier column keeps its position):
 *   1. H.secretexit: g_game.c's `boolean secretexit` (g_game.c:999), set by
 *      G_SecretExitLevel (g_game.c:1009-1018) and cleared by G_ExitLevel
 *      (g_game.c:1002-1006).
 *   2. P.message codes 38..40: PD_BLUEK, PD_YELLOWK, PD_REDK (d_englsh.h:128-130),
 *      assigned by EV_VerticalDoor (p_doors.c:378, 392, 405).
 *   3. A -Wl,--wrap=P_RemoveThinker observer (__wrap_P_RemoveThinker) that reads a removed
 *      non-mobj thinker's kind and sector before the genuine P_RemoveThinker
 *      (p_tick.c:80-84) overwrites its function pointer.
 *   4. R rows carry fn,sector from that observer (1 T_VerticalDoor p_doors.c:63,
 *      2 T_PlatRaise p_plats.c:52, 3 T_MoveFloor p_floor.c:209, 4 T_MoveCeiling
 *      p_ceilng.c:52, 0 other; sector -1 when unknown).
 * Everything else below is the M3h9c driver text unchanged.
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
 *   - End on the exit: G_Ticker runs its gameaction loop (g_game.c:617-652) before it reads
 *     the tic's ticcmd (g_game.c:656-669), and G_ExitLevel / G_SecretExitLevel set
 *     gameaction = ga_completed during P_Ticker (g_game.c:1002-1018, from the exit
 *     switch or line, p_switch.c:359-363). So the G_Ticker after an exit always runs
 *     G_DoCompleted (g_game.c:636-637, 1020-1141), which enters the intermission
 *     (gamestate = GS_INTERMISSION and WI_Start, g_game.c:1133-1140), presentation the
 *     runner does not model, and the demo marker is never reached. After dumping each
 *     tic the driver therefore stops if gameaction is ga_completed, ga_victory or
 *     ga_worlddone and reports end=exit with that tic; a demo that reaches its marker
 *     reports end=marker. The WI_Start stub still aborts as a safety net.
 *   - End on a reborn request: P_DeathThink sets playerstate = PST_REBORN when a dead
 *     player presses BT_USE (p_user.c:227-228). The next G_Ticker would start with its
 *     reborn loop (g_game.c:612-614), whose G_DoReborn sets gameaction = ga_loadlevel in
 *     single player (g_game.c:924-931), and its gameaction loop would reload the level
 *     (G_DoLoadLevel, g_game.c:621-623): outside the admitted domain (decision G11; Rust
 *     refuses that tic with PlayerReborn). After dumping each tic the driver therefore
 *     also stops if any player in the game is PST_REBORN and reports end=reborn with
 *     that tic.
 *   - Linker-level observers (see the block above the wrappers below): P_AddThinker,
 *     Z_Free, P_SetThingPosition and P_InitThinkers are wrapped with -Wl,--wrap; each
 *     wrapper records thinker serials, link serials, frees and removed-mobj copies in
 *     side tables and calls the genuine function unchanged. They add the T, K and RM rows.
 *   - K rows carry `adds`, the number of P_AddThinker calls made before the free (the
 *     next thinker serial at that moment), which places each free among the tic's spawns.
 *   - A pending removal (function -1, still in the list) keeps its R row; for a mobj an
 *     RR row follows with the M columns (idx = list position; bpos/spos -1, unlinked by
 *     P_RemoveMobj), so a run's last snapshot shows every pending mobj's fields.
 *   - The HU_Ticker stub records players[consoleplayer].message before clearing it as
 *     vanilla does; P rows report it as a pickup-message index.
 *   - ZeroFill: Z_Malloc is also wrapped (-Wl,--wrap=Z_Malloc) and every block it returns
 *     is zero-filled after the genuine allocation. Finding (real-E1M1 barrels run): the
 *     strict and address dumps differed only in plat_t.count and floormove_t.newspecial,
 *     zone memory the genuine code never initialises before the dump reads it. EV_DoPlat
 *     allocates the plat (p_plats.c:173) without setting count; T_PlatRaise reads count
 *     only in the waiting state (p_plats.c:120) after setting it on every transition
 *     into waiting (75, 84, 113). EV_DoFloor allocates the floor (p_floor.c:282) and
 *     sets newspecial only for lowerAndChange when a neighbour with the destination
 *     height exists (p_floor.c:422, 433) and donutRaise sets it (p_spec.c:1203); T_MoveFloor
 *     reads it only for those two types (p_floor.c:231, 242). Vanilla demos play back
 *     identically across machines, so gameplay never depends on uninitialised zone
 *     contents; the fill fixes only values undefined in the original and keeps the
 *     three-variant identity check meaningful. One genuine path does read undefined
 *     memory: lowerAndChange with no neighbour at the destination height reads
 *     newspecial and texture uninitialised (p_floor.c:242-243); the fill makes them 0
 *     there (not reachable on E1M1, which has no lowerAndChange line).
 *   - After every tic the driver calls the genuine Z_CheckHeap (z_zone.c:399-420), a
 *     read-only walk of the zone block list that calls I_Error on corruption; vanilla
 *     calls it only in G_DoLoadLevel (g_game.c:488). --no-checkheap disables it.
 *   - --selftest-extra-draw TIC (test only) calls the public P_Random () once before
 *     G_Ticker of tic TIC; it never touches gameplay code text.
 *   - --zone-mb N sets the zone size (vanilla mb_used = 6, i_system.c:51).
 *
 * Columns added for M4a (appended at the end of their rows, so every earlier column keeps
 * its position and name; readers select columns by name):
 *   - H.secretexit: g_game.c's `boolean secretexit` (g_game.c:999), set by
 *     G_SecretExitLevel (g_game.c:1009-1018, line special 51, p_switch.c:431-435) and
 *     cleared by G_ExitLevel (g_game.c:1002-1006); it is what tells a secret exit
 *     (end=exit, secretexit 1) from a normal one (secretexit 0) on the stopping tic.
 *   - P.message codes 38..40: the three locked-door strings PD_BLUEK, PD_YELLOWK and
 *     PD_REDK (d_englsh.h:128-130) that EV_VerticalDoor assigns to player->message
 *     (p_doors.c:378, 392, 405), appended after GOTSHOTGUN2 so codes 1..37 are unchanged.
 * Rows that already carried what M4a reads (no change): S.floorpic (sector_t.floorpic,
 * written by EV_DoPlat raiseToNearestAndChange p_plats.c:185-195 and by the donut's
 * T_MoveFloor pastdest p_floor.c:230-232), S.lightlevel (EV_LightTurnOn p_lights.c:271-306),
 * P.pw0..pw5, card0..card5, backpack, ammo0..3, maxammo0..3 (player_t, d_player.h), and
 * M.flags (MF_SHADOW, p_inter.c:300-305; MF_JUSTATTACKED, p_pspr.c:542).
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
#include "dstrings.h"

/* genuine definitions without header declarations */
void R_InitTextures (void);
void R_InitFlats (void);
void T_FireFlicker (fireflicker_t* flick);   /* p_lights.c:46 */
extern short numlinespecials;
extern boolean singledemo;
extern int rndindex, prndindex;
extern gameaction_t gameaction;
extern boolean secretexit;               /* g_game.c:999 */
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
extern char* stub_hu_message;   /* players[consoleplayer].message as HU_Ticker saw and cleared it */
extern int stub_hu_ran;

/* ---------------- linker-level observers ----------------
 * Linked with -Wl,--wrap=P_AddThinker,--wrap=Z_Free,--wrap=P_SetThingPosition,
 * --wrap=P_InitThinkers (m3h9c_build.py LINK_FLAGS): every call from another object
 * file reaches __wrap_X, which records into the side tables below (never game memory,
 * never the RNG) and then calls the genuine __real_X with the same arguments, returning
 * its result. The one wrapper that writes game memory is ZeroFill's __wrap_Z_Malloc
 * (declared in the header: it zero-fills each new block, nothing else). Every gameplay call of the four is cross-object (p_tick.c defines
 * P_AddThinker/P_InitThinkers, p_maputl.c P_SetThingPosition, z_zone.c Z_Free); the only
 * unwrapped calls are z_zone.c's internal Z_Free calls (Z_FreeTags z_zone.c:316 and the
 * purge in Z_Malloc), which never free a live thinker except Z_FreeTags (PU_LEVEL) in
 * P_SetupLevel (p_setup.c:618), immediately followed by P_InitThinkers (p_setup.c:622),
 * whose wrapper drops every table entry (K rows with seq -1).
 * Thinker lifecycle (genuine): P_AddThinker appends (p_tick.c:65-71); P_RemoveThinker
 * only sets function.acv = -1 (p_tick.c:80-84; P_RemoveMobj first unlinks the mobj with
 * P_UnsetThingPosition and stops its sound, p_mobj.c:546-570); the thinker stays in the
 * list until P_RunThinkers reaches it, unlinks it and calls Z_Free (p_tick.c:101-122). The
 * Z_Free wrapper copies a mobj (function -1, added as a P_MobjThinker: P_SpawnMobj sets
 * the function before P_AddThinker, p_mobj.c:529-531) before the genuine free. */
void __real_P_AddThinker (thinker_t* thinker);
void __real_Z_Free (void* ptr);
void __real_P_SetThingPosition (mobj_t* thing);
void __real_P_InitThinkers (void);
void* __real_Z_Malloc (int size, int tag, void* user);

/* ZeroFill (declared, see the header): the genuine allocation, then memset to 0. */
void* __wrap_Z_Malloc (int size, int tag, void* user)
{
    void* block = __real_Z_Malloc (size, tag, user);
    memset (block, 0, size);
    return block;
}

typedef struct { void* p; int serial, linkserial, mobj, rfn; long rsector; } obs_t;
#define OBS_CAP 65536                     /* power of two; far above any thinker count */
static obs_t obs[OBS_CAP];               /* open addressing; p == NULL empty, p == OBS_DEAD deleted */
static char obs_dead_marker;
#define OBS_DEAD ((void*)&obs_dead_marker)
static int obs_used, next_serial, next_linkserial;
typedef struct { int serial, seq, adds; } freed_t;
static freed_t* freed; static int nfreed, capfreed, freeseq;
typedef struct { int serial; mobj_t m; } removed_t;
static removed_t* removed; static int nremoved, capremoved;
static int unknown_messages, observer_misses;

static unsigned obs_hash (void* p) { unsigned long v = (unsigned long)p >> 3; return (unsigned)(v ^ (v >> 17)) & (OBS_CAP - 1); }
static obs_t* obs_find (void* p)
{
    unsigned h = obs_hash (p);
    while (obs[h].p) { if (obs[h].p == p) return &obs[h]; h = (h + 1) & (OBS_CAP - 1); }
    return NULL;
}
static void obs_rehash (void)
{
    static obs_t old[OBS_CAP];
    int i;
    memcpy (old, obs, sizeof obs); memset (obs, 0, sizeof obs); obs_used = 0;
    for (i = 0; i < OBS_CAP; i++)
        if (old[i].p && old[i].p != OBS_DEAD) {
            unsigned h = obs_hash (old[i].p);
            while (obs[h].p) h = (h + 1) & (OBS_CAP - 1);
            obs[h] = old[i]; obs_used++;
        }
    if (obs_used >= OBS_CAP / 2) { fprintf (stderr, "observer table full\n"); abort (); }
}
static obs_t* obs_insert (void* p)        /* p is not in the table */
{
    unsigned h;
    if (obs_used >= OBS_CAP / 2) obs_rehash ();   /* obs_used counts live and deleted slots */
    h = obs_hash (p);
    while (obs[h].p && obs[h].p != OBS_DEAD) h = (h + 1) & (OBS_CAP - 1);
    if (!obs[h].p) obs_used++;
    obs[h].p = p; obs[h].serial = -1; obs[h].linkserial = -1; obs[h].mobj = 0; obs[h].rfn = 0; obs[h].rsector = -1;
    return &obs[h];
}
static void note_freed (int serial, int seq, int adds)
{
    if (nfreed == capfreed) { capfreed = capfreed ? capfreed * 2 : 64; freed = realloc (freed, capfreed * sizeof *freed); }
    freed[nfreed].serial = serial; freed[nfreed].seq = seq; freed[nfreed].adds = adds; nfreed++;
}
void __wrap_P_AddThinker (thinker_t* thinker)
{
    obs_t* e = obs_find (thinker);
    if (!e) e = obs_insert (thinker);
    e->serial = next_serial++;
    e->mobj = thinker->function.acp1 == (actionf_p1)P_MobjThinker;
    __real_P_AddThinker (thinker);
}
void __wrap_P_SetThingPosition (mobj_t* thing)
{
    obs_t* e = obs_find (thing);
    if (!e) e = obs_insert (thing);      /* a thing positioned before P_AddThinker (P_SpawnMobj, p_mobj.c:517) */
    e->linkserial = next_linkserial++;
    __real_P_SetThingPosition (thing);
}
void __wrap_Z_Free (void* ptr)
{
    obs_t* e = ptr ? obs_find (ptr) : NULL;
    if (e) {
        if (e->mobj && ((thinker_t*)ptr)->function.acv == (actionf_v)(-1)) {
            if (nremoved == capremoved) { capremoved = capremoved ? capremoved * 2 : 16; removed = realloc (removed, capremoved * sizeof *removed); }
            removed[nremoved].serial = e->serial;
            memcpy (&removed[nremoved].m, ptr, sizeof (mobj_t));
            nremoved++;
        }
        note_freed (e->serial, freeseq++, next_serial);
        e->p = OBS_DEAD;
    }
    __real_Z_Free (ptr);
}
static int cmp_serial (const void* a, const void* b) { return ((const obs_t*)a)->serial - ((const obs_t*)b)->serial; }
void __wrap_P_InitThinkers (void)
{
    int i, n = 0; obs_t* live = malloc (sizeof (obs_t) * (obs_used ? obs_used : 1));
    for (i = 0; i < OBS_CAP; i++) if (obs[i].p && obs[i].p != OBS_DEAD && obs[i].serial >= 0) live[n++] = obs[i];
    qsort (live, n, sizeof *live, cmp_serial);
    for (i = 0; i < n; i++) note_freed (live[i].serial, -1, next_serial);
    free (live);
    memset (obs, 0, sizeof obs); obs_used = 0;
    __real_P_InitThinkers ();
}
static int serial_of (void* p) { obs_t* e = obs_find (p); if (!e || e->serial < 0) { observer_misses++; return -1; } return e->serial; }
static int linkserial_of (void* p) { obs_t* e = obs_find (p); return e ? e->linkserial : -1; }

/* d_englsh.h GOTARMOR .. GOTSHOTGUN2 in header order, then (M4a) the locked-door strings
 * PD_BLUEK, PD_YELLOWK, PD_REDK (d_englsh.h:128-130; p_doors.c:378, 392, 405); P rows
 * report 1 + the index. */
static const char* const PICKUP_MESSAGES[] = {
    GOTARMOR, GOTMEGA, GOTHTHBONUS, GOTARMBONUS, GOTSTIM, GOTMEDINEED, GOTMEDIKIT, GOTSUPER,
    GOTBLUECARD, GOTYELWCARD, GOTREDCARD, GOTBLUESKUL, GOTYELWSKUL, GOTREDSKULL,
    GOTINVUL, GOTBERSERK, GOTINVIS, GOTSUIT, GOTMAP, GOTVISOR, GOTMSPHERE,
    GOTCLIP, GOTCLIPBOX, GOTROCKET, GOTROCKBOX, GOTCELL, GOTCELLBOX, GOTSHELLS, GOTSHELLBOX, GOTBACKPACK,
    GOTBFG9000, GOTCHAINGUN, GOTCHAINSAW, GOTLAUNCHER, GOTPLASMA, GOTSHOTGUN, GOTSHOTGUN2,
    PD_BLUEK, PD_YELLOWK, PD_REDK };
static int message_index (const char* m)
{
    unsigned i;
    if (!m) return 0;
    for (i = 0; i < sizeof PICKUP_MESSAGES / sizeof *PICKUP_MESSAGES; i++)
        if (m == PICKUP_MESSAGES[i] || !strcmp (m, PICKUP_MESSAGES[i])) return (int)i + 1;
    unknown_messages++;
    return -1;
}

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

void __real_P_RemoveThinker (thinker_t* thinker);
/* P_RemoveThinker observer: the kind (1 vldoor_t, 2 plat_t, 3 floormove_t, 4 ceiling_t, 0 other)
   and sector of a non-mobj thinker, read before the genuine call overwrites its function. */
void __wrap_P_RemoveThinker (thinker_t* thinker)
{
    obs_t* e = obs_find (thinker);
    if (e && !e->mobj) {
        actionf_p1 fn = thinker->function.acp1;
        if (fn == (actionf_p1)T_VerticalDoor) { e->rfn = 1; e->rsector = sidx (((vldoor_t*)thinker)->sector); }
        else if (fn == (actionf_p1)T_PlatRaise) { e->rfn = 2; e->rsector = sidx (((plat_t*)thinker)->sector); }
        else if (fn == (actionf_p1)T_MoveFloor) { e->rfn = 3; e->rsector = sidx (((floormove_t*)thinker)->sector); }
        else if (fn == (actionf_p1)T_MoveCeiling) { e->rfn = 4; e->rsector = sidx (((ceiling_t*)thinker)->sector); }
    }
    __real_P_RemoveThinker (thinker);
}

static const char* COLUMNS =
 "#columns,H,tic,leveltime,gametic,prndindex,rndindex,totalkills,totalitems,totalsecret,gameaction,gamestate,numthinkers,numlinespecials,gameskill,secretexit\n"
 "#columns,M,tic,idx,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,R,tic,idx,fn,sector\n"
 "#columns,RR,tic,idx,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,DR,tic,idx,type,sector,topheight,speed,direction,topwait,topcountdown\n"
 "#columns,PL,tic,idx,running,sector,speed,low,high,wait,count,status,oldstatus,crush,tag,type\n"
 "#columns,FL,tic,idx,type,crush,sector,direction,newspecial,texture,floordestheight,speed\n"
 "#columns,CE,tic,idx,running,type,sector,bottomheight,topheight,speed,crush,direction,tag,olddirection\n"
 "#columns,LF,tic,idx,sector,count,maxlight,minlight\n"
 "#columns,LL,tic,idx,sector,count,maxlight,minlight,maxtime,mintime\n"
 "#columns,LS,tic,idx,sector,count,minlight,maxlight,darktime,brighttime\n"
 "#columns,LG,tic,idx,sector,minlight,maxlight,direction\n"
 "#columns,X,tic,idx\n"
 "#columns,T,tic,idx,serial,linkserial\n"
 "#columns,K,tic,serial,seq,adds\n"
 "#columns,RM,tic,serial,type,x,y,z,momx,momy,momz,health,state,tics,flags,height,radius,reactiontime,threshold,target,lastlook,floorz,ceilingz,subsector,angle,sprite,frame,movedir,movecount,tracer,sp_x,sp_y,sp_angle,sp_type,sp_options,player,bx,by,bpos,sector,spos\n"
 "#columns,S,tic,idx,floorheight,ceilingheight,floorpic,ceilingpic,lightlevel,special,tag,soundtraversed,soundtarget,specialdata,linecount\n"
 "#columns,L,tic,idx,special,tag,flags,s0top,s0mid,s0bottom,s0textureoffset,s0rowoffset,s1top,s1mid,s1bottom\n"
 "#columns,B,tic,idx,line,where,btexture,btimer\n"
 "#columns,P,tic,idx,ingame,mo,playerstate,cmd_forward,cmd_side,cmd_turn,cmd_buttons,viewheight,deltaviewheight,bob,health,armorpoints,armortype,"
 "pw0,pw1,pw2,pw3,pw4,pw5,card0,card1,card2,card3,card4,card5,backpack,frag0,frag1,frag2,frag3,readyweapon,pendingweapon,"
 "wo0,wo1,wo2,wo3,wo4,wo5,wo6,wo7,wo8,ammo0,ammo1,ammo2,ammo3,maxammo0,maxammo1,maxammo2,maxammo3,attackdown,usedown,cheats,refire,"
 "killcount,itemcount,secretcount,damagecount,bonuscount,attacker,extralight,fixedcolormap,colormap,"
 "ps0state,ps0tics,ps0sx,ps0sy,ps1state,ps1tics,ps1sx,ps1sy,didsecret,message\n";

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

/* One M row (kind "M", id = thinker-list index, linked) or RM row (kind "RM", id = serial,
 * unlinked by P_RemoveMobj: bpos and spos -1). */
static void print_mobj (FILE* f, const char* kind, int tic, int id, mobj_t* m, int linked)
{
    int bx = -1, by = -1, bp = -1, sp = -1;
    if (linked) { block_pos (m, &bx, &by, &bp); sp = sector_pos (m); }
    fprintf (f, "%s,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%u,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d\n",
        kind, tic, id, (int)m->type, m->x, m->y, m->z, m->momx, m->momy, m->momz, m->health, stidx (m->state), m->tics,
        m->flags, m->height, m->radius, m->reactiontime, m->threshold, tidx (m->target), m->lastlook,
        m->floorz, m->ceilingz, m->subsector ? (long)(m->subsector - subsectors) : -1L, (unsigned)m->angle,
        (int)m->sprite, m->frame, m->movedir, m->movecount, tidx (m->tracer),
        m->spawnpoint.x, m->spawnpoint.y, m->spawnpoint.angle, m->spawnpoint.type, m->spawnpoint.options,
        m->player ? (int)(m->player - players) : -1, bx, by, bp,
        ((m->flags & MF_NOSECTOR) || !m->subsector) ? -1L : sidx (m->subsector->sector), sp);
}

static void dump (const char* dir, const char* name, int tic)
{
    char path[1024]; FILE* f; int i, j;
    snprintf (path, sizeof path, "%s/%s", dir, name);
    f = fopen (path, "w");
    if (!f) { perror (path); exit (4); }
    walk_thinkers ();
    fputs (COLUMNS, f);
    fprintf (f, "H,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, leveltime, gametic, prndindex, rndindex,
             totalkills, totalitems, totalsecret, (int)gameaction, (int)gamestate, numth, (int)numlinespecials,
             (int)gameskill, (int)secretexit);
    for (i = 0; i < numth; i++) {
        thinker_t* t = order[i];
        actionf_p1 fn = t->function.acp1;
        if (fn == (actionf_p1)P_MobjThinker) {
            print_mobj (f, "M", tic, i, (mobj_t*)t, 1);
        } else if (t->function.acv == (actionf_v)(-1)) {
            { obs_t* e = obs_find (t);
              fprintf (f, "R,%d,%d,%d,%ld\n", tic, i, e ? e->rfn : 0, e ? e->rsector : -1L);
              if (e && e->mobj) print_mobj (f, "RR", tic, i, (mobj_t*)t, 0); }
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
    for (i = 0; i < numth; i++)
        fprintf (f, "T,%d,%d,%d,%d\n", tic, i, serial_of (order[i]), linkserial_of (order[i]));
    for (i = 0; i < nfreed; i++) fprintf (f, "K,%d,%d,%d,%d\n", tic, freed[i].serial, freed[i].seq, freed[i].adds);
    for (i = 0; i < nremoved; i++) print_mobj (f, "RM", tic, removed[i].serial, &removed[i].m, 0);
    nfreed = nremoved = freeseq = 0;
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
        fprintf (f, ",%d,%d\n", (int)p->didsecret,
                 message_index (i == consoleplayer && stub_hu_ran ? stub_hu_message : p->message));
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
    char* demo = NULL; char* out = NULL; int extra_draw = -1, i, tic, checkheap = 1, nodump = 0, exited = 0, reborn = 0, ran;
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
        if (gameaction == ga_completed || gameaction == ga_victory || gameaction == ga_worlddone) {
            exited = 1;                          /* the next G_Ticker would enter the intermission */
            break;
        }
        for (i = 0; i < MAXPLAYERS; i++)
            if (playeringame[i] && players[i].playerstate == PST_REBORN) reborn = 1;
        if (reborn) break;                       /* the next G_Ticker would reload the level */
    }
    ran = exited || reborn ? tic + 1 : tic;      /* tics whose G_Ticker completed and were dumped */
    printf ("summary: tics %d end=%s end_tic %d gamestate %d prndindex %d checkheap %s zone_mb %d "
            "observer_misses %d unknown_messages %d tick_seconds %.3f dump_seconds %.3f us_per_tic %.1f\n",
            ran, exited ? "exit" : reborn ? "reborn" : "marker", tic, (int)gamestate, prndindex,
            checkheap ? "every-tic-ok" : "load-only", stub_zone_mb, observer_misses, unknown_messages,
            tick_time, dump_time, ran ? tick_time * 1e6 / ran : 0.0);
    return 0;
}
