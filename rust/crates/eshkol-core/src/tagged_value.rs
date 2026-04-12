use crate::rational::Rational;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

pub const ESHKOL_VALUE_NULL: u8 = 0;
pub const ESHKOL_VALUE_INT64: u8 = 1;
pub const ESHKOL_VALUE_DOUBLE: u8 = 2;
pub const ESHKOL_VALUE_BOOL: u8 = 3;
pub const ESHKOL_VALUE_HEAP_PTR: u8 = 8;

pub const ESHKOL_VALUE_EXACT_FLAG: u8 = 0x10;
pub const ESHKOL_VALUE_INEXACT_FLAG: u8 = 0x20;

#[derive(Debug, Clone, PartialEq)]
pub enum TaggedValue {
    Null,
    Int64 { value: i64, exact: bool },
    Double(f64),
    Bool(bool),
    Rational(Rational),
}

impl TaggedValue {
    pub fn null() -> Self {
        Self::Null
    }

    pub fn exact_int(value: i64) -> Self {
        Self::Int64 { value, exact: true }
    }

    pub fn inexact_int(value: i64) -> Self {
        Self::Int64 {
            value,
            exact: false,
        }
    }

    pub fn double(value: f64) -> Self {
        Self::Double(value)
    }

    pub fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    pub fn rational(value: Rational) -> Self {
        if value.is_integer() {
            Self::exact_int(value.numerator())
        } else {
            Self::Rational(value)
        }
    }

    pub fn type_tag(&self) -> u8 {
        match self {
            TaggedValue::Null => ESHKOL_VALUE_NULL,
            TaggedValue::Int64 { .. } => ESHKOL_VALUE_INT64,
            TaggedValue::Double(_) => ESHKOL_VALUE_DOUBLE,
            TaggedValue::Bool(_) => ESHKOL_VALUE_BOOL,
            TaggedValue::Rational(_) => ESHKOL_VALUE_HEAP_PTR,
        }
    }

    pub fn flags(&self) -> u8 {
        match self {
            TaggedValue::Int64 { exact: true, .. } | TaggedValue::Rational(_) => {
                ESHKOL_VALUE_EXACT_FLAG
            }
            TaggedValue::Double(_) => ESHKOL_VALUE_INEXACT_FLAG,
            _ => 0,
        }
    }

    pub fn as_exact_rational(&self) -> Option<Rational> {
        match self {
            TaggedValue::Int64 { value, .. } => Some(Rational::new(*value, 1)),
            TaggedValue::Rational(r) => Some(*r),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            TaggedValue::Int64 { value, .. } => Some(*value as f64),
            TaggedValue::Double(value) => Some(*value),
            TaggedValue::Rational(r) => Some(r.to_f64()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TaggedValue::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

impl Display for TaggedValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaggedValue::Null => write!(f, "()"),
            TaggedValue::Int64 { value, .. } => write!(f, "{value}"),
            TaggedValue::Double(value) => write!(f, "{value}"),
            TaggedValue::Bool(true) => write!(f, "#t"),
            TaggedValue::Bool(false) => write!(f, "#f"),
            TaggedValue::Rational(r) => write!(f, "{r}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Lt,
    Gt,
    Eq,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericError {
    NonNumericOperand,
    DivideByZero,
}

impl Display for NumericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NumericError::NonNumericOperand => write!(f, "non-numeric operand"),
            NumericError::DivideByZero => write!(f, "division by zero"),
        }
    }
}

impl std::error::Error for NumericError {}

pub fn apply_numeric_op(
    lhs: &TaggedValue,
    rhs: &TaggedValue,
    op: NumericOp,
) -> Result<TaggedValue, NumericError> {
    let use_double = matches!(lhs, TaggedValue::Double(_)) || matches!(rhs, TaggedValue::Double(_));

    if use_double {
        let da = lhs.as_f64().ok_or(NumericError::NonNumericOperand)?;
        let db = rhs.as_f64().ok_or(NumericError::NonNumericOperand)?;
        let out = match op {
            NumericOp::Add => da + db,
            NumericOp::Sub => da - db,
            NumericOp::Mul => da * db,
            NumericOp::Div => da / db,
        };
        return Ok(TaggedValue::double(out));
    }

    let ra = lhs
        .as_exact_rational()
        .ok_or(NumericError::NonNumericOperand)?;
    let rb = rhs
        .as_exact_rational()
        .ok_or(NumericError::NonNumericOperand)?;

    if matches!(op, NumericOp::Div) && rb.numerator() == 0 {
        return Err(NumericError::DivideByZero);
    }

    let exact = match op {
        NumericOp::Add => ra.checked_add(rb),
        NumericOp::Sub => ra.checked_sub(rb),
        NumericOp::Mul => ra.checked_mul(rb),
        NumericOp::Div => ra.checked_div(rb),
    };

    if let Some(result) = exact {
        return Ok(TaggedValue::rational(result));
    }

    // Overflow fallback to inexact arithmetic, matching current C/C++ behavior.
    let da = ra.to_f64();
    let db = rb.to_f64();
    let out = match op {
        NumericOp::Add => da + db,
        NumericOp::Sub => da - db,
        NumericOp::Mul => da * db,
        NumericOp::Div => da / db,
    };
    Ok(TaggedValue::double(out))
}

pub fn apply_compare_op(
    lhs: &TaggedValue,
    rhs: &TaggedValue,
    op: CompareOp,
) -> Result<TaggedValue, NumericError> {
    let use_double = matches!(lhs, TaggedValue::Double(_)) || matches!(rhs, TaggedValue::Double(_));

    if use_double {
        let da = lhs.as_f64().ok_or(NumericError::NonNumericOperand)?;
        let db = rhs.as_f64().ok_or(NumericError::NonNumericOperand)?;
        let result = match op {
            CompareOp::Lt => da < db,
            CompareOp::Gt => da > db,
            CompareOp::Eq => da == db,
            CompareOp::Le => da <= db,
            CompareOp::Ge => da >= db,
        };
        return Ok(TaggedValue::bool(result));
    }

    let ra = lhs
        .as_exact_rational()
        .ok_or(NumericError::NonNumericOperand)?;
    let rb = rhs
        .as_exact_rational()
        .ok_or(NumericError::NonNumericOperand)?;

    let ordering = ra.compare(rb);
    let result = match op {
        CompareOp::Lt => ordering == Ordering::Less,
        CompareOp::Gt => ordering == Ordering::Greater,
        CompareOp::Eq => ordering == Ordering::Equal,
        CompareOp::Le => ordering != Ordering::Greater,
        CompareOp::Ge => ordering != Ordering::Less,
    };
    Ok(TaggedValue::bool(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_flags_match_expected_layout() {
        let int = TaggedValue::exact_int(3);
        let inexact = TaggedValue::double(3.5);
        let rational = TaggedValue::rational(Rational::new(3, 2));

        assert_eq!(int.type_tag(), ESHKOL_VALUE_INT64);
        assert_eq!(int.flags(), ESHKOL_VALUE_EXACT_FLAG);

        assert_eq!(inexact.type_tag(), ESHKOL_VALUE_DOUBLE);
        assert_eq!(inexact.flags(), ESHKOL_VALUE_INEXACT_FLAG);

        assert_eq!(rational.type_tag(), ESHKOL_VALUE_HEAP_PTR);
        assert_eq!(rational.flags(), ESHKOL_VALUE_EXACT_FLAG);
    }

    #[test]
    fn exact_numeric_dispatch_prefers_rational_and_reduces() {
        let lhs = TaggedValue::exact_int(1);
        let rhs = TaggedValue::rational(Rational::new(1, 2));

        let out = apply_numeric_op(&lhs, &rhs, NumericOp::Add).unwrap();
        assert_eq!(out, TaggedValue::rational(Rational::new(3, 2)));
    }

    #[test]
    fn inexact_promotes_to_double() {
        let lhs = TaggedValue::exact_int(1);
        let rhs = TaggedValue::double(0.25);

        let out = apply_numeric_op(&lhs, &rhs, NumericOp::Add).unwrap();
        assert_eq!(out, TaggedValue::double(1.25));
    }

    #[test]
    fn exact_overflow_falls_back_to_double() {
        let lhs = TaggedValue::exact_int(i64::MAX);
        let rhs = TaggedValue::exact_int(2);
        let out = apply_numeric_op(&lhs, &rhs, NumericOp::Add).unwrap();

        match out {
            TaggedValue::Double(v) => assert!(v.is_finite()),
            _ => panic!("expected double fallback"),
        }
    }

    #[test]
    fn exact_divide_by_zero_errors() {
        let lhs = TaggedValue::exact_int(10);
        let rhs = TaggedValue::exact_int(0);

        let err = apply_numeric_op(&lhs, &rhs, NumericOp::Div).unwrap_err();
        assert_eq!(err, NumericError::DivideByZero);
    }

    #[test]
    fn comparison_dispatch_handles_exact_and_inexact() {
        let a = TaggedValue::rational(Rational::new(3, 4));
        let b = TaggedValue::exact_int(1);

        let lt = apply_compare_op(&a, &b, CompareOp::Lt).unwrap();
        assert_eq!(lt.as_bool(), Some(true));

        let eq = apply_compare_op(&TaggedValue::double(1.0), &b, CompareOp::Eq).unwrap();
        assert_eq!(eq.as_bool(), Some(true));
    }
}
