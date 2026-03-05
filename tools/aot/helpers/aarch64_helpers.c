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
