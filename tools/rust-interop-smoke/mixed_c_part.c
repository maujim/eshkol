#include <stdint.h>
#include <string.h>

#define ESHKOL_VALUE_HEAP_PTR 8
#define HEAP_SUBTYPE_SYMBOL   10

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

extern char* eshkol_rust_symbol_create(void* arena, const char* c_str);
extern uint8_t eshkol_rust_get_subtype_from_data_ptr(const void* data_ptr);
extern EshkolTaggedValue eshkol_rust_tv_make_symbol(void* arena, const char* c_str);
extern uint8_t eshkol_rust_tv_type(EshkolTaggedValue tv);
extern uint8_t eshkol_rust_tv_heap_subtype(EshkolTaggedValue tv);
extern void* eshkol_rust_tv_ptr(EshkolTaggedValue tv);

int mixed_c_part_check(void* arena) {
    char* sym = eshkol_rust_symbol_create(arena, "mixed-c-symbol");
    if (!sym) return 1;
    if (strcmp(sym, "mixed-c-symbol") != 0) return 2;
    if (eshkol_rust_get_subtype_from_data_ptr(sym) != HEAP_SUBTYPE_SYMBOL) return 3;

    EshkolTaggedValue tv_sym = eshkol_rust_tv_make_symbol(arena, "mixed-c-tv-symbol");
    if (eshkol_rust_tv_type(tv_sym) != ESHKOL_VALUE_HEAP_PTR) return 4;
    if (eshkol_rust_tv_heap_subtype(tv_sym) != HEAP_SUBTYPE_SYMBOL) return 5;

    char* ptr = (char*)eshkol_rust_tv_ptr(tv_sym);
    if (!ptr) return 6;
    if (strcmp(ptr, "mixed-c-tv-symbol") != 0) return 7;

    return 0;
}
