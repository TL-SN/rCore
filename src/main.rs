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
    extern  "C" {
        fn stext();
        fn etext();
        
        fn srodata();
        fn erodata();


        fn sdata();
        fn edata();

        fn sbss();
        fn ebss();

        fn skernel();
        fn ekernel();
    }

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
    
    
    // 打印内存布局


    info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    error!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    debug!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);

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
