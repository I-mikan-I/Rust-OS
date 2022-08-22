.option norvc
.altmacro
.set NUM_GP_REGS, 32
.set NUM_FP_REGS, 32
.set REG_SIZE, 8
.set MAX_CPUS, 8

.macro save_gp i, basereg=t6
    sd x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro load_gp i, basereg=t6
    ld x\i, ((\i)*REG_SIZE)(\basereg)
.endm

.section .text
.global asm_trap_vector
.align 4
asm_trap_vector:
    csrrw t6, mscratch, t6
    .set i, 1
    .rept 30
        save_gp %i
        .set i, i+1
    .endr

    mv t5, t6
    csrr t6, mscratch
    save_gp 31, t5
    csrw mscratch, t5

    csrr a0, mepc
    csrr a1, mtval
    csrr a2, mcause
    csrr a3, mhartid
    csrr a4, mstatus
    mv a5, t5
    la t0, KERNEL_STACK_END
    ld sp, 0(t0)
    call m_trap

    csrw mepc, a0
    csrr t6, mscratch

    .set i, 1
    .rept 31
        load_gp %i
        .set i, i+1
    .endr

    mret

.global switch_to_user
switch_to_user:
    # a0 = frame, a1 = pc, a2 = satp
    csrw mscratch, a0
    li t0, 1 << 7 | 1 << 5
    csrw mstatus, t0
    csrw mepc, a1
    csrw satp, a2
    li t1, 0xaaa
    csrw mie, t1
    la t2, asm_trap_vector
    csrw mtvec, t2

    srli a2, a2, 44
    li t0, 0xffff
    and a2, a2, t0
    sfence.vma x0, a2

    mv t6, a0
    .set i, 1
    .rept 31
        load_gp %i
        .set i, i+1
    .endr
    mret