#include <cassert>
#include <cstdint>
#include <cstring>
#include <iostream>

constexpr uint8_t ESHKOL_VALUE_HEAP_PTR = 8;
constexpr uint8_t HEAP_SUBTYPE_RATIONAL = 19;

typedef union {
    int64_t int_val;
    double double_val;
    uint64_t ptr_val;
    uint64_t raw_val;
} EshkolTaggedData;

typedef struct {
    uint8_t value_type;
    uint8_t flags;
    uint16_t reserved;
    EshkolTaggedData data;
} EshkolTaggedValue;

typedef struct {
    int64_t numerator;
    int64_t denominator;
} EshkolRational;

extern "C" {
void* eshkol_rust_arena_create(size_t default_block_size);
void eshkol_rust_arena_destroy(void* arena);
size_t eshkol_rust_arena_get_used_memory(const void* arena);

EshkolRational* eshkol_rust_rational_create(void* arena, int64_t numerator, int64_t denominator);
EshkolTaggedValue eshkol_rust_tv_make_rational(void* arena, int64_t numerator, int64_t denominator);
uint8_t eshkol_rust_tv_type(EshkolTaggedValue tv);
uint8_t eshkol_rust_tv_heap_subtype(EshkolTaggedValue tv);
void* eshkol_rust_tv_ptr(EshkolTaggedValue tv);

int mixed_c_part_check(void* arena);
}

int main() {
    void* arena = eshkol_rust_arena_create(4096);
    assert(arena != nullptr);

    const int c_status = mixed_c_part_check(arena);
    assert(c_status == 0);

    EshkolRational* rat = eshkol_rust_rational_create(arena, 18, 12);
    assert(rat != nullptr);
    assert(rat->numerator == 3 && rat->denominator == 2);

    EshkolTaggedValue tv_rat = eshkol_rust_tv_make_rational(arena, 30, 9);
    assert(eshkol_rust_tv_type(tv_rat) == ESHKOL_VALUE_HEAP_PTR);
    assert(eshkol_rust_tv_heap_subtype(tv_rat) == HEAP_SUBTYPE_RATIONAL);

    auto* rat_from_tv = static_cast<EshkolRational*>(eshkol_rust_tv_ptr(tv_rat));
    assert(rat_from_tv != nullptr);
    assert(rat_from_tv->numerator == 10 && rat_from_tv->denominator == 3);

    assert(eshkol_rust_arena_get_used_memory(arena) > 0);

    eshkol_rust_arena_destroy(arena);
    std::cout << "rust interop smoke mixed PASS\n";
    return 0;
}
