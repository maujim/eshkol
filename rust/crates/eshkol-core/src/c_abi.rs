use crate::tagged_value::{
    TaggedValue, ESHKOL_VALUE_BOOL, ESHKOL_VALUE_DOUBLE, ESHKOL_VALUE_EXACT_FLAG,
    ESHKOL_VALUE_INEXACT_FLAG, ESHKOL_VALUE_INT64, ESHKOL_VALUE_NULL,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub union EshkolTaggedData {
    pub int_val: i64,
    pub double_val: f64,
    pub ptr_val: u64,
    pub raw_val: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EshkolTaggedValue {
    pub value_type: u8,
    pub flags: u8,
    pub reserved: u16,
    pub data: EshkolTaggedData,
}

impl EshkolTaggedValue {
    pub fn make_null() -> Self {
        Self {
            value_type: ESHKOL_VALUE_NULL,
            flags: 0,
            reserved: 0,
            data: EshkolTaggedData { raw_val: 0 },
        }
    }

    pub fn from_high_level(value: &TaggedValue) -> Option<Self> {
        match value {
            TaggedValue::Null => Some(Self::make_null()),
            TaggedValue::Int64 { value, exact } => Some(Self {
                value_type: ESHKOL_VALUE_INT64,
                flags: if *exact { ESHKOL_VALUE_EXACT_FLAG } else { 0 },
                reserved: 0,
                data: EshkolTaggedData { int_val: *value },
            }),
            TaggedValue::Double(v) => Some(Self {
                value_type: ESHKOL_VALUE_DOUBLE,
                flags: ESHKOL_VALUE_INEXACT_FLAG,
                reserved: 0,
                data: EshkolTaggedData { double_val: *v },
            }),
            TaggedValue::Bool(b) => Some(Self {
                value_type: ESHKOL_VALUE_BOOL,
                flags: 0,
                reserved: 0,
                data: EshkolTaggedData {
                    int_val: if *b { 1 } else { 0 },
                },
            }),
            TaggedValue::Rational(_) => None,
        }
    }

    /// # Safety
    ///
    /// Reads from a C-layout union field according to `value_type`; callers must ensure
    /// the tagged value was initialized with a matching payload.
    pub unsafe fn to_high_level(self) -> Option<TaggedValue> {
        match self.value_type {
            ESHKOL_VALUE_NULL => Some(TaggedValue::null()),
            ESHKOL_VALUE_INT64 => {
                let exact = (self.flags & ESHKOL_VALUE_EXACT_FLAG) != 0;
                // SAFETY: union field matches `ESHKOL_VALUE_INT64` tag.
                let value = unsafe { self.data.int_val };
                if exact {
                    Some(TaggedValue::exact_int(value))
                } else {
                    Some(TaggedValue::inexact_int(value))
                }
            }
            ESHKOL_VALUE_DOUBLE => {
                // SAFETY: union field matches `ESHKOL_VALUE_DOUBLE` tag.
                let value = unsafe { self.data.double_val };
                Some(TaggedValue::double(value))
            }
            ESHKOL_VALUE_BOOL => {
                // SAFETY: bool is stored in the integer field in C runtime conventions.
                let value = unsafe { self.data.int_val };
                Some(TaggedValue::bool(value != 0))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn tagged_value_layout_matches_c_contract() {
        assert_eq!(size_of::<EshkolTaggedValue>(), 16);
        assert_eq!(align_of::<EshkolTaggedValue>(), 8);
    }

    #[test]
    fn roundtrip_supported_values() {
        let samples = [
            TaggedValue::null(),
            TaggedValue::exact_int(42),
            TaggedValue::inexact_int(7),
            TaggedValue::double(3.25),
            TaggedValue::bool(true),
            TaggedValue::bool(false),
        ];

        for sample in samples {
            let c = EshkolTaggedValue::from_high_level(&sample).expect("convert to c abi");
            // SAFETY: value was created from a matching high-level variant.
            let back = unsafe { c.to_high_level() }.expect("convert from c abi");
            assert_eq!(back, sample);
        }
    }

    #[test]
    fn rational_is_not_directly_lowered_without_heap_pointer() {
        let r = TaggedValue::rational(crate::rational::Rational::new(3, 2));
        assert!(EshkolTaggedValue::from_high_level(&r).is_none());
    }

    #[test]
    fn unknown_type_returns_none() {
        let c = EshkolTaggedValue {
            value_type: 255,
            flags: 0,
            reserved: 0,
            data: EshkolTaggedData { raw_val: 0 },
        };

        // SAFETY: unknown types are handled by returning None.
        assert!(unsafe { c.to_high_level() }.is_none());
    }
}
