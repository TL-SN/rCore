// qemu-system-riscv64 \
//     -machine virt \
//     -nographic \
//     -bios '/home/tlsn/Desktop/OSSS/lab1/os/rustsbi-qemu.bin'  \
//     -device loader,file=./os.bin,addr=0x80200000 \
//     -s -S

// riscv64-unknown-elf-gdb \                         [8:37:18]
//     -ex '/home/tlsn/Desktop/OSSS/lab1/os/os/target/riscv64gc-unknown-none-elf/release/os'  \
//     -ex 'set arch riscv:rv64' \
//     -ex 'target remote localhost:1234'


// lab1
// 1、移除std
// 2、手动实现一个panic
// 3、移除main函数      // 编译器提醒我们缺少一个名为 start 的语义项。我们回忆一下，之前提到语言标准库和三方库作为应用程序的执行环境，需要负责在执行应用程序之前进行一些初始化工作，然后才跳转到应用程序的入口点（也就是跳转到我们编写的 main 函数）开始执行。事实上 start 语义项代表了标准库 std 在执行应用程序之前需要进行的一些初始化工作。由于我们禁用了标准库，编译器也就找不到这项功能的实现了。
// 4、我们通过 include_str! 宏将同目录下的汇编代码 entry.asm 转化为字符串并通过 global_asm! 宏嵌入到代码中。
// 5、通过链接脚本调整链接器的行为，使得最终生成的可执行文件的内存布局符合Qemu的预期，即内核第一条指令的地址应该位于 0x80200000 
// 至此，我们拥有了第一行汇编 代码: 80200000: 93 00 40 06  	li	ra, 100
// 6、剔除多余元数据 rust-objcopy --strip-all os -O binary os.bin
// 7、更新汇编与链接器，创造栈空间(创建到了bss上)
// 8、rust清零.bss段
// 9、实现RustSBI,ecall ，并实现一些简单的功能,如输出一个字符，关机,RustSBI 开源社区的 sbi_rt 封装了调用 SBI 服务的接口，我们直接使用即可
// 9.1、继续补充功能，实现格式化输出
// 10、借助9.1实现的println!宏处理致命错误
// 11、使用彩色log输出

// lab2
// l2-1、static mut 变量的访问控制都是 unsafe 的,不太安全，我们选择使用RefCell智能指针来实现AppManager 结构体，再封装一个UPSafeCell使之允许我们在 单核 上安全使用可变全局变量
// l2-2、完善AppMananger功能
// l2-3.1、(特权级切换前的操作)设置内核栈与用户栈
//  3.2、 设置TrapContext，保存上下文
//  3.3、 用汇编实现Trap上下文的切换
//  3.4、 实现Trap 分发与处理，实现trap_handler
//  l2-4、  为内核实现系统调用



#![no_std]  
#![no_main]
#![feature(panic_info_message)]
#[macro_use]            //#[macro_use] 可以使被注解的module模块中的宏应用到当前作用域中；或者注释crate中的宏应用到当前crate作用域中
mod console;    
mod lang_items;
mod sbi;
mod logging;
use log::{error,warn,info,debug,trace};
use core::{arch::{global_asm}};

pub mod trap;
pub mod syscall;
mod stack_trace;
mod sync;
pub mod batch;

global_asm!(include_str!("entry.asm")); //4、将同目录下的汇编代码 entry.asm 转化为字符串并通过 global_asm! 宏嵌入到代码中
global_asm!(include_str!("link_app.S"));


fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}



#[no_mangle]
pub fn rust_main() -> ! {
    extern "C" {
        fn stext(); // begin addr of text segment
        fn etext(); // end addr of text segment
        fn srodata(); // start addr of Read-Only data segment
        fn erodata(); // end addr of Read-Only data ssegment
        fn sdata(); // start addr of data segment
        fn edata(); // end addr of data segment
        fn sbss(); // start addr of BSS segment
        fn ebss(); // end addr of BSS segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_top(); // stack top
    }
    clear_bss();
    logging::init();
    println!("[kernel] Hello, world!");
    trace!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize,
        etext as usize
    );
    debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
    info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
    warn!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );
    error!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    // panic!("111111111111111111111111111111111111111111111111111111111111111111111111111111111111");
    trap::init();
    batch::init();
    batch::run_next_app();
    
}


