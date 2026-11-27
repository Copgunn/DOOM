/* Driver of tools/reference/wire27_read.py (the independent reader's harness of 2026-11-24, committed 2026-11-26).
 * fixed_body.inc and iv_body.inc are the genuine text of m_fixed.c:43-87 and p_maputl.c:231-284, extracted by the tool
 * into its work directory and never edited; build with -fwrapv. One row of eight ints per line on stdin (v2 then v1,
 * each x y dx dy); one line out: "Z frac" (den == 0, no division), "D num den frac sawmin" or "I num den" (I_Error).
 * PLANT (0 unless the self-test sets it) is added to every quotient: the planted fault of --selftest. */
#include <stdio.h>
#include <setjmp.h>
typedef int fixed_t;
#define FRACBITS 16
#define FRACUNIT (1 << FRACBITS)
#define MAXINT ((int)0x7fffffff)
#define MININT ((int)0x80000000)
#ifndef PLANT
#define PLANT 0
#endif
typedef struct { fixed_t x, y, dx, dy; } divline_t;
static jmp_buf jb;
static int saw_min, called, rec_num, rec_den;
static int my_abs(int x)
{
    if (x == MININT) saw_min = 1;
    return (int)(x < 0 ? 0u - (unsigned)x : (unsigned)x);
}
#define abs(x) my_abs(x)
static void I_Error(char *s, ...) { (void)s; longjmp(jb, 1); }
fixed_t FixedDiv2(fixed_t a, fixed_t b);
#include "fixed_body.inc"
static fixed_t FixedDivRec(fixed_t a, fixed_t b)
{
    called = 1; rec_num = a; rec_den = b;
    return FixedDiv(a, b) + PLANT;
}
#define FixedDiv FixedDivRec
fixed_t
#include "iv_body.inc"
}
int main(void)
{
    divline_t v2, v1;
    static char out[1 << 16], in[1 << 16];
    setvbuf(stdout, out, _IOFBF, sizeof out);
    setvbuf(stdin, in, _IOFBF, sizeof in);
    while (scanf("%d %d %d %d %d %d %d %d", &v2.x, &v2.y, &v2.dx, &v2.dy, &v1.x, &v1.y, &v1.dx, &v1.dy) == 8) {
        volatile int frac = 0;
        saw_min = called = 0;
        if (setjmp(jb)) { printf("I %d %d\n", rec_num, rec_den); continue; }
        frac = P_InterceptVector(&v2, &v1);
        if (!called) printf("Z %d\n", frac);
        else printf("D %d %d %d %d\n", rec_num, rec_den, frac, saw_min);
    }
    return 0;
}
