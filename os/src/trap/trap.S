# lab2-3.3
.altmacro                       # 我们需要在 trap.S 开头加上 .altmacro 才能正常使用 .rept 命令
.macro SAVE_GP n                # n代表寄存器的编号
    sd x\n, \n*8(sp)            # 这里的 x\n , 其中\n代表的变量n,即寄存器的编号，如果n等于5的话，\n就是5，x\n就是 x5 寄存器
                                # sp 代表栈指针 \n*8(sp)就代表 memory[5*8+sp]， 即表示从栈指针 sp 开始，向下（栈向低地址方向增长）偏移 \n*8 字节的地址。
                                # 所以这个宏的含义就是把 x\n中寄存器的值赋值给 memory[5*8+sp]处的八字节堆栈地址
.endm

.macro LOAD_GP n                # 这个宏的含义相当于把上面那个宏反过来
    ld x\n, \n*8(sp)
.endm

    .section .text
    .globl __alltraps
    .globl __restore
    .align 2

__alltraps:
    csrrw sp, sscratch, sp      # csrrw 原型是csrrw rd, csr, rs 可以将 CSR 当前的值读到通用寄存器rd 中，同时将通用寄存器rs 的值写入该 CSR，即交换对 sp 和 sscratch 两个寄存器内容
    # now sp->kernel stack, sscratch->user stack  #  在上一行之前 sp 指向用户栈， sscratch 指向内核栈（原因稍后说明），现在 sp 指向内核栈， sscratch 指向用户栈。
    # allocate a TrapContext on kernel stack
    addi sp, sp, -34*8          # 把sp栈指针上移/入栈 34 * 8个字节          
    # save general-purpose registers
    sd x1, 1*8(sp)              # 把x1寄存器存储在memory[8+sp]的栈地址中
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)              # 跳过x2寄存器，等会保存，我们把x3寄存器存储在memory[8*3+sp]的栈地址中
    # skip tp(x4), application does not use it  # 跳过x4寄存器，因为一般用不到
    # save x5~x31               # 下面保存 x5~x31寄存器
    .set n, 5                   # 循环保存 x5~x31寄存器
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    csrr t0, sstatus            # csrr rd,csr <=> 将CSR的值读到寄存器rd中  # 另外，需要注意的是x5-x7，x28-x31寄存器的别名是t0-t6
    csrr t1, sepc               # 把spce送到t1寄存器中    
    sd t0, 32*8(sp)             # 再把t0寄存器的值送入memory[sp+32*8]的栈位置
    sd t1, 33*8(sp)             # 同上
    # read user stack from sscratch and save it on the kernel stack
    csrr t2, sscratch           # 把sscratch指向的用户栈指针送给t2
    sd t2, 2*8(sp)              # 把t2加载到 memory[sp+2*8]的栈位置，填了上面的坑
    # set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp                   # a0 <-sp ,寄存器 a0 指向内核栈的栈指针也就是我们刚刚保存的 Trap 上下文的地址，
                                # 这是由于我们接下来要调用 trap_handler 进行 Trap 处理，它的第一个参数 cx 由调用规范要从 a0 中获取。
                                # 而 Trap 处理函数 trap_handler 需要 Trap 上下文的原因在于：它需要知道其中某些寄存器的值，
                                # 比如在系统调用的时候应用程序传过来的 syscall ID 和对应参数。我们不能直接使用这些寄存器现在的值，
                                # 因为它们可能已经被修改了，因此要去内核栈上找已经被保存下来的值。
    call trap_handler

__restore:
    # case1: start running app by __restore
    # case2: back to U after handling trap
    mv sp, a0                   # 先忽视这个指令，sp任然是内核栈               
    # now sp->kernel stack(after allocated), sscratch->user stack
    # restore sstatus/sepc
    ld t0, 32*8(sp)             # 把memory[sp+32*8]栈地址8字节数据送到t0寄存器中
    ld t1, 33*8(sp)             # 
    ld t2, 2*8(sp)              # 同上
    csrw sstatus, t0            # csrw mstatus, t0，将 t0 的值写入 mstatus
    csrw sepc, t1
    csrw sscratch, t2
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)              # 把memory[sp+1*8]栈地址8字节数据送到x1寄存器上
    ld x3, 3*8(sp)              # x2 保存的是 sp值，现在就改了就完蛋了，因此跳过x2  
    .set n, 5                   # 循环为寄存器赋值
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # release TrapContext on kernel stack
    addi sp, sp, 34*8           # 出栈，出34*8字节大小，即出栈34次
    # now sp->kernel stack, sscratch->user stack
    csrrw sp, sscratch, sp      # csrrw 原型是csrrw rd, csr, rs 可以将 CSR 当前的值读到通用寄存器rd 中，然后将通用寄存器rs 的值写入该 CSR
                                # 上一行之前 sp 指向内核栈， sscratch 指向用户栈，现在 sp 指向用户栈， sscratch 指向内核栈。
    sret                        # sret : 表示从 S 模式返回到 U 模式