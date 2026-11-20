//! Saturating arithmetic for game quantities.
//!
//! Magic quantities (life, damage, counters, power/toughness, mana amounts,
//! X and every derived count) saturate at the bounds of their storage type.
//! They never wrap and never fail: a value past the bound is treated as an
//! effectively infinite amount and clamps to the bound. Wide (`i64`)
//! intermediates are used for evaluation and narrowed with the helpers below.
//!
//! Object creation is the one place saturation does not help, because every
//! object costs memory. Counts that become objects (tokens, copies, ability
//! occurrences) are clamped separately to the engine's object budget.

/// Narrow a wide quantity to `i32`, clamping at the representable bounds.
#[inline]
pub const fn clamp_i32(value: i64) -> i32 {
    if value > i32::MAX as i64 {
        i32::MAX
    } else if value < i32::MIN as i64 {
        i32::MIN
    } else {
        value as i32
    }
}

/// Narrow a wide quantity to `u32`, clamping negatives to zero.
#[inline]
pub const fn clamp_u32(value: i64) -> u32 {
    if value > u32::MAX as i64 {
        u32::MAX
    } else if value < 0 {
        0
    } else {
        value as u32
    }
}

/// Narrow an unsigned wide quantity to `u32`.
#[inline]
pub const fn clamp_u32_from_u64(value: u64) -> u32 {
    if value > u32::MAX as u64 { u32::MAX } else { value as u32 }
}

/// Narrow an unsigned count to `i32`.
#[inline]
pub const fn clamp_i32_from_u32(value: u32) -> i32 {
    if value > i32::MAX as u32 { i32::MAX } else { value as i32 }
}

/// Narrow an `i128` intermediate to `i64`.
#[inline]
pub const fn clamp_i64_from_i128(value: i128) -> i64 {
    if value > i64::MAX as i128 {
        i64::MAX
    } else if value < i64::MIN as i128 {
        i64::MIN
    } else {
        value as i64
    }
}

/// Narrow a signed quantity to a `usize` count (negative counts are zero).
#[inline]
pub fn clamp_usize(value: i64) -> usize {
    usize::try_from(value.max(0)).unwrap_or(usize::MAX)
}

/// `value / divisor` rounded toward negative infinity; zero divisor yields zero.
#[inline]
pub const fn div_floor(value: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        return 0;
    }
    if value == i64::MIN && divisor == -1 {
        return i64::MAX;
    }
    let quotient = value / divisor;
    if (value % divisor != 0) && ((value < 0) != (divisor < 0)) {
        quotient - 1
    } else {
        quotient
    }
}

/// `value / divisor` rounded up for nonnegative operands; zero divisor yields zero.
#[inline]
pub const fn div_ceil(value: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        return 0;
    }
    -div_floor(value.saturating_neg(), divisor)
}

/// Saturating integer power with a nonnegative exponent.
#[inline]
pub const fn pow(base: i64, exponent: u32) -> i64 {
    base.saturating_pow(exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrowing_clamps_at_both_bounds() {
        assert_eq!(clamp_i32(i64::MAX), i32::MAX);
        assert_eq!(clamp_i32(i64::MIN), i32::MIN);
        assert_eq!(clamp_i32(-7), -7);
        assert_eq!(clamp_u32(-1), 0);
        assert_eq!(clamp_u32(i64::MAX), u32::MAX);
        assert_eq!(clamp_i32_from_u32(u32::MAX), i32::MAX);
        assert_eq!(clamp_usize(-3), 0);
    }

    #[test]
    fn division_rounds_and_never_panics() {
        assert_eq!(div_floor(7, 2), 3);
        assert_eq!(div_floor(-7, 2), -4);
        assert_eq!(div_ceil(7, 2), 4);
        assert_eq!(div_floor(5, 0), 0);
        assert_eq!(div_floor(i64::MIN, -1), i64::MAX);
    }

    #[test]
    fn power_saturates() {
        assert_eq!(pow(2, 30), 1 << 30);
        assert_eq!(pow(2, 200), i64::MAX);
        assert_eq!(pow(-2, 201), i64::MIN);
    }
}
