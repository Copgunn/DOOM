/* ---------------- M6b: the pause, a prior game in the same process, and their observers ----------------
 * tools/reference/m6b_build.py includes this file into the M6b driver, which is the generated M6a driver
 * text (m6a_build.driver_text, itself the frozen m5c_driver.c with the M6a patches) with the exact-once
 * patches listed in m6b_build.PATCHES (docs/M6B_REFERENCE_INVESTIGATION.md section 6.1). The genuine
 * files are compiled unchanged: the pause itself (g_game.c:698-712, p_tick.c:135-136, p_user.c:280-281,
 * s_sound.c:497-513) and every Nightmare rule are the original text. New here:
 *   - the pause picture of D_Display (d_main.c:303-311) in the D_Display mirror, where the M5b and M6a
 *     drivers aborted ("never in a demo without pause"): m6b_draw_pause, the five source lines verbatim.
 *     With --render none no level frame is drawn, so no picture either; the intermission and finale
 *     display runs with --render none as well and draws it;
 *   - observers of I_PauseSong and I_ResumeSong (-Wl,--wrap; i_sound.c:848-858, both empty in the
 *     original): what S_PauseSound and S_ResumeSound (s_sound.c:497-513) do is visible only there;
 *   - --prior-game SKILL,RESPAWN,FAST (repeatable, at most 4; evidence only): before the demo starts, the
 *     runner sets respawnparm and fastparm and calls the genuine G_InitNew (SKILL, 1, 1), as an earlier
 *     game of the same process would have (g_game.c:1364-1476: the table edit of 1421-1436 is keyed on
 *     the gameskill the earlier game left). G_DoPlayDemo then runs as always;
 *   - the M6b stream (--m6b-out FILE, DOOM-M6B-RUNNER-v1): a T row with the thirteen state durations
 *     S_SARG_RUN1..S_SARG_PAIN2 and the three missile speeds at the start, after every prior game and
 *     after the demo's G_InitNew; an MP row per I_PauseSong or I_ResumeSong call; a B row per tic that ends
 *     paused (the game state, both clocks, both random indices, whether the picture was drawn in the tic's
 *     display and the digest of the whole frame when it was); a Z row of totals;
 *   - --selftest-pause-y (test only): the picture one row lower, so a frame golden can be shown to fail. */
#define M6B_MAXPRIOR 4
static FILE* m6b_out;
static int m6b_prior[M6B_MAXPRIOR][3], m6b_nprior, m6b_selftest_y;
static int m6b_drawn, m6b_pause_frames, m6b_paused_tics, m6b_song_calls;

static const char* M6B_COLUMNS =
 "#schema,DOOM-M6B-RUNNER-v1\n"
 "#columns,T,when,skill,respawnparm,fastparm,respawnmonsters,sarg_tics,bruisershot_speed,headshot_speed,troopshot_speed\n"
 "#columns,MP,tic,call,handle\n"
 "#columns,B,tic,gamestate,leveltime,gametic,prndindex,rndindex,drawn,frame_sha256\n"
 "#columns,Z,totals\n";

static int m6b_option (int argc, char** argv, int* i)
{
    const char* a = argv[*i];
    if (!strcmp (a, "--selftest-pause-y")) { m6b_selftest_y = 1; return 1; }
    if (*i + 1 >= argc) return 0;
    if (!strcmp (a, "--m6b-out")) {
        if (!(m6b_out = fopen (argv[++*i], "w"))) { perror (argv[*i]); exit (4); }
        fputs (M6B_COLUMNS, m6b_out);
        return 1;
    }
    if (!strcmp (a, "--prior-game")) {
        int* p = m6b_prior[m6b_nprior];
        if (m6b_nprior == M6B_MAXPRIOR || sscanf (argv[++*i], "%d,%d,%d", p, p + 1, p + 2) != 3
            || p[0] < 0 || p[0] > 4 || (p[1] | p[2]) & ~1) { fprintf (stderr, "--prior-game SKILL,RESPAWN,FAST (0-4, 0-1, 0-1; at most %d)\n", M6B_MAXPRIOR); exit (2); }
        m6b_nprior++;
        return 1;
    }
    return 0;
}

/* The process-global tables G_InitNew edits (g_game.c:1423-1435). */
static void m6b_table_row (const char* when)
{
    int i;
    if (!m6b_out) return;
    fprintf (m6b_out, "T,%s,%d,%d,%d,%d,", when, (int)gameskill, (int)respawnparm, (int)fastparm, (int)respawnmonsters);
    for (i = S_SARG_RUN1; i <= S_SARG_PAIN2; i++) fprintf (m6b_out, i == S_SARG_RUN1 ? "%ld" : " %ld", states[i].tics);
    fprintf (m6b_out, ",%d,%d,%d\n", mobjinfo[MT_BRUISERSHOT].speed, mobjinfo[MT_HEADSHOT].speed, mobjinfo[MT_TROOPSHOT].speed);
    fflush (m6b_out);
}

/* Earlier games of this process, then the table the demo is about to meet. */
static void m6b_prior_games (void)
{
    int k; char when[16];
    m6b_table_row ("start");
    for (k = 0; k < m6b_nprior; k++) {
        respawnparm = m6b_prior[k][1];            /* d_main.c:809-810 */
        fastparm = m6b_prior[k][2];
        G_InitNew ((skill_t)m6b_prior[k][0], 1, 1);
        snprintf (when, sizeof when, "prior%d", k + 1);
        m6b_table_row (when);
    }
}

/* d_main.c:303-311, in the D_Display mirror after oldgamestate = wipegamestate = gamestate (d_main.c:300). */
static void m6b_draw_pause (void)
{
    int y;
    if (automapactive)                            /* d_main.c:305-308 */
        y = 4;
    else
        y = viewwindowy+4;
    if (m6b_selftest_y) y++;                      /* --selftest-pause-y: test only */
    V_DrawPatchDirect(viewwindowx+(scaledviewwidth-68)/2,
                      y,0,W_CacheLumpName ("M_PAUSE", PU_CACHE));   /* d_main.c:309-310 */
    m6b_drawn = 1; m6b_pause_frames++;
}

void __real_I_PauseSong (int handle);
void __wrap_I_PauseSong (int handle)
{
    m6b_song_calls++;
    if (m6b_out) fprintf (m6b_out, "MP,%d,pause,%d\n", snd_tic, handle);
    __real_I_PauseSong (handle);
}
void __real_I_ResumeSong (int handle);
void __wrap_I_ResumeSong (int handle)
{
    m6b_song_calls++;
    if (m6b_out) fprintf (m6b_out, "MP,%d,resume,%d\n", snd_tic, handle);
    __real_I_ResumeSong (handle);
}

/* The B row of a tic that ends paused, after its display. */
static void m6b_tic_end (int tic)
{
    char sha[65];
    if (paused) {
        m6b_paused_tics++;
        if (m6b_out) {
            if (m6b_drawn) sha256_hex (screens[0], SCREENWIDTH*SCREENHEIGHT, sha); else strcpy (sha, "-");
            fprintf (m6b_out, "B,%d,%d,%d,%d,%d,%d,%d,%s\n", tic, (int)gamestate, leveltime, gametic, prndindex, rndindex, m6b_drawn, sha);
            fflush (m6b_out);
        }
    }
    m6b_drawn = 0;
}

static void m6b_finish (void)
{
    printf ("m6b: prior_games %d paused_tics %d pause_frames %d song_calls %d paused_at_end %d\n", m6b_nprior, m6b_paused_tics,
            m6b_pause_frames, m6b_song_calls, (int)paused);
    if (!m6b_out) return;
    fprintf (m6b_out, "Z,prior_games,%d,paused_tics,%d,pause_frames,%d,song_calls,%d,paused_at_end,%d\n", m6b_nprior, m6b_paused_tics,
             m6b_pause_frames, m6b_song_calls, (int)paused);
    fclose (m6b_out);
}
