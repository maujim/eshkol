use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    numerator: i64,
    denominator: i64,
}

impl Rational {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    pub fn new(numerator: i64, denominator: i64) -> Self {
        Self::checked_from_i128(numerator as i128, denominator as i128)
            .unwrap_or(Self::ZERO)
    }

    pub fn checked_from_i128(mut numerator: i128, mut denominator: i128) -> Option<Self> {
        if denominator == 0 {
            numerator = 0;
            denominator = 1;
        }

        if denominator < 0 {
            numerator = -numerator;
            denominator = -denominator;
        }

        let g = gcd_i128(numerator, denominator);
        if g > 1 {
            numerator /= g;
            denominator /= g;
        }

        if !fits_i64(numerator) || !fits_i64(denominator) {
            return None;
        }

        Some(Self {
            numerator: numerator as i64,
            denominator: denominator as i64,
        })
    }

    pub fn numerator(self) -> i64 {
        self.numerator
    }

    pub fn denominator(self) -> i64 {
        self.denominator
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let num = (self.numerator as i128 * rhs.denominator as i128)
            + (rhs.numerator as i128 * self.denominator as i128);
        let den = self.denominator as i128 * rhs.denominator as i128;
        Self::checked_from_i128(num, den)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let num = (self.numerator as i128 * rhs.denominator as i128)
            - (rhs.numerator as i128 * self.denominator as i128);
        let den = self.denominator as i128 * rhs.denominator as i128;
        Self::checked_from_i128(num, den)
    }

    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let num = self.numerator as i128 * rhs.numerator as i128;
        let den = self.denominator as i128 * rhs.denominator as i128;
        Self::checked_from_i128(num, den)
    }

    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.numerator == 0 {
            return None;
        }

        let num = self.numerator as i128 * rhs.denominator as i128;
        let den = self.denominator as i128 * rhs.numerator as i128;
        Self::checked_from_i128(num, den)
    }

    pub fn compare(self, rhs: Self) -> Ordering {
        let lhs = self.numerator as i128 * rhs.denominator as i128;
        let rhs = rhs.numerator as i128 * self.denominator as i128;
        lhs.cmp(&rhs)
    }

    pub fn is_integer(self) -> bool {
        self.denominator == 1
    }

    pub fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    pub fn from_f64_exact(d: f64) -> Self {
        if d == 0.0 || !d.is_finite() {
            return Self::ZERO;
        }

        let mut abs_d = d.abs();
        let mut den: i64 = 1;
        while abs_d.fract() != 0.0 && den < (1_i64 << 52) {
            abs_d *= 2.0;
            den *= 2;
        }

        let mut num = abs_d as i64;
        if d < 0.0 {
            num = -num;
        }

        Self::new(num, den)
    }

    pub fn floor(self) -> i64 {
        let n = self.numerator;
        let d = self.denominator;
        if n >= 0 || n % d == 0 {
            n / d
        } else {
            n / d - 1
        }
    }

    pub fn ceil(self) -> i64 {
        let n = self.numerator;
        let d = self.denominator;
        if n <= 0 || n % d == 0 {
            n / d
        } else {
            n / d + 1
        }
    }

    pub fn truncate(self) -> i64 {
        self.numerator / self.denominator
    }

    pub fn round(self) -> i64 {
        let n = self.numerator;
        let d = self.denominator;
        let q = n / d;

        let mut rem = n % d;
        if rem < 0 {
            rem = -rem;
        }

        let twice_rem = rem as i128 * 2;
        let d128 = d as i128;

        if twice_rem > d128 {
            if n >= 0 { q + 1 } else { q - 1 }
        } else if twice_rem == d128 {
            if q % 2 != 0 {
                if n >= 0 { q + 1 } else { q - 1 }
            } else {
                q
            }
        } else {
            q
        }
    }
}

impl Display for Rational {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
    if a < 0 {
        a = -a;
    }
    if b < 0 {
        b = -b;
    }

    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }

    a
}

fn fits_i64(v: i128) -> bool {
    v >= i64::MIN as i128 && v <= i64::MAX as i128
}

#[cfg(test)]
mod tests {
    use super::Rational;
    use std::cmp::Ordering;

    #[test]
    fn normalizes_sign_and_reduces() {
        let r = Rational::new(-4, -6);
        assert_eq!(r.numerator(), 2);
        assert_eq!(r.denominator(), 3);

        let r2 = Rational::new(4, -6);
        assert_eq!(r2.numerator(), -2);
        assert_eq!(r2.denominator(), 3);
    }

    #[test]
    fn zero_denominator_falls_back_to_zero() {
        let r = Rational::new(7, 0);
        assert_eq!(r.numerator(), 0);
        assert_eq!(r.denominator(), 1);
    }

    #[test]
    fn arithmetic_matches_expected_values() {
        let a = Rational::new(1, 2);
        let b = Rational::new(1, 3);

        assert_eq!(a.checked_add(b).unwrap(), Rational::new(5, 6));
        assert_eq!(a.checked_sub(b).unwrap(), Rational::new(1, 6));
        assert_eq!(a.checked_mul(b).unwrap(), Rational::new(1, 6));
        assert_eq!(a.checked_div(b).unwrap(), Rational::new(3, 2));
    }

    #[test]
    fn checked_div_none_on_divide_by_zero() {
        let a = Rational::new(1, 2);
        let zero = Rational::new(0, 1);
        assert!(a.checked_div(zero).is_none());
    }

    #[test]
    fn returns_none_on_overflow() {
        let a = Rational::new(i64::MAX, 1);
        let b = Rational::new(1, 1);
        assert!(a.checked_add(b).is_none());
    }

    #[test]
    fn compare_works() {
        let a = Rational::new(5, 7);
        let b = Rational::new(10, 14);
        let c = Rational::new(3, 4);

        assert_eq!(a.compare(b), Ordering::Equal);
        assert_eq!(a.compare(c), Ordering::Less);
        assert_eq!(c.compare(a), Ordering::Greater);
    }

    #[test]
    fn integer_and_string_views() {
        let a = Rational::new(6, 3);
        let b = Rational::new(7, 5);
        assert!(a.is_integer());
        assert!(!b.is_integer());
        assert_eq!(a.to_string(), "2");
        assert_eq!(b.to_string(), "7/5");
    }

    #[test]
    fn exact_conversion_from_f64() {
        let a = Rational::from_f64_exact(0.75);
        let b = Rational::from_f64_exact(-1.5);

        assert_eq!(a, Rational::new(3, 4));
        assert_eq!(b, Rational::new(-3, 2));
    }

    #[test]
    fn rounding_behaviors_match_r7rs_style() {
        let p = Rational::new(7, 3);   // 2.333...
        let n = Rational::new(-7, 3);  // -2.333...
        let tie_pos_even = Rational::new(5, 2); // 2.5
        let tie_neg_even = Rational::new(-5, 2); // -2.5

        assert_eq!(p.floor(), 2);
        assert_eq!(p.ceil(), 3);
        assert_eq!(n.floor(), -3);
        assert_eq!(n.ceil(), -2);
        assert_eq!(n.truncate(), -2);
        assert_eq!(tie_pos_even.round(), 2);
        assert_eq!(tie_neg_even.round(), -2);
    }
}
