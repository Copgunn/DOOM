/* Evidence observer for tools/media/m6a2_readme_visual.py: linked after the genuine M6a runner's own objects
 * (tools/reference/m6a_build.py, built and untouched), it adds three --wrap hooks and changes nothing the runner
 * computes. After each genuine F_Drawer, WI_Drawer and wipe_ScreenWipe call it writes screens[0] (64,000 palette
 * indices) to $M6A_TAP/{d_TIC,m_TIC_STEP}.raw, and at a melt's first step the start and end screens
 * (screens[2], screens[3]) as ms_TIC.raw and me_TIC.raw. TIC is gametic - 1: the display follows G_Ticker's
 * increment. The script checks every file against the committed genuine digests (tests/reference/m6a_frames,
 * m6a2_frames, m6a2_melt); a wrong name or content fails there. */
#include <stdio.h>
#include <stdlib.h>
#include "doomtype.h"
extern byte* screens[5];
extern int gametic;
void __real_F_Drawer (void);
void __real_WI_Drawer (void);
int __real_wipe_ScreenWipe (int wipeno, int x, int y, int width, int height, int ticks);
static int fresh = 1, step;
static void dump (const char* fmt, int a, int b, int s)
{
    char path[1024]; FILE* f;
    snprintf (path, sizeof path, fmt, getenv ("M6A_TAP"), a, b);
    if (!(f = fopen (path, "wb")) || fwrite (screens[s], 1, 64000, f) != 64000) { perror (path); exit (4); }
    fclose (f);
}
void __wrap_F_Drawer (void) { __real_F_Drawer (); dump ("%s/d_%d.raw", gametic - 1, 0, 0); }
void __wrap_WI_Drawer (void) { __real_WI_Drawer (); dump ("%s/d_%d.raw", gametic - 1, 0, 0); }
int __wrap_wipe_ScreenWipe (int w, int x, int y, int wd, int h, int ticks)
{
    int done;
    if (fresh) { dump ("%s/ms_%d.raw", gametic - 1, 0, 2); dump ("%s/me_%d.raw", gametic - 1, 0, 3); step = 0; }
    done = __real_wipe_ScreenWipe (w, x, y, wd, h, ticks);
    dump ("%s/m_%d_%d.raw", gametic - 1, ++step, 0);
    fresh = done;
    return done;
}
