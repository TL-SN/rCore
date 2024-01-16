.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text.trampoline                   # 把下面这些汇编代码放置在.text.trampoline 段
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp                                          # 实验了一下，sscratch存储的值是0xFFFFE000，这样说的话，在外面，用rust创建的结构体在trap的时候会被
                                                                    # 保存在这里，甚至都保存在0xFFFFE000(不过每个用户的cr3不一样)
                                                                    # lab3是专门用几个内核栈/用户栈保存的
    # now sp->*TrapContext in user space, sscratch->user stack
    # save other general purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they have been saved in TrapContext
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it in TrapContext
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # load kernel_satp into t0
    ld t0, 34*8(sp)                 // 将内核地址空间的 token 载入到 t0 寄存器中；
    # load trap_handler into t1 
    ld t1, 36*8(sp)                 // 将 trap handler 入口点的虚拟地址载入到 t1 寄存器中；
    # move to kernel_sp
    ld sp, 35*8(sp)                 // 直接将 sp 修改为应用内核栈顶的地址；

    # switch to kernel space
    csrw satp, t0                   // 将 satp 修改为内核地址空间的 token 并使用 sfence.vma 刷新快表，这就切换到了内核地址空间；
    sfence.vma                      // 刷新tlb
    # jump to trap_handler
    jr t1                           // 跳转到t1的位置

__restore:
    # a0: *TrapContext in user space(Constant); a1: user space token
    # switch to user space
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapContext in user space, start restoring based on it
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # restore general purpose registers except x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # back to user stack
    ld sp, 2*8(sp)
    sret
