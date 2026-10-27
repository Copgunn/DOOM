use std::{
    error::Error,
    fmt,
    ops::{Add, Div, Mul, Neg, Sub},
};

/// Signed 32-bit 16.16 value with explicitly wrapping add/sub/negation/narrowing.
/// Multiplication uses an arithmetic shift (floor); division truncates toward zero.
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Fixed(i32);

/// Failure of the direct Linux floating-point division helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedDivError {
    /// Zero divisor; includes the C-undefined NaN-to-integer 0/0 case.
    DivisionByZero,
    /// Scaled quotient lies outside signed 32-bit range.
    Overflow,
}
impl fmt::Display for FixedDivError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fixed division: {self:?}")
    }
}
impl Error for FixedDivError {}

impl Fixed {
    /// Number of fractional bits.
    pub const FRACTION_BITS: u32 = 16;
    /// Zero.
    pub const ZERO: Self = Self(0);
    /// One map unit.
    pub const ONE: Self = Self(65536);
    /// Minimum representable raw value.
    pub const MIN: Self = Self(i32::MIN);
    /// Maximum representable raw value.
    pub const MAX: Self = Self(i32::MAX);
    /// Constructs from an exact raw bit pattern.
    pub const fn from_bits(bits: i32) -> Self {
        Self(bits)
    }
    /// Returns the signed raw representation, independent of host word size.
    pub const fn bits(self) -> i32 {
        self.0
    }
    /// Exact conversion of a classic signed map coordinate.
    pub const fn from_map_units(units: i16) -> Self {
        Self((units as i32).wrapping_shl(16))
    }
    /// Widened signed multiplication, arithmetic shift, then low-32-bit narrowing.
    pub const fn fixed_mul(self, rhs: Self) -> Self {
        Self(((self.0 as i64 * rhs.0 as i64) >> 16) as i32)
    }
    /// Linux `FixedDiv` guard and active double helper. Zero divides saturate by
    /// operand sign, including 0/0 -> MAX. Unsigned magnitudes define INT_MIN.
    pub fn fixed_div(self, rhs: Self) -> Self {
        if (self.0.unsigned_abs() >> 14) >= rhs.0.unsigned_abs() {
            return if (self.0 ^ rhs.0) < 0 {
                Self::MIN
            } else {
                Self::MAX
            };
        }
        // The guard bounds the finite quotient, including our INT_MIN policy.
        match self.div2_checked(rhs) {
            Ok(value) => value,
            Err(_) => {
                if (self.0 ^ rhs.0) < 0 {
                    Self::MIN
                } else {
                    Self::MAX
                }
            }
        }
    }
    /// Direct `FixedDiv2` using IEEE binary64 division then multiplication by
    /// 65536. Errors replace the reference fatal path and undefined zero/NaN cast.
    pub fn div2_checked(self, rhs: Self) -> Result<Self, FixedDivError> {
        if rhs.0 == 0 {
            return Err(FixedDivError::DivisionByZero);
        }
        let scaled = (f64::from(self.0) / f64::from(rhs.0)) * 65536.0;
        if !(-2147483648.0..2147483648.0).contains(&scaled) {
            return Err(FixedDivError::Overflow);
        }
        Ok(Self(scaled as i32))
    }
}
impl Add for Fixed {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }
}
impl Sub for Fixed {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.wrapping_sub(rhs.0))
    }
}
impl Neg for Fixed {
    type Output = Self;
    fn neg(self) -> Self {
        Self(self.0.wrapping_neg())
    }
}
impl Mul for Fixed {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.fixed_mul(rhs)
    }
}
impl Div for Fixed {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.fixed_div(rhs)
    }
}
