#![allow(unused)] // 

use core::arch::asm;
// const SBI_SET_TIMER: usize = 0;
// const SBI_CONSOLE_PUTCHAR: usize = 1;
// const SBI_CONSOLE_GETCHAR: usize = 2;
// const SBI_CLEAR_IPI: usize = 3;
// const SBI_SEND_IPI: usize = 4;
// const SBI_REMOTE_FENCE_I: usize = 5;
// const SBI_REMOTE_SFENCE_VMA: usize = 6;
// const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
// const SBI_SHUTDOWN: usize = 8;




// 实现ecall
#[inline(always)]
fn sbi_call(which :usize,arg0:usize,arg1:usize,arg2:usize) -> usize{
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") which,
        );
    }
    ret
}


// 在屏幕上输出一个字符
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

// 关机服务
pub fn shutdown(failure: bool) -> ! {
    use sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};
    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!()
}


pub fn sleep(time:usize){
    // sbi_rt:
}