/*
 * Wire 27 wrap witness, compiled into the `witness` build of tools/reference/wire27_goldens.py only (never into the
 * strict, overflow or address builds, whose dumps are the claim; the witness build's dumps must equal them). Each
 * observer leaves the genuine text unchanged and every wrapper returns the genuine function's result:
 *   - p_maputl.c is compiled with -finstrument-functions: __cyg_profile_func_enter/exit below keep the stack of
 *     p_maputl.c functions, so a FixedMul or FixedDiv call is known to come from P_InterceptVector (p_maputl.c:231-251),
 *     called by PIT_AddLineIntercepts (p_maputl.c:561-609) or PIT_AddThingIntercepts (p_maputl.c:616-674);
 *   - -Wl,--wrap=FixedMul,FixedDiv: the call site, operands and result of the FixedMul calls of p_maputl.c:240 and
 *     246-248 and of the FixedDiv of p_maputl.c:250, all reaching m_fixed.c through these wrappers;
 *   - -Wl,--wrap=P_PathTraverse: the traverser of the current trace (PTR_AimTraverse, PTR_ShootTraverse,
 *     PTR_SlideTraverse or PTR_UseTraverse, p_map.c).
 * One row per P_InterceptVector call, at its exit, to the file named by WIRE27_WITNESS:
 *   V tic traverser caller trace.x trace.y trace.dx trace.dy n {site a b result}*n [D a b result]
 * tic is gametic during the call (the 0-based G_Ticker of the run); site is the FixedMul return address minus
 * P_InterceptVector's address. The rows are read by wire27_goldens.py, which classifies them.
 */
#include <stdio.h>
#include <stdlib.h>
#include "doomdef.h"
#include "doomstat.h"
#include "m_fixed.h"
#include "p_local.h"

#define NO_HOOK __attribute__ ((no_instrument_function))
fixed_t __real_FixedMul (fixed_t a, fixed_t b);
fixed_t __real_FixedDiv (fixed_t a, fixed_t b);
boolean __real_P_PathTraverse (fixed_t x1, fixed_t y1, fixed_t x2, fixed_t y2, int flags, boolean (*trav) (intercept_t*));
boolean PIT_AddLineIntercepts (line_t* ld);
boolean PIT_AddThingIntercepts (mobj_t* thing);
boolean PTR_AimTraverse (intercept_t* in);
boolean PTR_ShootTraverse (intercept_t* in);
boolean PTR_SlideTraverse (intercept_t* in);
boolean PTR_UseTraverse (intercept_t* in);

#define DEPTH 256
static void* stack[DEPTH];
static int depth, lost;
static boolean (*traverser) (intercept_t*);
static long site[4];
static fixed_t mul[4][3], quo[3];
static int nmul, ndiv;
static FILE* out;

static int NO_HOOK in_intercept (void) { return depth > 0 && depth <= DEPTH && stack[depth - 1] == (void*)P_InterceptVector; }

static void NO_HOOK emit (void)
{
    int i;
    void* caller = depth > 0 && depth <= DEPTH ? stack[depth - 1] : NULL;
    if (!out) {
        const char* path = getenv ("WIRE27_WITNESS");
        if (!path || !(out = fopen (path, "w"))) { fprintf (stderr, "wire27 witness: no WIRE27_WITNESS file\n"); exit (3); }
    }
    fprintf (out, "V %d %s %s %d %d %d %d %d", gametic,
             traverser == PTR_AimTraverse ? "aim" : traverser == PTR_ShootTraverse ? "shoot"
             : traverser == PTR_SlideTraverse ? "slide" : traverser == PTR_UseTraverse ? "use" : "other",
             caller == (void*)PIT_AddLineIntercepts ? "line" : caller == (void*)PIT_AddThingIntercepts ? "thing" : "other",
             trace.x, trace.y, trace.dx, trace.dy, lost ? -1 : nmul);
    for (i = 0; i < nmul && i < 4; i++) fprintf (out, " %ld %d %d %d", site[i], mul[i][0], mul[i][1], mul[i][2]);
    if (ndiv) fprintf (out, " D %d %d %d", quo[0], quo[1], quo[2]);
    fputc ('\n', out);
    fflush (out);
    nmul = ndiv = 0;
}

void NO_HOOK __cyg_profile_func_enter (void* fn, void* call_site)
{
    (void)call_site;
    if (depth < DEPTH) stack[depth] = fn; else lost = 1;
    depth++;
}

void NO_HOOK __cyg_profile_func_exit (void* fn, void* call_site)
{
    (void)call_site;
    depth--;
    if (fn == (void*)P_InterceptVector) emit ();
}

fixed_t NO_HOOK __wrap_FixedMul (fixed_t a, fixed_t b)
{
    fixed_t r = __real_FixedMul (a, b);
    if (in_intercept ()) {
        if (nmul < 4) {
            site[nmul] = (long)((char*)__builtin_return_address (0) - (char*)P_InterceptVector);
            mul[nmul][0] = a; mul[nmul][1] = b; mul[nmul][2] = r;
        } else lost = 1;
        nmul++;
    }
    return r;
}

fixed_t NO_HOOK __wrap_FixedDiv (fixed_t a, fixed_t b)
{
    fixed_t r = __real_FixedDiv (a, b);
    if (in_intercept ()) { quo[0] = a; quo[1] = b; quo[2] = r; ndiv++; }
    return r;
}

boolean NO_HOOK __wrap_P_PathTraverse (fixed_t x1, fixed_t y1, fixed_t x2, fixed_t y2, int flags,
                                       boolean (*trav) (intercept_t*))
{
    boolean r;
    traverser = trav;
    r = __real_P_PathTraverse (x1, y1, x2, y2, flags, trav);
    traverser = NULL;
    return r;
}
