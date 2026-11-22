/* ---------------- M6a: the session past the level boundary ----------------
 * tools/reference/m6a_build.py includes this file into the M6a driver, which is the frozen
 * tools/reference/m5c_driver.c text with the exact-once patches listed in m6a_build.PATCHES
 * (docs/M6A_REFERENCE_INVESTIGATION.md section 3.1). It is the new code of the M6a runner:
 *   - G_Ticker runs every tic and the driver never stops at a level boundary: the genuine gameaction
 *     loop (g_game.c:617-650) runs G_DoCompleted, WI_Start, G_DoWorldDone, G_DoReborn and
 *     F_StartFinale as the original does; the run ends at the demo marker, at a refusal or at an
 *     I_Error, as the M5c runner's does;
 *   - the D_Display mirror for GS_INTERMISSION and GS_FINALE (d_main.c:191-344): WI_Drawer or
 *     F_Drawer, I_SetPalette (PLAYPAL) on entering a state other than the level (d_main.c:274-275),
 *     the wipe when gamestate != wipegamestate (d_main.c:223-229, 320-344), and a G row in the frame
 *     stream with the digest of the whole frame;
 *   - level-change bookkeeping through a P_SetupLevel observer (-Wl,--wrap, called from G_DoLoadLevel,
 *     g_game.c:484): the per-level driver tables (lines with a special, mapped lines) are rebuilt
 *     right after each load, before the new level's tic 0, the same-map reborn reload included;
 *   - the M_Random observer gains an intermission bucket (draws inside WI_Start and WI_Ticker: the
 *     animation draws of WI_initAnimatedBack, wi_stuff.c:523) and a finale bucket (F_StartFinale and
 *     F_Ticker), so "other" stays 0;
 *   - the declared game mode (--game-mode registered|retail, required, owner's ruling U4) in place of
 *     IdentifyVersion (d_main.c:563-717, which decides by file name only): the runner sets gamemode
 *     from the flag and refuses when the IWAD's file name is one IdentifyVersion knows and names
 *     another mode (doomu.wad retail, d_main.c:690-695; doom.wad registered, 697-702; the shareware
 *     and commercial names refused); a synthetic IWAD under any other name runs in the declared mode;
 *   - the session stream (--session-out FILE, DOOM-M6A-SESSION-v1): per tic the game state and
 *     action, map, leveltime, gametic, levelstarttic, both random indices and every draw bucket (G),
 *     the console player's carried fields (C), in the intermission wminfo (I) and the wi_stuff.c
 *     statics read through the accessor unit tools/reference/m6a_wi_access.c (J), in the finale its
 *     stage and count (F), and an L row per load;
 *   - the melt hook (--melt-clock counter|unit): counter is the M5c declared clock (the I_GetTime
 *     counter stub: 2 tics on the first step, then 1); unit passes ticks = 1 to every wipe_ScreenWipe
 *     call. Either way a V row per step (the M5c row) and a Y row per melt (its start and end screens
 *     and the palette set when it starts) are written. Evidence only; no claim rests on them yet. */
#include "wi_stuff.h"
#include "f_finale.h"
extern int levelstarttic;                         /* g_game.c:124 */
extern int finalestage, finalecount;              /* f_finale.c:52, 54 */
extern char* finaleflat; extern char* finaletext; /* f_finale.c:85-86 */
extern char* e1text; extern char* e2text; extern char* e3text; extern char* e4text;   /* f_finale.c:59-62 */
#define M6A_WI_NV 40
int m6a_wi_statics (int* v);                      /* tools/reference/m6a_wi_access.c */

static int m6a_mode = -1;                         /* --game-mode: registered | retail */
static FILE* m6a_session_out;
static int m6a_loads, m6a_wipes;

static const char* M6A_SESSION_COLUMNS =
 "#schema,DOOM-M6A-SESSION-v1\n"
 "#columns,L,tic,episode,map,skill,levelstarttic,prndindex_before,prndindex_after,rndindex_before,rndindex_after\n"
 "#columns,G,tic,gamestate,gameaction,episode,map,leveltime,gametic,levelstarttic,prndindex,rndindex,snd_draws,st_draws,wipe_draws,wi_draws,fin_draws,other_draws,wipegamestate,secretexit,paused,loads,wipes\n"
 "#columns,C,tic,playerstate,health,armorpoints,armortype,readyweapon,pendingweapon,weaponowned,ammo0,ammo1,ammo2,ammo3,maxammo0,maxammo1,maxammo2,maxammo3,backpack,cards,pw0,pw1,pw2,pw3,pw4,pw5,cheats,viewheight,deltaviewheight,attackdown,usedown,didsecret,killcount,itemcount,secretcount,refire,damagecount,bonuscount\n"
 "#columns,I,tic,epsd,didsecret,last,next,maxkills,maxitems,maxsecret,maxfrags,partime,pnum,in,skills,sitems,ssecret,stime,score\n"
 "#columns,J,tic,state,acceleratestage,me,cnt,bcnt,firstrefresh,sp_state,cnt_kills,cnt_items,cnt_secret,cnt_time,cnt_par,cnt_pause,snl_pointeron,wbs_is_wminfo,nanims,anim_ctr,anim_nexttic\n"
 "#columns,F,tic,finalestage,finalecount,flat,text\n"
 "#columns,Z,totals\n";
static const char* M6A_FRAME_COLUMNS =
 "#schema-m6a,DOOM-M6A-FRAMES-v1\n"
 "#columns,G,tic,gamestate,sha256,palette,palette_set,rndindex,skipped_patches,beyond_rows,lump_overrun\n"
 "#columns,Y,tic,wipe,gamestate,start_sha256,end_sha256,palette,palette_set,clock\n";
static const char* m6a_mode_name (void) { return m6a_mode == retail ? "retail" : m6a_mode == registered ? "registered" : "undeclared"; }

/* Options of the M6a runner, offered before the M5c driver's usage message. */
static int m6a_option (int argc, char** argv, int* i)
{
    const char* a = argv[*i];
    if (*i + 1 >= argc) return 0;
    if (!strcmp (a, "--game-mode")) {
        a = argv[++*i];
        if (!strcmp (a, "registered")) m6a_mode = registered;
        else if (!strcmp (a, "retail")) m6a_mode = retail;
        else { fprintf (stderr, "--game-mode registered|retail\n"); exit (2); }
        return 1;
    }
    if (!strcmp (a, "--melt-clock")) {
        a = argv[++*i];
        if (strcmp (a, "counter") && strcmp (a, "unit")) { fprintf (stderr, "--melt-clock counter|unit\n"); exit (2); }
        m6a_melt_unit = !strcmp (a, "unit");
        return 1;
    }
    if (!strcmp (a, "--session-out")) {
        if (!(m6a_session_out = fopen (argv[++*i], "w"))) { perror (argv[*i]); exit (4); }
        return 1;
    }
    return 0;
}

/* The stream headers; called after the M5c headers are written, before anything is loaded. */
static void m6a_begin_streams (void)
{
    if (frames_out) { fputs (M6A_FRAME_COLUMNS, frames_out);
                      fprintf (frames_out, "#config-m6a,game_mode,%s,melt_clock,%s\n", m6a_mode_name (), m6a_melt_unit ? "unit" : "counter"); }
    if (sound_out) fprintf (sound_out, "#config-m6a,game_mode,%s,level_row,LV,tic,episode,map\n", m6a_mode_name ());
    if (m6a_session_out) {
        fputs (M6A_SESSION_COLUMNS, m6a_session_out);
        fprintf (m6a_session_out, "#config,game_mode,%s,melt_clock,%s,wipe,%s,sound,%s,render,%s,screenblocks,%d\n", m6a_mode_name (),
                 m6a_melt_unit ? "unit" : "counter", wipe_mode ? "genuine" : "none", sound_mode ? "on" : "off", render_mode ? "every" : "none", screenblocks);
    }
}

/* The declared game mode (configuration adapter for IdentifyVersion, d_main.c:563-717). Returns 0
 * when the mode is set, 1 when the run is refused. The file-name test is case-insensitive, which is
 * stricter than IdentifyVersion's access () on a case-sensitive file system (it would not find
 * DOOM.WAD at all): an IWAD named for one mode cannot be run in the other. */
static int m6a_set_game_mode (void)
{
    static const char* const known[] = { "doomu.wad", "doom.wad", "doom1.wad", "doom2.wad", "doom2f.wad", "plutonia.wad", "tnt.wad" };
    const char* base; char low[32]; int i, k = -1, named = -1;
    if (m6a_mode < 0) { fprintf (stderr, "configuration refused: --game-mode registered|retail is required (the mode is declared, owner's ruling U4)\n"); return 1; }
    base = strrchr (wadfiles[0], '/'); base = base ? base + 1 : wadfiles[0];
    for (i = 0; base[i] && i < 31; i++) low[i] = (base[i] >= 'A' && base[i] <= 'Z') ? base[i] - 'A' + 'a' : base[i];
    low[i] = 0;
    for (i = 0; i < (int)(sizeof known / sizeof *known); i++) if (!strcmp (low, known[i])) k = i;
    if (k == 0) named = retail;                   /* d_main.c:690-695 */
    else if (k == 1) named = registered;          /* d_main.c:697-702 */
    else if (k >= 2) {
        fprintf (stderr, "configuration refused: the IWAD %s names a shareware or commercial game, which this runner does not admit\n", base);
        return 1;
    }
    if (named >= 0 && named != m6a_mode) {
        fprintf (stderr, "configuration refused: --game-mode %s but the IWAD file is named %s, which IdentifyVersion runs as %s; "
                 "run a scratch copy named for the mode (doomu.wad retail, doom.wad registered)\n", m6a_mode_name (), base,
                 named == retail ? "retail" : "registered");
        return 1;
    }
    gamemode = m6a_mode;
    printf ("mode: %s (declared by --game-mode; IWAD file %s %s)\n", m6a_mode_name (), base,
            named >= 0 ? "agrees, as IdentifyVersion names it" : "is not a name IdentifyVersion knows");
    return 0;
}

/* P_SetupLevel (p_setup.c:583-693), called only by G_DoLoadLevel (g_game.c:484). After every load
 * but the first (which the M5c driver text handles after command_source_begin), the per-level driver
 * tables are rebuilt before the new level's first P_Ticker. */
void __real_P_SetupLevel (int episode, int map, int playermask, skill_t skill);
void __wrap_P_SetupLevel (int episode, int map, int playermask, skill_t skill)
{
    int p0 = prndindex, r0 = rndindex, i;
    if (m6a_loads && sound_out && sound_mode) fprintf (sound_out, "LV,%d,%d,%d\n", snd_tic, episode, map);
    __real_P_SetupLevel (episode, map, playermask, skill);
    if (m6a_loads) {
        free (mapped_before); free (linesel);
        mapped_before = calloc (numlines ? numlines : 1, 1);
        linesel = calloc (numlines ? numlines : 1, 1);
        for (i = 0; i < numlines; i++) linesel[i] = lines[i].special != 0;
    }
    m6a_loads++;
    if (m6a_session_out) { fprintf (m6a_session_out, "L,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", snd_tic, episode, map, (int)skill, levelstarttic,
                                    p0, prndindex, r0, rndindex); fflush (m6a_session_out); }
}

/* The intermission and finale entry points, called from G_Ticker and G_DoCompleted (g_game.c:639-641,
 * 736-742, 1140): the M_Random draws inside them are counted in their own buckets. */
void __real_WI_Start (wbstartstruct_t* wb);
void __wrap_WI_Start (wbstartstruct_t* wb) { m6a_in_wi = 1; __real_WI_Start (wb); m6a_in_wi = 0; }
void __real_WI_Ticker (void);
void __wrap_WI_Ticker (void) { m6a_in_wi = 1; __real_WI_Ticker (); m6a_in_wi = 0; }
void __real_F_StartFinale (void);
void __wrap_F_StartFinale (void) { m6a_in_fin = 1; __real_F_StartFinale (); m6a_in_fin = 0; }
void __real_F_Ticker (void);
void __wrap_F_Ticker (void) { m6a_in_fin = 1; __real_F_Ticker (); m6a_in_fin = 0; }

/* The Y row: called by the melt after wipe_EndScreen (d_main.c:327), so screens[2] holds the start
 * screen (wipe_StartScreen, f_wipe.c:237-245) and screens[3] the end screen (f_wipe.c:249-258). */
static void m6a_melt_begin (int tic)
{
    char a[65], b[65];
    m6a_wipes++;
    if (!frames_out) return;
    sha256_hex (screens[2], SCREENWIDTH*SCREENHEIGHT, a);
    sha256_hex (screens[3], SCREENWIDTH*SCREENHEIGHT, b);
    fprintf (frames_out, "Y,%d,%d,%d,%s,%s,%d,%d,%s\n", tic, m6a_wipes, (int)gamestate, a, b, cur_palette, palette_set_this_frame,
             m6a_melt_unit ? "unit" : "counter");
}

/* D_Display (d_main.c:191-344) for GS_INTERMISSION and GS_FINALE. Shares the mirror's statics with
 * the level display (one D_Display, one oldgamestate). Runs with --render none as well: vanilla always
 * displays, and WI_drawNoState writes snl_pointeron (wi_stuff.c:814), which only drawing reads. */
static void m6a_display_other (int tic)
{
    int wipe; char sha[65];
    cur_tic = tic;
    palette_set_this_frame = -1; frame_skipped = frame_beyond = frame_lump_overrun = bar_undef_px = 0;
    if (nodrawers) return;                                    /* d_main.c:209-210 (false) */
    if (setsizeneeded) {                                      /* d_main.c:215-220 */
        R_ExecuteSetViewSize ();
        dd_oldgamestate = -1;
        dd_borderdrawcount = 3;
    }
    wipe = gamestate != wipegamestate;                        /* d_main.c:223-229 */
    if (wipe && wipe_mode) wipe_StartScreen (0, 0, SCREENWIDTH, SCREENHEIGHT);
    if (gamestate == GS_INTERMISSION) WI_Drawer ();           /* d_main.c:250-252 */
    else if (gamestate == GS_FINALE) F_Drawer ();             /* d_main.c:254-256 */
    else { fprintf (stderr, "display: gamestate %d\n", (int)gamestate); abort (); }
    /* I_UpdateNoBlit (d_main.c:263) is presentation; R_RenderPlayerView and HU_Drawer are GS_LEVEL only (266-271) */
    if (gamestate != dd_oldgamestate && gamestate != GS_LEVEL) I_SetPalette (W_CacheLumpName ("PLAYPAL", PU_CACHE));   /* d_main.c:274-275 */
    /* d_main.c:278-295: GS_LEVEL only */
    dd_menuactivestate = menuactive;                          /* d_main.c:297-300 */
    dd_viewactivestate = viewactive;
    dd_inhelpscreensstate = inhelpscreens;
    dd_oldgamestate = wipegamestate = gamestate;
    if (paused) { fprintf (stderr, "display: paused\n"); abort (); }   /* d_main.c:303-312 */
    NetUpdate ();                                             /* d_main.c:317; M_Drawer (316): no menu */
    if (wipe && wipe_mode) m5c_melt (tic);                    /* d_main.c:320-344 */
    total_skipped += frame_skipped; total_beyond += frame_beyond; total_lump_overrun += frame_lump_overrun;
    if (frames_out) {
        sha256_hex (screens[0], SCREENWIDTH*SCREENHEIGHT, sha);
        fprintf (frames_out, "G,%d,%d,%s,%d,%d,%d,%d,%d,%d\n", tic, (int)gamestate, sha, cur_palette, palette_set_this_frame, rndindex,
                 frame_skipped, frame_beyond, frame_lump_overrun);
        fflush (frames_out);
    }
}

/* The session rows of a tic, after its G_Ticker, S_UpdateSounds and display. */
static void m6a_tic_end (int tic)
{
    player_t* p = &players[consoleplayer]; FILE* f = m6a_session_out; int i, w = 0, c = 0;
    if (!f) return;
    fprintf (f, "G,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, (int)gamestate, (int)gameaction, gameepisode, gamemap,
             leveltime, gametic, levelstarttic, prndindex, rndindex, snd_draws, st_draws, wipe_draws, m6a_wi_draws, m6a_fin_draws, other_draws,
             (int)wipegamestate, (int)secretexit, (int)paused, m6a_loads, m6a_wipes);
    for (i = 0; i < NUMWEAPONS; i++) w |= (p->weaponowned[i] ? 1 : 0) << i;
    for (i = 0; i < NUMCARDS; i++) c |= (p->cards[i] ? 1 : 0) << i;
    fprintf (f, "C,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic,
             (int)p->playerstate, p->health, p->armorpoints, p->armortype, (int)p->readyweapon, (int)p->pendingweapon, w,
             p->ammo[0], p->ammo[1], p->ammo[2], p->ammo[3], p->maxammo[0], p->maxammo[1], p->maxammo[2], p->maxammo[3], (int)p->backpack, c,
             p->powers[0], p->powers[1], p->powers[2], p->powers[3], p->powers[4], p->powers[5], p->cheats, p->viewheight, p->deltaviewheight,
             p->attackdown, p->usedown, (int)p->didsecret, p->killcount, p->itemcount, p->secretcount, p->refire, p->damagecount, p->bonuscount);
    if (gamestate == GS_INTERMISSION) {
        wbplayerstruct_t* q = &wminfo.plyr[consoleplayer]; int v[M6A_WI_NV], n = m6a_wi_statics (v), k;
        fprintf (f, "I,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n", tic, wminfo.epsd, (int)wminfo.didsecret, wminfo.last, wminfo.next,
                 wminfo.maxkills, wminfo.maxitems, wminfo.maxsecret, wminfo.maxfrags, wminfo.partime, wminfo.pnum, (int)q->in, q->skills,
                 q->sitems, q->ssecret, q->stime, q->score);
        if (n != M6A_WI_NV) { fprintf (stderr, "m6a_wi_statics: %d values\n", n); abort (); }
        fprintf (f, "J,%d", tic);
        for (k = 0; k < 16; k++) fprintf (f, ",%d", v[k]);
        for (k = 0; k < 2; k++) {
            fputc (',', f);
            for (i = 0; i < v[15]; i++) fprintf (f, i ? " %d" : "%d", v[16 + 12*k + i]);
        }
        fputc ('\n', f);
    }
    if (gamestate == GS_FINALE)
        fprintf (f, "F,%d,%d,%d,%.8s,%d\n", tic, finalestage, finalecount, finaleflat ? finaleflat : "-",
                 finaletext == e1text ? 1 : finaletext == e2text ? 2 : finaletext == e3text ? 3 : finaletext == e4text ? 4 : 0);
    fflush (f);
}

static void m6a_finish (void)
{
    printf ("m6a: mode %s loads %d wipes %d wi_draws %d fin_draws %d other_draws %d gamestate %d episode %d map %d\n", m6a_mode_name (),
            m6a_loads, m6a_wipes, m6a_wi_draws, m6a_fin_draws, other_draws, (int)gamestate, gameepisode, gamemap);
    if (!m6a_session_out) return;
    fprintf (m6a_session_out, "Z,loads,%d,wipes,%d,snd_draws,%d,st_draws,%d,wipe_draws,%d,wi_draws,%d,fin_draws,%d,other_draws,%d,gamestate,%d,episode,%d,map,%d\n",
             m6a_loads, m6a_wipes, snd_draws, st_draws, wipe_draws, m6a_wi_draws, m6a_fin_draws, other_draws, (int)gamestate, gameepisode, gamemap);
    fclose (m6a_session_out);
}
