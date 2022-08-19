.option norvc
.section .data
.section .text.init
.global _start
_start:
    csrr t0, mhartid
    bnez t0, 3f
.option push
.option norelax
    la gp, _global_pointer
.option pop
    la a0, _bss_start
    la a1, _bss_end
    bgeu a0, a1, 2f

1:
    sd  zero, (a0)
    addi a0, a0, 8
    bltu a0, a1, 1b

2:
    la sp, _stack_end
    li		t0, (0b11 << 11)
    csrw	mstatus, t0
    csrw    mie, zero
    la		t1, kinit
    csrw	mepc, t1
    la		ra, 2f
    mret

2:
    li t0, (0b01 << 11) | (1 << 7) | (1 << 5)
    csrw mstatus, t0
    la t2, asm_trap_vector
    csrw mtvec, t2
    la t1, kmain
    csrw mepc, t1
    li t2, 0x888
    csrw mie, t2
    la ra, 3f
    mret

3:
    wfi
    j 3b