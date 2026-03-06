// AArch64 helper functions for AOT compilation
// These are compiled to LLVM bitcode and linked into AOT modules
// to enable inlining and cross-function optimization.

#include <stdint.h>

// Division helpers
uint64_t helper_udiv64(uint64_t n, uint64_t m) {
    return m == 0 ? 0 : n / m;
}

uint64_t helper_udiv32(uint64_t n, uint64_t m) {
    uint32_t n32 = (uint32_t)n;
    uint32_t m32 = (uint32_t)m;
    return m32 == 0 ? 0 : (uint64_t)(n32 / m32);
}

uint64_t helper_sdiv64(uint64_t n, uint64_t m) {
    int64_t ns = (int64_t)n;
    int64_t ms = (int64_t)m;
    return ms == 0 ? 0 : (uint64_t)(ns / ms);
}

uint64_t helper_sdiv32(uint64_t n, uint64_t m) {
    int32_t n32 = (int32_t)n;
    int32_t m32 = (int32_t)m;
    return m32 == 0 ? 0 : (uint64_t)(n32 / m32);
}

// ADC/SBC helpers
uint64_t helper_adc64(uint64_t n, uint64_t m, uint64_t c) {
    return n + m + (c & 1);
}

uint64_t helper_adc32(uint64_t n, uint64_t m, uint64_t c) {
    uint32_t result = (uint32_t)n + (uint32_t)m + (uint32_t)(c & 1);
    return (uint64_t)result;
}

uint64_t helper_sbc64(uint64_t n, uint64_t m, uint64_t c) {
    return n - m - (1 - (c & 1));
}

uint64_t helper_sbc32(uint64_t n, uint64_t m, uint64_t c) {
    uint32_t result = (uint32_t)n - (uint32_t)m - (uint32_t)(1 - (c & 1));
    return (uint64_t)result;
}

// Bit manipulation helpers
uint64_t helper_rbit64(uint64_t a) {
    uint64_t result = 0;
    for (int i = 0; i < 64; i++) {
        result = (result << 1) | (a & 1);
        a >>= 1;
    }
    return result;
}

uint64_t helper_rbit32(uint64_t a) {
    uint32_t a32 = (uint32_t)a;
    uint32_t result = 0;
    for (int i = 0; i < 32; i++) {
        result = (result << 1) | (a32 & 1);
        a32 >>= 1;
    }
    return (uint64_t)result;
}

uint64_t helper_rev16_64(uint64_t a) {
    uint64_t result = 0;
    for (int i = 0; i < 4; i++) {
        uint16_t chunk = (a >> (i * 16)) & 0xFFFF;
        uint16_t reversed = ((chunk & 0xFF) << 8) | ((chunk >> 8) & 0xFF);
        result |= ((uint64_t)reversed) << (i * 16);
    }
    return result;
}

uint64_t helper_rev16_32(uint64_t a) {
    uint32_t a32 = (uint32_t)a;
    uint32_t result = 0;
    for (int i = 0; i < 2; i++) {
        uint16_t chunk = (a32 >> (i * 16)) & 0xFFFF;
        uint16_t reversed = ((chunk & 0xFF) << 8) | ((chunk >> 8) & 0xFF);
        result |= ((uint32_t)reversed) << (i * 16);
    }
    return (uint64_t)result;
}

uint64_t helper_rev32_64(uint64_t a) {
    uint64_t lo = (uint32_t)a;
    uint64_t hi = a >> 32;
    lo = ((lo & 0xFF000000) >> 24) | ((lo & 0x00FF0000) >> 8) |
         ((lo & 0x0000FF00) << 8) | ((lo & 0x000000FF) << 24);
    hi = ((hi & 0xFF000000) >> 24) | ((hi & 0x00FF0000) >> 8) |
         ((hi & 0x0000FF00) << 8) | ((hi & 0x000000FF) << 24);
    return (hi << 32) | lo;
}

// Lazy NZCV condition evaluation helper for AOT.
// Keep this helper in embedded LLVM bitcode to avoid requiring
// exported symbols from the runtime binary.
#define CC_OP_EAGER   0
#define CC_OP_ADD32   1
#define CC_OP_ADD64   2
#define CC_OP_SUB32   3
#define CC_OP_SUB64   4
#define CC_OP_LOGIC32 5
#define CC_OP_LOGIC64 6

static uint64_t compute_nzcv_add_local(
    uint64_t a, uint64_t b, uint64_t result, int sf
) {
    if (sf) {
        uint64_t n = (result >> 63) & 1;
        uint64_t z = result == 0 ? 1 : 0;
        uint64_t c = result < a ? 1 : 0;
        uint64_t xor_ab = a ^ b;
        uint64_t xor_ar = a ^ result;
        uint64_t v = ((~xor_ab) & xor_ar) >> 63;
        return (n << 31) | (z << 30) | (c << 29) | (v << 28);
    } else {
        uint32_t a32 = (uint32_t)a;
        uint32_t b32 = (uint32_t)b;
        uint32_t r32 = (uint32_t)result;
        uint64_t n = (r32 >> 31) & 1;
        uint64_t z = r32 == 0 ? 1 : 0;
        uint64_t c = r32 < a32 ? 1 : 0;
        uint32_t xor_ab = a32 ^ b32;
        uint32_t xor_ar = a32 ^ r32;
        uint64_t v = ((~xor_ab) & xor_ar) >> 31;
        return (n << 31) | (z << 30) | (c << 29) | (v << 28);
    }
}

static uint64_t compute_nzcv_sub_local(
    uint64_t a, uint64_t b, uint64_t result, int sf
) {
    if (sf) {
        uint64_t n = (result >> 63) & 1;
        uint64_t z = result == 0 ? 1 : 0;
        uint64_t c = a >= b ? 1 : 0;
        uint64_t xor_ab = a ^ b;
        uint64_t xor_ar = a ^ result;
        uint64_t v = (xor_ab & xor_ar) >> 63;
        return (n << 31) | (z << 30) | (c << 29) | (v << 28);
    } else {
        uint32_t a32 = (uint32_t)a;
        uint32_t b32 = (uint32_t)b;
        uint32_t r32 = (uint32_t)result;
        uint64_t n = (r32 >> 31) & 1;
        uint64_t z = r32 == 0 ? 1 : 0;
        uint64_t c = a32 >= b32 ? 1 : 0;
        uint32_t xor_ab = a32 ^ b32;
        uint32_t xor_ar = a32 ^ r32;
        uint64_t v = (xor_ab & xor_ar) >> 31;
        return (n << 31) | (z << 30) | (c << 29) | (v << 28);
    }
}

static uint64_t compute_nzcv_logic_local(
    uint64_t result, int sf
) {
    if (sf) {
        uint64_t n = (result >> 63) & 1;
        uint64_t z = result == 0 ? 1 : 0;
        return (n << 31) | (z << 30);
    } else {
        uint32_t r32 = (uint32_t)result;
        uint64_t n = (r32 >> 31) & 1;
        uint64_t z = r32 == 0 ? 1 : 0;
        return (n << 31) | (z << 30);
    }
}

static uint64_t lazy_nzcv_to_packed_local(
    uint64_t cc_op, uint64_t cc_a, uint64_t cc_b, uint64_t cc_result
) {
    switch (cc_op) {
    case CC_OP_EAGER:
        return cc_a;
    case CC_OP_ADD32:
        return compute_nzcv_add_local(cc_a, cc_b, cc_result, 0);
    case CC_OP_ADD64:
        return compute_nzcv_add_local(cc_a, cc_b, cc_result, 1);
    case CC_OP_SUB32:
        return compute_nzcv_sub_local(cc_a, cc_b, cc_result, 0);
    case CC_OP_SUB64:
        return compute_nzcv_sub_local(cc_a, cc_b, cc_result, 1);
    case CC_OP_LOGIC32:
        return compute_nzcv_logic_local(cc_result, 0);
    case CC_OP_LOGIC64:
        return compute_nzcv_logic_local(cc_result, 1);
    default:
        return 0;
    }
}

static uint64_t eval_cond_from_packed_local(
    uint64_t nzcv, uint32_t cond
) {
    uint64_t n = (nzcv >> 31) & 1;
    uint64_t z = (nzcv >> 30) & 1;
    uint64_t c = (nzcv >> 29) & 1;
    uint64_t v = (nzcv >> 28) & 1;
    uint64_t result = 0;

    switch (cond >> 1) {
    case 0: result = z; break;
    case 1: result = c; break;
    case 2: result = n; break;
    case 3: result = v; break;
    case 4: result = c & !z; break;
    case 5: result = (n == v); break;
    case 6: result = (n == v) & !z; break;
    case 7: result = 1; break;
    default: result = 0; break;
    }

    if ((cond & 1) != 0 && cond != 15) {
        result ^= 1;
    }
    return result;
}

__attribute__((visibility("hidden")))
uint64_t helper_lazy_nzcv_eval_cond(
    uint64_t cc_op, uint64_t cc_a, uint64_t cc_b, uint64_t cc_result,
    uint64_t cond
) {
    uint64_t nzcv = (cc_op == CC_OP_EAGER)
        ? cc_a
        : lazy_nzcv_to_packed_local(cc_op, cc_a, cc_b, cc_result);
    return eval_cond_from_packed_local(nzcv, (uint32_t)cond);
}
