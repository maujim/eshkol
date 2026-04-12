#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ESHKOL_VALUE_NULL     0
#define ESHKOL_VALUE_INT64    1
#define ESHKOL_VALUE_DOUBLE   2
#define ESHKOL_VALUE_BOOL     3
#define ESHKOL_VALUE_HEAP_PTR 8

#define HEAP_SUBTYPE_STRING   1
#define HEAP_SUBTYPE_SYMBOL   10
#define HEAP_SUBTYPE_RATIONAL 19

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

extern void* eshkol_rust_arena_create(size_t default_block_size);
extern void eshkol_rust_arena_destroy(void* arena);
extern void* eshkol_rust_arena_allocate_string_with_header(void* arena, size_t length);
extern size_t eshkol_rust_arena_get_used_memory(const void* arena);
extern uint8_t eshkol_rust_get_subtype_from_data_ptr(const void* data_ptr);
extern uint8_t eshkol_rust_get_flags_from_data_ptr(const void* data_ptr);

extern EshkolRational* eshkol_rust_rational_create(void* arena, int64_t numerator, int64_t denominator);
extern char* eshkol_rust_string_create(void* arena, const char* c_str);
extern char* eshkol_rust_symbol_create(void* arena, const char* c_str);

extern EshkolTaggedValue eshkol_rust_tv_make_null(void);
extern EshkolTaggedValue eshkol_rust_tv_make_int64(int64_t value, uint8_t exact);
extern EshkolTaggedValue eshkol_rust_tv_make_double(double value);
extern EshkolTaggedValue eshkol_rust_tv_make_bool(uint8_t value);
extern EshkolTaggedValue eshkol_rust_tv_make_heap_ptr(void* data_ptr, uint8_t exact);
extern EshkolTaggedValue eshkol_rust_tv_make_rational(void* arena, int64_t numerator, int64_t denominator);
extern EshkolTaggedValue eshkol_rust_tv_make_string(void* arena, const char* c_str);
extern EshkolTaggedValue eshkol_rust_tv_make_symbol(void* arena, const char* c_str);

extern uint8_t eshkol_rust_tv_type(EshkolTaggedValue tv);
extern uint8_t eshkol_rust_tv_flags(EshkolTaggedValue tv);
extern int64_t eshkol_rust_tv_int64(EshkolTaggedValue tv);
extern double eshkol_rust_tv_double(EshkolTaggedValue tv);
extern uint8_t eshkol_rust_tv_bool(EshkolTaggedValue tv);
extern void* eshkol_rust_tv_ptr(EshkolTaggedValue tv);
extern uint8_t eshkol_rust_tv_heap_subtype(EshkolTaggedValue tv);

#define CHECK(cond, msg) \
    do { \
        if (!(cond)) { \
            fprintf(stderr, "[FAIL] %s\n", (msg)); \
            return 1; \
        } \
    } while (0)

int main(void) {
    void* arena = eshkol_rust_arena_create(4096);
    CHECK(arena != NULL, "arena creation failed");

    char* s = (char*)eshkol_rust_arena_allocate_string_with_header(arena, 32);
    CHECK(s != NULL, "string allocation failed");
    CHECK(eshkol_rust_get_subtype_from_data_ptr((const void*)s) == HEAP_SUBTYPE_STRING,
          "string subtype mismatch");

    EshkolRational* rat = eshkol_rust_rational_create(arena, 4, 6);
    CHECK(rat != NULL, "rational allocation failed");
    CHECK(rat->numerator == 2 && rat->denominator == 3, "rational normalization mismatch");
    CHECK(eshkol_rust_get_subtype_from_data_ptr((const void*)rat) == HEAP_SUBTYPE_RATIONAL,
          "rational subtype mismatch");
    CHECK(eshkol_rust_get_flags_from_data_ptr((const void*)rat) == 0,
          "rational flags mismatch");

    char* s2 = eshkol_rust_string_create(arena, "hello-c-interop");
    CHECK(s2 != NULL, "string create helper failed");
    CHECK(strcmp(s2, "hello-c-interop") == 0, "string helper payload mismatch");

    char* sym = eshkol_rust_symbol_create(arena, "sym-c");
    CHECK(sym != NULL, "symbol create helper failed");
    CHECK(eshkol_rust_get_subtype_from_data_ptr((const void*)sym) == HEAP_SUBTYPE_SYMBOL,
          "symbol subtype mismatch");

    EshkolTaggedValue tv_i = eshkol_rust_tv_make_int64(42, 1);
    CHECK(eshkol_rust_tv_type(tv_i) == ESHKOL_VALUE_INT64, "int64 type mismatch");
    CHECK(eshkol_rust_tv_int64(tv_i) == 42, "int64 payload mismatch");
    CHECK((eshkol_rust_tv_flags(tv_i) & 0x10) != 0, "int64 exact flag missing");

    EshkolTaggedValue tv_d = eshkol_rust_tv_make_double(3.5);
    CHECK(eshkol_rust_tv_type(tv_d) == ESHKOL_VALUE_DOUBLE, "double type mismatch");
    CHECK(fabs(eshkol_rust_tv_double(tv_d) - 3.5) < 1e-12, "double payload mismatch");

    EshkolTaggedValue tv_b = eshkol_rust_tv_make_bool(1);
    CHECK(eshkol_rust_tv_type(tv_b) == ESHKOL_VALUE_BOOL, "bool type mismatch");
    CHECK(eshkol_rust_tv_bool(tv_b) == 1, "bool payload mismatch");

    EshkolTaggedValue tv_n = eshkol_rust_tv_make_null();
    CHECK(eshkol_rust_tv_type(tv_n) == ESHKOL_VALUE_NULL, "null type mismatch");

    EshkolTaggedValue tv_hp = eshkol_rust_tv_make_heap_ptr((void*)rat, 1);
    CHECK(eshkol_rust_tv_type(tv_hp) == ESHKOL_VALUE_HEAP_PTR, "heap-ptr type mismatch");
    CHECK(eshkol_rust_tv_heap_subtype(tv_hp) == HEAP_SUBTYPE_RATIONAL,
          "heap-ptr rational subtype mismatch");

    EshkolTaggedValue tv_rat = eshkol_rust_tv_make_rational(arena, 10, 4);
    CHECK(eshkol_rust_tv_type(tv_rat) == ESHKOL_VALUE_HEAP_PTR, "tv rational type mismatch");
    CHECK(eshkol_rust_tv_heap_subtype(tv_rat) == HEAP_SUBTYPE_RATIONAL,
          "tv rational subtype mismatch");

    EshkolTaggedValue tv_str = eshkol_rust_tv_make_string(arena, "str-tv");
    CHECK(eshkol_rust_tv_type(tv_str) == ESHKOL_VALUE_HEAP_PTR, "tv string type mismatch");
    CHECK(eshkol_rust_tv_heap_subtype(tv_str) == HEAP_SUBTYPE_STRING,
          "tv string subtype mismatch");

    EshkolTaggedValue tv_sym = eshkol_rust_tv_make_symbol(arena, "sym-tv");
    CHECK(eshkol_rust_tv_heap_subtype(tv_sym) == HEAP_SUBTYPE_SYMBOL,
          "tv symbol subtype mismatch");

    char* str_from_tv = (char*)eshkol_rust_tv_ptr(tv_str);
    CHECK(str_from_tv != NULL && strcmp(str_from_tv, "str-tv") == 0,
          "tv ptr extraction mismatch");

    size_t used = eshkol_rust_arena_get_used_memory(arena);
    CHECK(used > 0, "arena usage should be > 0");

    eshkol_rust_arena_destroy(arena);

    printf("rust interop smoke C PASS\n");
    return 0;
}
