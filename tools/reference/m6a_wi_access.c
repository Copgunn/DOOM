/*
 * M6a runner: the wi_stuff.c accessor unit (declared adapter, docs/M6A_REFERENCE_INVESTIGATION.md
 * section 3.1 item 5).
 *
 * wi_stuff.c keeps the intermission's state in file-static variables (wi_stuff.c:304-331, 749,
 * 1316 and the animation tables 226-275), which no other unit can name. This unit #includes the
 * genuine wi_stuff.c, unchanged and whole (tools/reference/m6a_build.py copies the genuine text
 * into the build directory and records its SHA-256 in the contract), and adds one function that
 * only reads those statics. It is compiled in place of wi_stuff.c; nothing else is added, and no
 * genuine statement is changed or reordered.
 */
#include "wi_stuff.c"

/* The intermission statics, in the order of M6A_WI_COLUMNS of tools/reference/m6a_session.c.
 * v must hold M6A_WI_NV ints. Reads only. */
#define M6A_WI_NV 40
int m6a_wi_statics (int* v)
{
    int i, n = 0, k;
    v[n++] = (int)state;                       /* wi_stuff.c:310 */
    v[n++] = acceleratestage;                  /* 304 */
    v[n++] = me;                               /* 307 */
    v[n++] = cnt;                              /* 318 */
    v[n++] = bcnt;                             /* 321 */
    v[n++] = firstrefresh;                     /* 324 */
    v[n++] = sp_state;                         /* 1316 */
    v[n++] = cnt_kills[0];                     /* 326-331, the console player's counters */
    v[n++] = cnt_items[0];
    v[n++] = cnt_secret[0];
    v[n++] = cnt_time;
    v[n++] = cnt_par;
    v[n++] = cnt_pause;
    v[n++] = (int)snl_pointeron;               /* 749 */
    v[n++] = wbs == &wminfo;                   /* 313: WI_Start (&wminfo), g_game.c:1140 */
    k = (wbs && wbs->epsd >= 0 && wbs->epsd < 3) ? NUMANIMS[wbs->epsd] : 0;   /* 263-275 */
    if (k > 12) k = 12;
    v[n++] = k;
    for (i = 0; i < 12; i++) v[n++] = i < k ? anims[wbs->epsd][i].ctr : -1;       /* anim_t.ctr */
    for (i = 0; i < 12; i++) v[n++] = i < k ? anims[wbs->epsd][i].nexttic : -1;   /* anim_t.nexttic */
    return n;
}
