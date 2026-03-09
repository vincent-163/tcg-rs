    .text
    .align 12

    .global __kernel_clock_gettime
    .type __kernel_clock_gettime, %function
__kernel_clock_gettime:
    mov x8, #113
    svc #0
    ret
    .size __kernel_clock_gettime, .-__kernel_clock_gettime

    .global __kernel_gettimeofday
    .type __kernel_gettimeofday, %function
__kernel_gettimeofday:
    mov x8, #169
    svc #0
    ret
    .size __kernel_gettimeofday, .-__kernel_gettimeofday

    .global __kernel_clock_getres
    .type __kernel_clock_getres, %function
__kernel_clock_getres:
    mov x8, #114
    svc #0
    ret
    .size __kernel_clock_getres, .-__kernel_clock_getres
