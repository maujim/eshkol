use eshkol_core::c_abi::{EshkolTaggedData, EshkolTaggedValue};
use eshkol_core::tagged_value::{
    ESHKOL_VALUE_BOOL, ESHKOL_VALUE_DOUBLE, ESHKOL_VALUE_EXACT_FLAG, ESHKOL_VALUE_HEAP_PTR,
    ESHKOL_VALUE_INEXACT_FLAG, ESHKOL_VALUE_INT64, ESHKOL_VALUE_NULL,
};
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ffi::{c_char, c_void, CStr};
use std::mem::{align_of, size_of};
use std::ptr::{self, NonNull};

pub const HEAP_SUBTYPE_STRING: u8 = 1;
pub const HEAP_SUBTYPE_VECTOR: u8 = 2;
pub const HEAP_SUBTYPE_HASH: u8 = 5;
pub const HEAP_SUBTYPE_RECORD: u8 = 7;
pub const HEAP_SUBTYPE_PORT: u8 = 9;
pub const HEAP_SUBTYPE_SYMBOL: u8 = 10;
pub const HEAP_SUBTYPE_PROMISE: u8 = 18;
pub const HEAP_SUBTYPE_RATIONAL: u8 = 19;

pub const ESHKOL_OBJ_FLAG_MARKED: u8 = 0x01;
pub const ESHKOL_OBJ_FLAG_LINEAR: u8 = 0x02;
pub const ESHKOL_OBJ_FLAG_BORROWED: u8 = 0x04;
pub const ESHKOL_OBJ_FLAG_CONSUMED: u8 = 0x08;
pub const ESHKOL_OBJ_FLAG_SHARED: u8 = 0x10;
pub const ESHKOL_OBJ_FLAG_WEAK: u8 = 0x20;
pub const ESHKOL_OBJ_FLAG_PINNED: u8 = 0x40;
pub const ESHKOL_OBJ_FLAG_EXTERNAL: u8 = 0x80;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EshkolObjectHeader {
    pub subtype: u8,
    pub flags: u8,
    pub ref_count: u16,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EshkolRational {
    pub numerator: i64,
    pub denominator: i64,
}

#[derive(Debug)]
struct ArenaAllocation {
    ptr: NonNull<u8>,
    layout: Layout,
}

#[derive(Debug)]
pub struct Arena {
    default_block_size: usize,
    used_bytes: usize,
    allocations: Vec<ArenaAllocation>,
}

impl Arena {
    fn new(default_block_size: usize) -> Self {
        Self {
            default_block_size,
            used_bytes: 0,
            allocations: Vec::new(),
        }
    }

    fn allocate_raw_zeroed(&mut self, size: usize, align: usize) -> *mut u8 {
        if size == 0 {
            return ptr::null_mut();
        }

        let Ok(layout) = Layout::from_size_align(size, align.max(1)) else {
            return ptr::null_mut();
        };

        // SAFETY: `layout` is validated above.
        let raw = unsafe { alloc_zeroed(layout) };
        let Some(ptr) = NonNull::new(raw) else {
            return ptr::null_mut();
        };

        self.used_bytes = self.used_bytes.saturating_add(size);
        self.allocations.push(ArenaAllocation { ptr, layout });
        ptr.as_ptr()
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        for allocation in self.allocations.drain(..) {
            // SAFETY: Every allocation was created via `alloc_zeroed` with this exact layout
            // and is deallocated exactly once in arena drop.
            unsafe { dealloc(allocation.ptr.as_ptr(), allocation.layout) };
        }
    }
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
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

fn normalize_rational(mut numerator: i64, mut denominator: i64) -> (i64, i64) {
    if denominator == 0 {
        return (0, 1);
    }

    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }

    let g = gcd_i64(numerator, denominator);
    if g > 1 {
        (numerator / g, denominator / g)
    } else {
        (numerator, denominator)
    }
}

fn is_known_heap_subtype(subtype: u8) -> bool {
    matches!(subtype, 0..=13 | 15..=19)
}

fn allows_ref_count_mutation(subtype: u8) -> bool {
    matches!(
        subtype,
        HEAP_SUBTYPE_STRING
            | HEAP_SUBTYPE_SYMBOL
            | HEAP_SUBTYPE_VECTOR
            | HEAP_SUBTYPE_HASH
            | HEAP_SUBTYPE_RECORD
            | HEAP_SUBTYPE_PORT
            | HEAP_SUBTYPE_PROMISE
    )
}

fn allowed_flag_mask_for_subtype(subtype: u8) -> u8 {
    match subtype {
        // Exact numeric heap objects: no sharing/external lifecycle flags.
        HEAP_SUBTYPE_RATIONAL => {
            ESHKOL_OBJ_FLAG_MARKED
                | ESHKOL_OBJ_FLAG_LINEAR
                | ESHKOL_OBJ_FLAG_BORROWED
                | ESHKOL_OBJ_FLAG_CONSUMED
        }
        // Text-like heap objects: allow sharing/lifecycle flags, disallow linearity semantics.
        HEAP_SUBTYPE_STRING | HEAP_SUBTYPE_SYMBOL => {
            ESHKOL_OBJ_FLAG_MARKED
                | ESHKOL_OBJ_FLAG_SHARED
                | ESHKOL_OBJ_FLAG_WEAK
                | ESHKOL_OBJ_FLAG_PINNED
                | ESHKOL_OBJ_FLAG_EXTERNAL
        }
        // Default: full flag surface (existing behavior).
        _ => 0xFF,
    }
}

unsafe fn validated_header_from_data_ptr(data_ptr: *const c_void) -> Option<*mut EshkolObjectHeader> {
    if data_ptr.is_null() {
        return None;
    }

    // SAFETY: caller provides pointer to data region with preceding object header.
    let header_ptr = unsafe {
        (data_ptr as *const u8)
            .sub(size_of::<EshkolObjectHeader>())
            .cast::<EshkolObjectHeader>() as *mut EshkolObjectHeader
    };

    // SAFETY: `header_ptr` derives from caller-provided object data pointer.
    let subtype = unsafe { (*header_ptr).subtype };
    if !is_known_heap_subtype(subtype) {
        return None;
    }

    Some(header_ptr)
}

unsafe fn validated_header_for_ref_count_mutation(
    data_ptr: *const c_void,
) -> Option<*mut EshkolObjectHeader> {
    let header_ptr = unsafe { validated_header_from_data_ptr(data_ptr) }?;
    // SAFETY: validated above.
    let subtype = unsafe { (*header_ptr).subtype };
    if !allows_ref_count_mutation(subtype) {
        return None;
    }
    Some(header_ptr)
}

unsafe fn validated_header_for_flag_mutation(
    data_ptr: *const c_void,
) -> Option<(*mut EshkolObjectHeader, u8)> {
    let header_ptr = unsafe { validated_header_from_data_ptr(data_ptr) }?;
    // SAFETY: validated above.
    let subtype = unsafe { (*header_ptr).subtype };
    Some((header_ptr, allowed_flag_mask_for_subtype(subtype)))
}

unsafe fn allocate_cstr_with_subtype(
    arena: *mut Arena,
    c_str: *const c_char,
    subtype: u8,
) -> *mut c_char {
    if arena.is_null() || c_str.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: `c_str` must be a valid NUL-terminated string from caller.
    let bytes = unsafe { CStr::from_ptr(c_str) }.to_bytes();
    let len_plus_nul = bytes.len().saturating_add(1);

    // SAFETY: allocator validates arena pointer and size.
    let data_ptr = unsafe { arena_allocate_with_header(arena, len_plus_nul, subtype, 0) } as *mut u8;
    if data_ptr.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: allocation size is len_plus_nul bytes.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), data_ptr, bytes.len());
        *data_ptr.add(bytes.len()) = 0;
    }

    data_ptr as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn arena_create(default_block_size: usize) -> *mut Arena {
    Box::into_raw(Box::new(Arena::new(default_block_size)))
}

#[no_mangle]
pub unsafe extern "C" fn arena_destroy(arena: *mut Arena) {
    if arena.is_null() {
        return;
    }

    // SAFETY: pointer was returned by `arena_create` and is consumed exactly once here.
    unsafe { drop(Box::from_raw(arena)) };
}

#[no_mangle]
pub unsafe extern "C" fn arena_allocate(arena: *mut Arena, size: usize) -> *mut c_void {
    if arena.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: null-checked above.
    let arena = unsafe { &mut *arena };
    arena.allocate_raw_zeroed(size, align_of::<u64>()) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn arena_allocate_with_header(
    arena: *mut Arena,
    data_size: usize,
    subtype: u8,
    flags: u8,
) -> *mut c_void {
    if arena.is_null() || data_size > u32::MAX as usize {
        return ptr::null_mut();
    }

    let Some(total_size) = size_of::<EshkolObjectHeader>().checked_add(data_size) else {
        return ptr::null_mut();
    };

    // SAFETY: null-checked above.
    let arena = unsafe { &mut *arena };
    let raw = arena.allocate_raw_zeroed(total_size, align_of::<u64>());
    if raw.is_null() {
        return ptr::null_mut();
    }

    let header = EshkolObjectHeader {
        subtype,
        flags,
        ref_count: 0,
        size: data_size as u32,
    };

    // SAFETY: `raw` points to at least `size_of::<EshkolObjectHeader>()` bytes and is properly
    // aligned by the allocator call above.
    unsafe {
        raw.cast::<EshkolObjectHeader>().write(header);
        raw.add(size_of::<EshkolObjectHeader>()) as *mut c_void
    }
}

#[no_mangle]
pub unsafe extern "C" fn arena_allocate_with_header_zeroed(
    arena: *mut Arena,
    data_size: usize,
    subtype: u8,
    flags: u8,
) -> *mut c_void {
    // Allocator uses zeroed allocations by default.
    unsafe { arena_allocate_with_header(arena, data_size, subtype, flags) }
}

#[no_mangle]
pub unsafe extern "C" fn arena_allocate_string_with_header(
    arena: *mut Arena,
    length: usize,
) -> *mut c_char {
    // SAFETY: delegated to header allocation function.
    unsafe { arena_allocate_with_header(arena, length, HEAP_SUBTYPE_STRING, 0) as *mut c_char }
}

#[no_mangle]
pub unsafe extern "C" fn arena_get_used_memory(arena: *const Arena) -> usize {
    if arena.is_null() {
        return 0;
    }

    // SAFETY: null-checked above.
    unsafe { (*arena).used_bytes }
}

#[no_mangle]
pub unsafe extern "C" fn arena_get_block_count(arena: *const Arena) -> usize {
    if arena.is_null() {
        return 0;
    }

    // SAFETY: null-checked above.
    unsafe { (*arena).allocations.len() }
}

#[no_mangle]
pub unsafe extern "C" fn arena_get_default_block_size(arena: *const Arena) -> usize {
    if arena.is_null() {
        return 0;
    }

    // SAFETY: null-checked above.
    unsafe { (*arena).default_block_size }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_get_header_from_data_ptr(
    data_ptr: *mut c_void,
) -> *mut EshkolObjectHeader {
    if data_ptr.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: caller guarantees `data_ptr` comes from arena allocation with prepended header.
    unsafe { (data_ptr as *mut u8).sub(size_of::<EshkolObjectHeader>()) as *mut EshkolObjectHeader }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_get_subtype_from_data_ptr(data_ptr: *const c_void) -> u8 {
    if data_ptr.is_null() {
        return 0;
    }

    // SAFETY: caller guarantees `data_ptr` points to object data with a valid header before it.
    unsafe {
        let header_ptr = (data_ptr as *const u8).sub(size_of::<EshkolObjectHeader>());
        (*header_ptr.cast::<EshkolObjectHeader>()).subtype
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_get_flags_from_data_ptr(data_ptr: *const c_void) -> u8 {
    if data_ptr.is_null() {
        return 0;
    }

    // SAFETY: caller guarantees `data_ptr` points to object data with a valid header before it.
    unsafe {
        let header_ptr = (data_ptr as *const u8).sub(size_of::<EshkolObjectHeader>());
        (*header_ptr.cast::<EshkolObjectHeader>()).flags
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_validate(data_ptr: *const c_void) -> u8 {
    if unsafe { validated_header_from_data_ptr(data_ptr) }.is_some() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_ref_count(data_ptr: *const c_void) -> u16 {
    let Some(header_ptr) = (unsafe { validated_header_from_data_ptr(data_ptr) }) else {
        return 0;
    };
    // SAFETY: pointer validated above.
    unsafe { (*header_ptr).ref_count }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_set_ref_count(
    data_ptr: *const c_void,
    value: u16,
) -> u16 {
    let Some(header_ptr) = (unsafe { validated_header_from_data_ptr(data_ptr) }) else {
        return 0;
    };

    if (unsafe { validated_header_for_ref_count_mutation(data_ptr) }).is_none() {
        // SAFETY: pointer validated above.
        return unsafe { (*header_ptr).ref_count };
    }

    // SAFETY: pointer validated above and subtype policy allows mutation.
    unsafe {
        (*header_ptr).ref_count = value;
        (*header_ptr).ref_count
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_inc_ref_count(data_ptr: *const c_void) -> u16 {
    let Some(header_ptr) = (unsafe { validated_header_from_data_ptr(data_ptr) }) else {
        return 0;
    };

    if (unsafe { validated_header_for_ref_count_mutation(data_ptr) }).is_none() {
        // SAFETY: pointer validated above.
        return unsafe { (*header_ptr).ref_count };
    }

    // SAFETY: pointer validated above and subtype policy allows mutation.
    unsafe {
        (*header_ptr).ref_count = (*header_ptr).ref_count.saturating_add(1);
        (*header_ptr).ref_count
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_dec_ref_count(data_ptr: *const c_void) -> u16 {
    let Some(header_ptr) = (unsafe { validated_header_from_data_ptr(data_ptr) }) else {
        return 0;
    };

    if (unsafe { validated_header_for_ref_count_mutation(data_ptr) }).is_none() {
        // SAFETY: pointer validated above.
        return unsafe { (*header_ptr).ref_count };
    }

    // SAFETY: pointer validated above and subtype policy allows mutation.
    unsafe {
        (*header_ptr).ref_count = (*header_ptr).ref_count.saturating_sub(1);
        (*header_ptr).ref_count
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_set_flags(data_ptr: *const c_void, flags: u8) -> u8 {
    let Some((header_ptr, allowed_mask)) = (unsafe { validated_header_for_flag_mutation(data_ptr) }) else {
        return 0;
    };
    // SAFETY: pointer validated above.
    unsafe {
        let preserved_disallowed = (*header_ptr).flags & !allowed_mask;
        (*header_ptr).flags = preserved_disallowed | (flags & allowed_mask);
        (*header_ptr).flags
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_add_flags(data_ptr: *const c_void, flags: u8) -> u8 {
    let Some((header_ptr, allowed_mask)) = (unsafe { validated_header_for_flag_mutation(data_ptr) }) else {
        return 0;
    };
    // SAFETY: pointer validated above.
    unsafe {
        (*header_ptr).flags |= flags & allowed_mask;
        (*header_ptr).flags
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_header_clear_flags(data_ptr: *const c_void, flags: u8) -> u8 {
    let Some((header_ptr, allowed_mask)) = (unsafe { validated_header_for_flag_mutation(data_ptr) }) else {
        return 0;
    };
    // SAFETY: pointer validated above.
    unsafe {
        (*header_ptr).flags &= !(flags & allowed_mask);
        (*header_ptr).flags
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Namespaced C ABI wrappers (avoid symbol collisions with existing C runtime)
// ─────────────────────────────────────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_arena_create(default_block_size: usize) -> *mut Arena {
    unsafe { arena_create(default_block_size) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_arena_destroy(arena: *mut Arena) {
    unsafe { arena_destroy(arena) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_arena_allocate(arena: *mut Arena, size: usize) -> *mut c_void {
    unsafe { arena_allocate(arena, size) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_arena_allocate_with_header(
    arena: *mut Arena,
    data_size: usize,
    subtype: u8,
    flags: u8,
) -> *mut c_void {
    unsafe { arena_allocate_with_header(arena, data_size, subtype, flags) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_arena_allocate_string_with_header(
    arena: *mut Arena,
    length: usize,
) -> *mut c_char {
    unsafe { arena_allocate_string_with_header(arena, length) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_arena_get_used_memory(arena: *const Arena) -> usize {
    unsafe { arena_get_used_memory(arena) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_get_subtype_from_data_ptr(data_ptr: *const c_void) -> u8 {
    unsafe { eshkol_get_subtype_from_data_ptr(data_ptr) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_get_flags_from_data_ptr(data_ptr: *const c_void) -> u8 {
    unsafe { eshkol_get_flags_from_data_ptr(data_ptr) }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_rational_normalize_into(
    numerator: i64,
    denominator: i64,
    out_numerator: *mut i64,
    out_denominator: *mut i64,
) {
    let (n, d) = normalize_rational(numerator, denominator);

    // SAFETY: caller provides optional out-pointers for writeback.
    unsafe {
        if !out_numerator.is_null() {
            *out_numerator = n;
        }
        if !out_denominator.is_null() {
            *out_denominator = d;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_rational_create(
    arena: *mut Arena,
    numerator: i64,
    denominator: i64,
) -> *mut EshkolRational {
    if arena.is_null() {
        return ptr::null_mut();
    }

    let (n, d) = normalize_rational(numerator, denominator);
    // SAFETY: null checked above and allocation routine validates size.
    let data_ptr = unsafe {
        arena_allocate_with_header(
            arena,
            size_of::<EshkolRational>(),
            HEAP_SUBTYPE_RATIONAL,
            0,
        )
    } as *mut EshkolRational;

    if data_ptr.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: `data_ptr` points to a valid allocation large enough for EshkolRational.
    unsafe {
        data_ptr.write(EshkolRational {
            numerator: n,
            denominator: d,
        });
    }

    data_ptr
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_string_create(
    arena: *mut Arena,
    c_str: *const c_char,
) -> *mut c_char {
    unsafe { allocate_cstr_with_subtype(arena, c_str, HEAP_SUBTYPE_STRING) }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_symbol_create(
    arena: *mut Arena,
    c_str: *const c_char,
) -> *mut c_char {
    unsafe { allocate_cstr_with_subtype(arena, c_str, HEAP_SUBTYPE_SYMBOL) }
}

// Tagged-value helper exports for C call sites.
#[no_mangle]
pub extern "C" fn eshkol_rust_tv_make_null() -> EshkolTaggedValue {
    EshkolTaggedValue {
        value_type: ESHKOL_VALUE_NULL,
        flags: 0,
        reserved: 0,
        data: EshkolTaggedData { raw_val: 0 },
    }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_make_int64(value: i64, exact: u8) -> EshkolTaggedValue {
    EshkolTaggedValue {
        value_type: ESHKOL_VALUE_INT64,
        flags: if exact != 0 { ESHKOL_VALUE_EXACT_FLAG } else { 0 },
        reserved: 0,
        data: EshkolTaggedData { int_val: value },
    }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_make_double(value: f64) -> EshkolTaggedValue {
    EshkolTaggedValue {
        value_type: ESHKOL_VALUE_DOUBLE,
        flags: ESHKOL_VALUE_INEXACT_FLAG,
        reserved: 0,
        data: EshkolTaggedData { double_val: value },
    }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_make_bool(value: u8) -> EshkolTaggedValue {
    EshkolTaggedValue {
        value_type: ESHKOL_VALUE_BOOL,
        flags: 0,
        reserved: 0,
        data: EshkolTaggedData {
            int_val: if value != 0 { 1 } else { 0 },
        },
    }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_make_heap_ptr(
    data_ptr: *mut c_void,
    exact: u8,
) -> EshkolTaggedValue {
    EshkolTaggedValue {
        value_type: ESHKOL_VALUE_HEAP_PTR,
        flags: if exact != 0 { ESHKOL_VALUE_EXACT_FLAG } else { 0 },
        reserved: 0,
        data: EshkolTaggedData {
            ptr_val: data_ptr as u64,
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_tv_make_rational(
    arena: *mut Arena,
    numerator: i64,
    denominator: i64,
) -> EshkolTaggedValue {
    // SAFETY: delegated to interop allocator.
    let rat = unsafe { eshkol_rust_rational_create(arena, numerator, denominator) };
    if rat.is_null() {
        return eshkol_rust_tv_make_null();
    }
    eshkol_rust_tv_make_heap_ptr(rat.cast::<c_void>(), 1)
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_tv_make_string(
    arena: *mut Arena,
    c_str: *const c_char,
) -> EshkolTaggedValue {
    // SAFETY: delegated to interop allocator.
    let s = unsafe { eshkol_rust_string_create(arena, c_str) };
    if s.is_null() {
        return eshkol_rust_tv_make_null();
    }
    eshkol_rust_tv_make_heap_ptr(s.cast::<c_void>(), 0)
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_tv_make_symbol(
    arena: *mut Arena,
    c_str: *const c_char,
) -> EshkolTaggedValue {
    // SAFETY: delegated to interop allocator.
    let s = unsafe { eshkol_rust_symbol_create(arena, c_str) };
    if s.is_null() {
        return eshkol_rust_tv_make_null();
    }
    eshkol_rust_tv_make_heap_ptr(s.cast::<c_void>(), 1)
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_ptr(tv: EshkolTaggedValue) -> *mut c_void {
    if tv.value_type != ESHKOL_VALUE_HEAP_PTR {
        return ptr::null_mut();
    }

    // SAFETY: tag check above guarantees pointer payload.
    let ptr_val = unsafe { tv.data.ptr_val };
    ptr_val as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn eshkol_rust_tv_heap_subtype(tv: EshkolTaggedValue) -> u8 {
    let ptr = eshkol_rust_tv_ptr(tv);
    if ptr.is_null() {
        return 0;
    }

    // SAFETY: pointer comes from heap-pointer tagged value.
    unsafe { eshkol_rust_get_subtype_from_data_ptr(ptr.cast::<c_void>()) }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_type(tv: EshkolTaggedValue) -> u8 {
    tv.value_type
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_flags(tv: EshkolTaggedValue) -> u8 {
    tv.flags
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_int64(tv: EshkolTaggedValue) -> i64 {
    if tv.value_type != ESHKOL_VALUE_INT64 {
        return 0;
    }

    // SAFETY: tag check above guarantees `int_val` payload.
    unsafe { tv.data.int_val }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_double(tv: EshkolTaggedValue) -> f64 {
    if tv.value_type != ESHKOL_VALUE_DOUBLE {
        return 0.0;
    }

    // SAFETY: tag check above guarantees `double_val` payload.
    unsafe { tv.data.double_val }
}

#[no_mangle]
pub extern "C" fn eshkol_rust_tv_bool(tv: EshkolTaggedValue) -> u8 {
    if tv.value_type != ESHKOL_VALUE_BOOL {
        return 0;
    }

    // SAFETY: tag check above guarantees bool payload stored in int_val.
    let value = unsafe { tv.data.int_val };
    if value != 0 { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn object_header_layout_matches_c_contract() {
        assert_eq!(size_of::<EshkolObjectHeader>(), 8);
        assert_eq!(align_of::<EshkolObjectHeader>(), 4);
    }

    #[test]
    fn arena_allocates_and_tracks_usage() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = arena_create(64 * 1024);
            assert!(!arena.is_null());
            assert_eq!(arena_get_default_block_size(arena), 64 * 1024);

            let p1 = arena_allocate(arena, 16);
            assert!(!p1.is_null());

            let p2 = arena_allocate_with_header(arena, 24, HEAP_SUBTYPE_RATIONAL, 0x10);
            assert!(!p2.is_null());

            let used = arena_get_used_memory(arena);
            assert!(used >= 16 + size_of::<EshkolObjectHeader>() + 24);
            assert_eq!(arena_get_block_count(arena), 2);

            arena_destroy(arena);
        }
    }

    #[test]
    fn header_roundtrip_helpers_work() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = arena_create(4096);
            let data = arena_allocate_with_header(arena, 32, HEAP_SUBTYPE_RATIONAL, 0x02);
            assert!(!data.is_null());

            let header = eshkol_get_header_from_data_ptr(data);
            assert!(!header.is_null());
            assert_eq!((*header).subtype, HEAP_SUBTYPE_RATIONAL);
            assert_eq!((*header).flags, 0x02);
            assert_eq!((*header).size, 32);

            assert_eq!(eshkol_get_subtype_from_data_ptr(data), HEAP_SUBTYPE_RATIONAL);
            assert_eq!(eshkol_get_flags_from_data_ptr(data), 0x02);

            arena_destroy(arena);
        }
    }

    #[test]
    fn string_allocation_sets_string_subtype() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = arena_create(4096);
            let s = arena_allocate_string_with_header(arena, 12);
            assert!(!s.is_null());

            let subtype = eshkol_get_subtype_from_data_ptr(s.cast::<c_void>());
            assert_eq!(subtype, HEAP_SUBTYPE_STRING);

            arena_destroy(arena);
        }
    }

    #[test]
    fn rational_pilot_allocation_normalizes_and_sets_header() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = eshkol_rust_arena_create(4096);
            let rat = eshkol_rust_rational_create(arena, 4, -6);
            assert!(!rat.is_null());
            assert_eq!((*rat).numerator, -2);
            assert_eq!((*rat).denominator, 3);

            assert_eq!(eshkol_rust_get_subtype_from_data_ptr(rat.cast::<c_void>()), HEAP_SUBTYPE_RATIONAL);

            eshkol_rust_arena_destroy(arena);
        }
    }

    #[test]
    fn rational_normalize_helper_export_works() {
        let mut n = 0_i64;
        let mut d = 0_i64;
        eshkol_rust_rational_normalize_into(12, -18, &mut n, &mut d);
        assert_eq!((n, d), (-2, 3));

        eshkol_rust_rational_normalize_into(5, 0, &mut n, &mut d);
        assert_eq!((n, d), (0, 1));
    }

    #[test]
    fn second_allocation_path_string_and_symbol_work() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = eshkol_rust_arena_create(4096);

            let s_input = CString::new("hello-rust").unwrap();
            let s_ptr = eshkol_rust_string_create(arena, s_input.as_ptr());
            assert!(!s_ptr.is_null());
            assert_eq!(eshkol_rust_get_subtype_from_data_ptr(s_ptr.cast::<c_void>()), HEAP_SUBTYPE_STRING);
            assert_eq!(CStr::from_ptr(s_ptr).to_str().unwrap(), "hello-rust");

            let sym_input = CString::new("my-symbol").unwrap();
            let sym_ptr = eshkol_rust_symbol_create(arena, sym_input.as_ptr());
            assert!(!sym_ptr.is_null());
            assert_eq!(eshkol_rust_get_subtype_from_data_ptr(sym_ptr.cast::<c_void>()), HEAP_SUBTYPE_SYMBOL);
            assert_eq!(CStr::from_ptr(sym_ptr).to_str().unwrap(), "my-symbol");

            eshkol_rust_arena_destroy(arena);
        }
    }

    #[test]
    fn tagged_value_heap_pointer_helpers_cover_rational_and_string() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = eshkol_rust_arena_create(4096);

            let tv_rat = eshkol_rust_tv_make_rational(arena, 9, 6);
            assert_eq!(eshkol_rust_tv_type(tv_rat), ESHKOL_VALUE_HEAP_PTR);
            assert_eq!(eshkol_rust_tv_heap_subtype(tv_rat), HEAP_SUBTYPE_RATIONAL);
            let rat_ptr = eshkol_rust_tv_ptr(tv_rat).cast::<EshkolRational>();
            assert!(!rat_ptr.is_null());
            assert_eq!((*rat_ptr).numerator, 3);
            assert_eq!((*rat_ptr).denominator, 2);

            let s_input = CString::new("interop-str").unwrap();
            let tv_str = eshkol_rust_tv_make_string(arena, s_input.as_ptr());
            assert_eq!(eshkol_rust_tv_type(tv_str), ESHKOL_VALUE_HEAP_PTR);
            assert_eq!(eshkol_rust_tv_heap_subtype(tv_str), HEAP_SUBTYPE_STRING);
            let str_ptr = eshkol_rust_tv_ptr(tv_str).cast::<c_char>();
            assert_eq!(CStr::from_ptr(str_ptr).to_str().unwrap(), "interop-str");

            let sym_input = CString::new("interop-symbol").unwrap();
            let tv_sym = eshkol_rust_tv_make_symbol(arena, sym_input.as_ptr());
            assert_eq!(eshkol_rust_tv_heap_subtype(tv_sym), HEAP_SUBTYPE_SYMBOL);

            eshkol_rust_arena_destroy(arena);
        }
    }

    #[test]
    fn header_metadata_mutation_helpers_are_guarded_and_work() {
        // SAFETY: FFI-style API used within test scope with valid pointers.
        unsafe {
            let arena = eshkol_rust_arena_create(4096);

            let rat = eshkol_rust_rational_create(arena, 8, 12);
            assert!(!rat.is_null());
            let rat_ptr = rat.cast::<c_void>();
            assert_eq!(eshkol_rust_header_validate(rat_ptr), 1);

            let text = CString::new("meta-text").unwrap();
            let str_ptr = eshkol_rust_string_create(arena, text.as_ptr()).cast::<c_void>();
            assert!(!str_ptr.is_null());
            assert_eq!(eshkol_rust_header_validate(str_ptr), 1);

            // Refcount mutation policy: allowed for strings, blocked for rationals.
            assert_eq!(eshkol_rust_header_ref_count(rat_ptr), 0);
            assert_eq!(eshkol_rust_header_inc_ref_count(rat_ptr), 0);
            assert_eq!(eshkol_rust_header_set_ref_count(rat_ptr, 9), 0);

            assert_eq!(eshkol_rust_header_ref_count(str_ptr), 0);
            assert_eq!(eshkol_rust_header_inc_ref_count(str_ptr), 1);
            assert_eq!(eshkol_rust_header_inc_ref_count(str_ptr), 2);
            assert_eq!(eshkol_rust_header_dec_ref_count(str_ptr), 1);
            assert_eq!(eshkol_rust_header_set_ref_count(str_ptr, 7), 7);

            // Flag mutation policy: rational disallows external/shared bits.
            assert_eq!(eshkol_rust_header_set_flags(rat_ptr, ESHKOL_OBJ_FLAG_EXTERNAL), 0);
            assert_eq!(eshkol_rust_header_set_flags(rat_ptr, ESHKOL_OBJ_FLAG_MARKED), ESHKOL_OBJ_FLAG_MARKED);
            assert_eq!(
                eshkol_rust_header_add_flags(rat_ptr, ESHKOL_OBJ_FLAG_BORROWED | ESHKOL_OBJ_FLAG_EXTERNAL),
                ESHKOL_OBJ_FLAG_MARKED | ESHKOL_OBJ_FLAG_BORROWED
            );
            assert_eq!(
                eshkol_rust_header_clear_flags(rat_ptr, ESHKOL_OBJ_FLAG_MARKED),
                ESHKOL_OBJ_FLAG_BORROWED
            );

            // String permits external/shared lifecycle flags.
            assert_eq!(eshkol_rust_header_set_flags(str_ptr, ESHKOL_OBJ_FLAG_EXTERNAL), ESHKOL_OBJ_FLAG_EXTERNAL);
            assert_eq!(
                eshkol_rust_header_add_flags(str_ptr, ESHKOL_OBJ_FLAG_SHARED | ESHKOL_OBJ_FLAG_WEAK),
                ESHKOL_OBJ_FLAG_EXTERNAL | ESHKOL_OBJ_FLAG_SHARED | ESHKOL_OBJ_FLAG_WEAK
            );
            assert_eq!(
                eshkol_rust_header_clear_flags(str_ptr, ESHKOL_OBJ_FLAG_EXTERNAL),
                ESHKOL_OBJ_FLAG_SHARED | ESHKOL_OBJ_FLAG_WEAK
            );

            eshkol_rust_arena_destroy(arena);
        }
    }

    #[test]
    fn header_metadata_helpers_reject_invalid_pointers() {
        // SAFETY: passing null pointers is explicitly handled by helper guards.
        unsafe {
            assert_eq!(eshkol_rust_header_validate(ptr::null()), 0);
            assert_eq!(eshkol_rust_header_ref_count(ptr::null()), 0);
            assert_eq!(eshkol_rust_header_set_ref_count(ptr::null(), 3), 0);
            assert_eq!(eshkol_rust_header_inc_ref_count(ptr::null()), 0);
            assert_eq!(eshkol_rust_header_dec_ref_count(ptr::null()), 0);
            assert_eq!(eshkol_rust_header_set_flags(ptr::null(), 0xFF), 0);
            assert_eq!(eshkol_rust_header_add_flags(ptr::null(), 0xFF), 0);
            assert_eq!(eshkol_rust_header_clear_flags(ptr::null(), 0xFF), 0);
        }
    }

    #[test]
    fn tagged_value_helpers_roundtrip_basic_values() {
        let i = eshkol_rust_tv_make_int64(42, 1);
        assert_eq!(eshkol_rust_tv_type(i), ESHKOL_VALUE_INT64);
        assert_eq!(eshkol_rust_tv_flags(i), ESHKOL_VALUE_EXACT_FLAG);
        assert_eq!(eshkol_rust_tv_int64(i), 42);

        let d = eshkol_rust_tv_make_double(3.5);
        assert_eq!(eshkol_rust_tv_type(d), ESHKOL_VALUE_DOUBLE);
        assert!((eshkol_rust_tv_double(d) - 3.5).abs() < 1e-12);

        let b = eshkol_rust_tv_make_bool(1);
        assert_eq!(eshkol_rust_tv_type(b), ESHKOL_VALUE_BOOL);
        assert_eq!(eshkol_rust_tv_bool(b), 1);

        let n = eshkol_rust_tv_make_null();
        assert_eq!(eshkol_rust_tv_type(n), ESHKOL_VALUE_NULL);
    }
}
