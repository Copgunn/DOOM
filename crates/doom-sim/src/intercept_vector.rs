//! Wire 27 (owner's ruling of 2026-11-24): the traces' `P_InterceptVector`
//! (p_maputl.c:231-251) computed with the genuine engine's wrapping `int`
//! arithmetic, where wires 23 to 26 refuse the tic ([`crate::sight::checked_intercept`],
//! decision H3). Sight's `P_InterceptVector2` (p_sight.c:109-128) never wraps:
//! it stays checked under every wire, and under wire 27 alone it is also refused
//! on an `INT_MIN` `FixedDiv` operand ([`sight_intercept`], owner's ruling of
//! 2026-11-24).
use crate::{Fixed, StaticWorld};

/// The five source callers of `P_InterceptVector`, each through
/// `PIT_AddLineIntercepts` (p_maputl.c:589) or `PIT_AddThingIntercepts`
/// (p_maputl.c:663) of one `P_PathTraverse`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InterceptCaller {
    /// The aim trace (`P_AimLineAttack`, `PTR_AimTraverse`): lines and things.
    Aim,
    /// The shoot trace (`P_LineAttack`, `PTR_ShootTraverse`) against a line.
    ShootLine,
    /// The shoot trace against a thing.
    ShootThing,
    /// The slide trace (`P_SlideMove`, `PTR_SlideTraverse`).
    Slide,
    /// The use trace (`P_UseLines`, `PTR_UseTraverse`).
    Use,
}

/// Wire 27's per-caller switch (ruling 6): `true` takes [`wrapping_intercept`],
/// `false` keeps [`crate::sight::checked_intercept`] under wire 27. Each entry
/// is one revertible line.
pub(crate) const fn wire27_wraps(caller: InterceptCaller) -> bool {
    match caller {
        InterceptCaller::Aim => true,
        InterceptCaller::ShootLine => true,
        InterceptCaller::ShootThing => true,
        InterceptCaller::Slide => true,
        InterceptCaller::Use => true,
    }
}

/// Why wire 27 refuses a `FixedDiv(num, den)` (m_fixed.c:57-64) of
/// `P_InterceptVector` (ruling 7: no unevidenced behaviour). Each is the
/// existing `Arithmetic` refusal of the tic; the variants are separate so that
/// one can be lifted alone when it has genuine evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FixedDivRefusal {
    /// Operands that wrapped (a case `checked_intercept` refuses) reach the
    /// saturation guard `(abs(a)>>14) >= abs(b)` (m_fixed.c:62-63). Saturation of
    /// operands that did not wrap is wire 26's value.
    Saturation,
    /// `num` or `den` is `INT_MIN`, whose `abs` (m_fixed.c:62) is undefined in C.
    /// Refused whether or not anything wrapped: the one place where wire 27
    /// differs from wire 26 on a path that did not wrap (wire 26 keeps
    /// `Fixed::fixed_div`'s unsigned-magnitude policy there, Q-p_sight-125).
    AbsMin,
    /// `FixedDiv2`'s `I_Error` range (m_fixed.c:84-85). Unreachable without
    /// `INT_MIN`: when the guard fails, `|a| < |b|·2^14`, so
    /// `|a/b|·65536 < 2^30`; kept as its own variant.
    Div2Range,
}

/// `(num, den)` of `P_InterceptVector` (p_maputl.c:240, 246-248) and of
/// `P_InterceptVector2` (p_sight.c:117, 123-124), the same two expressions, for
/// trace `v2` and line `v1`: every `int` subtraction and addition wraps and every
/// `FixedMul` (m_fixed.c:43-49) narrows to its low 32 bits. `None` when
/// `den == 0`: both functions return 0 there (p_maputl.c:242-243,
/// p_sight.c:119-120) before `num` is computed or `FixedDiv` is reached, so no
/// `num` is refused there. Where nothing wraps (`checked_intercept` is `Some`),
/// these are the exact operands.
fn wrapping_operands(
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Option<(i32, i32)> {
    // FixedMul (m_fixed.c:43-49): 64-bit product, arithmetic shift, low 32 bits.
    let mul = |a: i32, b: i32| ((i64::from(a) * i64::from(b)) >> 16) as i32;
    // den = FixedMul(v1->dy>>8, v2->dx) - FixedMul(v1->dx>>8, v2->dy) (p_maputl.c:240).
    let den = mul(v1_delta[1].bits() >> 8, v2_delta[0].bits())
        .wrapping_sub(mul(v1_delta[0].bits() >> 8, v2_delta[1].bits()));
    if den == 0 {
        return None;
    }
    // num (p_maputl.c:246-248).
    let x = v1_start[0].bits().wrapping_sub(v2_start[0].bits());
    let y = v2_start[1].bits().wrapping_sub(v1_start[1].bits());
    let num = mul(x >> 8, v1_delta[1].bits()).wrapping_add(mul(y >> 8, v1_delta[0].bits()));
    Some((num, den))
}

/// The one definition of [`FixedDivRefusal::AbsMin`]: `FixedDiv(num, den)`
/// evaluates `abs(num)` and `abs(den)` (m_fixed.c:62), undefined for `INT_MIN`.
fn abs_min(num: i32, den: i32) -> Result<(), FixedDivRefusal> {
    if num == i32::MIN || den == i32::MIN {
        return Err(FixedDivRefusal::AbsMin);
    }
    Ok(())
}

/// Wire 27's `P_InterceptVector` of trace `v2` and line `v1`. Every `int`
/// subtraction and addition wraps and every `FixedMul` narrows to its low 32
/// bits, identically in debug and release. Where `checked_intercept` is `Some`
/// the result is that value, except an `INT_MIN` operand
/// ([`FixedDivRefusal::AbsMin`]). Otherwise `den == 0` gives 0 (p_maputl.c:242-243)
/// and the rest is `FixedDiv(num, den)` of the wrapped operands, refused per
/// [`FixedDivRefusal`].
pub(crate) fn wrapping_intercept(
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Result<Fixed, FixedDivRefusal> {
    let Some((num, den)) = wrapping_operands(v2_start, v2_delta, v1_start, v1_delta) else {
        return Ok(Fixed::ZERO);
    };
    abs_min(num, den)?;
    if let Some(exact) = crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta) {
        return Ok(exact);
    }
    if (num.unsigned_abs() >> 14) >= den.unsigned_abs() {
        return Err(FixedDivRefusal::Saturation);
    }
    Fixed::from_bits(num)
        .div2_checked(Fixed::from_bits(den))
        .map_err(|_| FixedDivRefusal::Div2Range)
}

/// `P_InterceptVector` for a caller under the Episode 1 policy's checked
/// arithmetic: [`wrapping_intercept`] under wire 27 for a caller of
/// [`wire27_wraps`], [`crate::sight::checked_intercept`] otherwise (every wire
/// before 27 unchanged). `None` refuses the tic.
pub(crate) fn policy_intercept(
    world: &StaticWorld,
    caller: InterceptCaller,
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Option<Fixed> {
    if world.wrapping_intercept && wire27_wraps(caller) {
        wrapping_intercept(v2_start, v2_delta, v1_start, v1_delta).ok()
    } else {
        crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta)
    }
}

/// Sight's `P_InterceptVector2` (p_sight.c:109-128) under the Episode 1 policy's
/// checked arithmetic. `None` where [`crate::sight::checked_intercept`] is `None`
/// (the source would overflow), under every such wire as before. Under wire 27
/// alone (`wire27`, owner's ruling of 2026-11-24) a `FixedDiv(num, den)`
/// (p_sight.c:125) with `num` or `den` exactly `INT_MIN` is
/// `Some(Err(`[`FixedDivRefusal::AbsMin`]`))`. A `den` of 0 returns 0
/// (p_sight.c:119-120) before `num` is computed or `FixedDiv` is reached, so it
/// is never an `AbsMin` refusal. Sight does not wrap under any wire: otherwise
/// the value is `checked_intercept`'s. Either refusal refuses the tic with the
/// same `Arithmetic` error.
pub(crate) fn sight_intercept(
    wire27: bool,
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Option<Result<Fixed, FixedDivRefusal>> {
    let exact = crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta)?;
    if wire27 {
        // Nothing wrapped (`exact` is Some), so these are the exact operands.
        if let Some((num, den)) = wrapping_operands(v2_start, v2_delta, v1_start, v1_delta) {
            if let Err(refusal) = abs_min(num, den) {
                return Some(Err(refusal));
            }
        }
    }
    Some(Ok(exact))
}
