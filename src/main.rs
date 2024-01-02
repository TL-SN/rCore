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
// 10、借助9.1实现的println!宏处理致命错误,rust的宏处理有待学习


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
global_asm!(include_str!("entry.asm")); //4、将同目录下的汇编代码 entry.asm 转化为字符串并通过 global_asm! 宏嵌入到代码中





#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("Hello, world!");
    

    // lab1 job，achive Implement fonts with different colors
    logging::init();
    error!("Man!");
    warn!("what can I say,");
    info!("Man Ba out!");
    debug!("2024");
    trace!("happy new year");
    println!("Miss you, Lao Da");
    println!("o.o   o.O");
    panic!("Shutdown machine!");

}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}