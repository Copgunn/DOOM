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

/// Wire 27's per-caller switch (ruling 6): `false` keeps
/// [`crate::sight::checked_intercept`] under wire 27, with its
/// [`FixedDivRefusal::AbsMin`] refusal; `true` lets [`wire27_admits`] admit the
/// caller's witnessed wrap classes. A caller wraps only when a genuine-compared
/// item takes its wrapping path (tests/reference/wire27_results.json).
pub(crate) const fn wire27_wraps(caller: InterceptCaller) -> bool {
    match caller {
        InterceptCaller::Aim => true,
        InterceptCaller::ShootLine => true,
        // No item can wrap here, so none is genuine-compared: the divline of
        // PIT_AddThingIntercepts (p_maputl.c:616-674) is a thing's diagonal,
        // |dx|, |dy| = 2·radius, and a thing in a cell of a 2,048-unit trace is
        // within 2,048 + 128·√2 + radius units, so every product of p_maputl.c:240
        // and 246-248 and both sums stay under 2^29 for any mobjinfo radius (at
        // most 128 units, info.c).
        InterceptCaller::ShootThing => false,
        InterceptCaller::Slide => true,
        InterceptCaller::Use => true,
    }
}

/// Whether the `int` value of `den` or `num` (p_maputl.c:240, 246-248) equals
/// the exact value of the same expression (owner's correction of 2026-11-24).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operand {
    /// The wraps cancelled: the `int` value is the exact one.
    Exact,
    /// The `int` value differs from the exact one.
    Wrong,
}

/// The class of a `P_InterceptVector` whose checked computation overflows (the
/// `wrap.` classes of tools/reference/wire27_goldens.py, which names them).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WrapClass {
    /// The `int` den is 0, so the function returns 0 (p_maputl.c:242-243);
    /// `exact` when the exact den is 0 too.
    DenZero { exact: bool },
    /// `FixedDiv2` (m_fixed.c:69-87) of the `int` operands.
    Div2 { den: Operand, num: Operand },
    /// `FixedDiv`'s saturation (m_fixed.c:62-63) of the `int` operands:
    /// `MAXINT` when `max` (`(num ^ den) >= 0`), `MININT` otherwise.
    Saturation {
        den: Operand,
        num: Operand,
        max: bool,
    },
}

/// Wire 27's admission of a wrap class for a caller (owner's corrections of
/// 2026-11-24): only a class that a genuine-compared item takes for that caller
/// (the `T` rows of tests/reference/wire27_state.txt, which
/// `wire27_tests::wire27_admits_exactly_the_witnessed_classes` checks this
/// against). Every other wrap class refuses the tic.
pub(crate) const fn wire27_admits(caller: InterceptCaller, class: WrapClass) -> bool {
    use InterceptCaller::{Aim, ShootLine, Slide, Use};
    use Operand::{Exact, Wrong};
    match class {
        // The wraps cancel: USE, SLIDE, AIMSHOT (aim) and SHOOTC.
        WrapClass::Div2 {
            den: Exact,
            num: Exact,
        } => matches!(caller, Aim | ShootLine | Slide | Use),
        // A wrong den, FixedDiv2: AIMSHOT.
        WrapClass::Div2 {
            den: Wrong,
            num: Exact,
        } => matches!(caller, Aim | ShootLine),
        // A wrong num and den: FixedDiv2 in NUMWRAP, MAXINT in SATUR, MININT in
        // SATNEG.
        WrapClass::Div2 {
            den: Wrong,
            num: Wrong,
        }
        | WrapClass::Saturation {
            den: Wrong,
            num: Wrong,
            ..
        } => matches!(caller, ShootLine),
        // The den wraps to 0: SHOOTZ.
        WrapClass::DenZero { exact: false } => matches!(caller, ShootLine),
        _ => false,
    }
}

/// Why wire 27 refuses a `FixedDiv(num, den)` (m_fixed.c:57-64) of
/// `P_InterceptVector` (ruling 7: no unevidenced behaviour). Each is the
/// existing `Arithmetic` refusal of the tic; the variants are separate so that
/// one can be lifted alone when it has genuine evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FixedDivRefusal {
    /// Operands that wrapped (a case `checked_intercept` refuses) reach the
    /// saturation guard `(abs(a)>>14) >= abs(b)` (m_fixed.c:62-63) in a class
    /// that [`wire27_admits`] refuses for the caller; where it admits the class,
    /// wire 27 reproduces the saturation. Saturation of operands that did not
    /// wrap is wire 26's value, under wire 27 too.
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

/// Wire 27's `P_InterceptVector` of trace `v2` and line `v1` regardless of
/// caller (the unit tests' transcription check). Every `int`
/// subtraction and addition wraps and every `FixedMul` narrows to its low 32
/// bits, identically in debug and release. Where `checked_intercept` is `Some`
/// the result is that value, except an `INT_MIN` operand
/// ([`FixedDivRefusal::AbsMin`]). Otherwise `den == 0` gives 0 (p_maputl.c:242-243)
/// and the rest is `FixedDiv(num, den)` of the wrapped operands, refused per
/// [`FixedDivRefusal`].
#[cfg(test)]
pub(crate) fn wrapping_intercept(
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Result<Fixed, FixedDivRefusal> {
    match classified(v2_start, v2_delta, v1_start, v1_delta)? {
        (_, Some(WrapClass::Saturation { .. })) => Err(FixedDivRefusal::Saturation),
        (value, _) => Ok(value),
    }
}

/// The source value of `P_InterceptVector` (p_maputl.c:240-250, `FixedDiv` of
/// m_fixed.c:57-87) and, where `checked_intercept` is `None`, its
/// [`WrapClass`]; `Err` for an `INT_MIN` operand or `FixedDiv2`'s range. The
/// exact `den` and `num` are the same expressions in 64-bit integers (each
/// `FixedMul` product, its shift and the sums fit), without any narrowing.
fn classified(
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Result<(Fixed, Option<WrapClass>), FixedDivRefusal> {
    let checked = crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta);
    let wide = |a: i64, b: i32| (a * i64::from(b)) >> 16;
    let shifted = |v: Fixed| i64::from(v.bits() >> 8);
    let den_exact = wide(shifted(v1_delta[1]), v2_delta[0].bits())
        - wide(shifted(v1_delta[0]), v2_delta[1].bits());
    let Some((num, den)) = wrapping_operands(v2_start, v2_delta, v1_start, v1_delta) else {
        let class = WrapClass::DenZero {
            exact: den_exact == 0,
        };
        return Ok((Fixed::ZERO, checked.is_none().then_some(class)));
    };
    abs_min(num, den)?;
    if let Some(exact) = checked {
        return Ok((exact, None));
    }
    let x = i64::from(v1_start[0].bits()) - i64::from(v2_start[0].bits());
    let y = i64::from(v2_start[1].bits()) - i64::from(v1_start[1].bits());
    let num_exact = wide(x >> 8, v1_delta[1].bits()) + wide(y >> 8, v1_delta[0].bits());
    let operand = |int: i32, exact: i64| {
        if i64::from(int) == exact {
            Operand::Exact
        } else {
            Operand::Wrong
        }
    };
    let (den_class, num_class) = (operand(den, den_exact), operand(num, num_exact));
    if (num.unsigned_abs() >> 14) >= den.unsigned_abs() {
        let max = (num ^ den) >= 0;
        let class = WrapClass::Saturation {
            den: den_class,
            num: num_class,
            max,
        };
        return Ok((if max { Fixed::MAX } else { Fixed::MIN }, Some(class)));
    }
    let value = Fixed::from_bits(num)
        .div2_checked(Fixed::from_bits(den))
        .map_err(|_| FixedDivRefusal::Div2Range)?;
    let class = WrapClass::Div2 {
        den: den_class,
        num: num_class,
    };
    Ok((value, Some(class)))
}

/// `P_InterceptVector` for a caller under the Episode 1 policy's checked
/// arithmetic: [`wire27_intercept`] under wire 27,
/// [`crate::sight::checked_intercept`] under every wire before 27 (unchanged).
/// `None` refuses the tic.
pub(crate) fn policy_intercept(
    world: &StaticWorld,
    caller: InterceptCaller,
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Option<Fixed> {
    if world.wrapping_intercept {
        wire27_intercept(caller, v2_start, v2_delta, v1_start, v1_delta).ok()
    } else {
        crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta)
    }
}

/// Why wire 27 refuses a `P_InterceptVector` for a caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Wire27Refusal {
    /// A refused `FixedDiv` ([`FixedDivRefusal`]).
    FixedDiv(FixedDivRefusal),
    /// A wrap class that [`wire27_admits`] refuses for the caller (no
    /// genuine-compared item takes it), or any wrap class of a caller that
    /// [`wire27_wraps`] keeps checked (`refuse:Checked` in the goldens' table),
    /// saturation aside.
    Unevidenced(WrapClass),
}

/// Wire 27's `P_InterceptVector` for `caller`: [`classified`]'s value where
/// nothing wraps or where [`wire27_admits`] its wrap class for a caller of
/// [`wire27_wraps`]; otherwise the refusal.
pub(crate) fn wire27_intercept(
    caller: InterceptCaller,
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Result<Fixed, Wire27Refusal> {
    match classified(v2_start, v2_delta, v1_start, v1_delta).map_err(Wire27Refusal::FixedDiv)? {
        (value, None) => Ok(value),
        (value, Some(class)) if wire27_wraps(caller) && wire27_admits(caller, class) => Ok(value),
        (_, Some(WrapClass::Saturation { .. })) => {
            Err(Wire27Refusal::FixedDiv(FixedDivRefusal::Saturation))
        }
        (_, Some(class)) => Err(Wire27Refusal::Unevidenced(class)),
    }
}

/// [`crate::sight::checked_intercept`] with wire 27's
/// [`FixedDivRefusal::AbsMin`]: `None` where the source would overflow, and
/// `Some(Err(AbsMin))` where `FixedDiv(num, den)` (p_maputl.c:250, p_sight.c:125)
/// has `num` or `den` exactly `INT_MIN` (m_fixed.c:62). A `den` of 0 returns 0
/// before `num` is computed or `FixedDiv` is reached (p_maputl.c:242-243,
/// p_sight.c:119-120), so it is never refused here.
fn checked_abs_min(
    v2_start: [Fixed; 2],
    v2_delta: [Fixed; 2],
    v1_start: [Fixed; 2],
    v1_delta: [Fixed; 2],
) -> Option<Result<Fixed, FixedDivRefusal>> {
    let exact = crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta)?;
    // Nothing wrapped (`exact` is Some), so these are the exact operands.
    if let Some((num, den)) = wrapping_operands(v2_start, v2_delta, v1_start, v1_delta) {
        if let Err(refusal) = abs_min(num, den) {
            return Some(Err(refusal));
        }
    }
    Some(Ok(exact))
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
    if wire27 {
        checked_abs_min(v2_start, v2_delta, v1_start, v1_delta)
    } else {
        crate::sight::checked_intercept(v2_start, v2_delta, v1_start, v1_delta).map(Ok)
    }
}
