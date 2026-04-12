#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <iostream>

constexpr uint8_t ESHKOL_VALUE_NULL = 0;
constexpr uint8_t ESHKOL_VALUE_INT64 = 1;
constexpr uint8_t ESHKOL_VALUE_DOUBLE = 2;
constexpr uint8_t ESHKOL_VALUE_BOOL = 3;
constexpr uint8_t ESHKOL_VALUE_HEAP_PTR = 8;

constexpr uint8_t HEAP_SUBTYPE_STRING = 1;
constexpr uint8_t HEAP_SUBTYPE_SYMBOL = 10;
constexpr uint8_t HEAP_SUBTYPE_RATIONAL = 19;

union EshkolTaggedData {
    int64_t int_val;
    double double_val;
    uint64_t ptr_val;
    uint64_t raw_val;
};

struct EshkolTaggedValue {
    uint8_t value_type;
    uint8_t flags;
    uint16_t reserved;
    EshkolTaggedData data;
};

struct EshkolRational {
    int64_t numerator;
    int64_t denominator;
};

extern "C" {
void* eshkol_rust_arena_create(size_t default_block_size);
void eshkol_rust_arena_destroy(void* arena);
void* eshkol_rust_arena_allocate_string_with_header(void* arena, size_t length);
size_t eshkol_rust_arena_get_used_memory(const void* arena);
uint8_t eshkol_rust_get_subtype_from_data_ptr(const void* data_ptr);

EshkolRational* eshkol_rust_rational_create(void* arena, int64_t numerator, int64_t denominator);
char* eshkol_rust_string_create(void* arena, const char* c_str);
char* eshkol_rust_symbol_create(void* arena, const char* c_str);

EshkolTaggedValue eshkol_rust_tv_make_null(void);
EshkolTaggedValue eshkol_rust_tv_make_int64(int64_t value, uint8_t exact);
EshkolTaggedValue eshkol_rust_tv_make_double(double value);
EshkolTaggedValue eshkol_rust_tv_make_bool(uint8_t value);
EshkolTaggedValue eshkol_rust_tv_make_heap_ptr(void* data_ptr, uint8_t exact);
EshkolTaggedValue eshkol_rust_tv_make_rational(void* arena, int64_t numerator, int64_t denominator);
EshkolTaggedValue eshkol_rust_tv_make_string(void* arena, const char* c_str);
EshkolTaggedValue eshkol_rust_tv_make_symbol(void* arena, const char* c_str);

uint8_t eshkol_rust_tv_type(EshkolTaggedValue tv);
int64_t eshkol_rust_tv_int64(EshkolTaggedValue tv);
double eshkol_rust_tv_double(EshkolTaggedValue tv);
uint8_t eshkol_rust_tv_bool(EshkolTaggedValue tv);
void* eshkol_rust_tv_ptr(EshkolTaggedValue tv);
uint8_t eshkol_rust_tv_heap_subtype(EshkolTaggedValue tv);
}

int main() {
    void* arena = eshkol_rust_arena_create(4096);
    assert(arena != nullptr);

    auto* raw = static_cast<char*>(eshkol_rust_arena_allocate_string_with_header(arena, 24));
    assert(raw != nullptr);
    assert(eshkol_rust_get_subtype_from_data_ptr(raw) == HEAP_SUBTYPE_STRING);

    auto* rat = eshkol_rust_rational_create(arena, 12, 30);
    assert(rat != nullptr);
    assert(rat->numerator == 2 && rat->denominator == 5);

    auto* str = eshkol_rust_string_create(arena, "hello-cpp");
    assert(str != nullptr);
    assert(std::strcmp(str, "hello-cpp") == 0);

    auto* sym = eshkol_rust_symbol_create(arena, "cpp-symbol");
    assert(sym != nullptr);
    assert(eshkol_rust_get_subtype_from_data_ptr(sym) == HEAP_SUBTYPE_SYMBOL);

    EshkolTaggedValue tv_i = eshkol_rust_tv_make_int64(99, 1);
    assert(eshkol_rust_tv_type(tv_i) == ESHKOL_VALUE_INT64);
    assert(eshkol_rust_tv_int64(tv_i) == 99);

    EshkolTaggedValue tv_d = eshkol_rust_tv_make_double(9.25);
    assert(eshkol_rust_tv_type(tv_d) == ESHKOL_VALUE_DOUBLE);
    assert(std::fabs(eshkol_rust_tv_double(tv_d) - 9.25) < 1e-12);

    EshkolTaggedValue tv_b = eshkol_rust_tv_make_bool(1);
    assert(eshkol_rust_tv_type(tv_b) == ESHKOL_VALUE_BOOL);
    assert(eshkol_rust_tv_bool(tv_b) == 1);

    EshkolTaggedValue tv_n = eshkol_rust_tv_make_null();
    assert(eshkol_rust_tv_type(tv_n) == ESHKOL_VALUE_NULL);

    EshkolTaggedValue tv_heap = eshkol_rust_tv_make_heap_ptr(rat, 1);
    assert(eshkol_rust_tv_type(tv_heap) == ESHKOL_VALUE_HEAP_PTR);
    assert(eshkol_rust_tv_heap_subtype(tv_heap) == HEAP_SUBTYPE_RATIONAL);

    EshkolTaggedValue tv_rat = eshkol_rust_tv_make_rational(arena, 21, 6);
    assert(eshkol_rust_tv_type(tv_rat) == ESHKOL_VALUE_HEAP_PTR);
    assert(eshkol_rust_tv_heap_subtype(tv_rat) == HEAP_SUBTYPE_RATIONAL);

    EshkolTaggedValue tv_str = eshkol_rust_tv_make_string(arena, "cpp-str-tv");
    assert(eshkol_rust_tv_heap_subtype(tv_str) == HEAP_SUBTYPE_STRING);
    auto* str_ptr = static_cast<char*>(eshkol_rust_tv_ptr(tv_str));
    assert(str_ptr != nullptr && std::strcmp(str_ptr, "cpp-str-tv") == 0);

    EshkolTaggedValue tv_sym = eshkol_rust_tv_make_symbol(arena, "cpp-sym-tv");
    assert(eshkol_rust_tv_heap_subtype(tv_sym) == HEAP_SUBTYPE_SYMBOL);

    assert(eshkol_rust_arena_get_used_memory(arena) > 0);

    eshkol_rust_arena_destroy(arena);
    std::cout << "rust interop smoke C++ PASS\n";
    return 0;
}
