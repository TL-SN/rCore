mod context;

use crate::batch::run_next_app;
use crate::syscall::syscall;
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);// // 把trap陷入地址写入0x105号CSR寄存器((这个型号的寄存器就是用来保存陷入地址的))
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {             // byd这个参数还是通过__alltraps 里的a0寄存器传的
    let scause = scause::read(); // get trap cause      // 获取trap发生的原因
    let stval = stval::read(); // get extra value   // 
    match scause.cause() {                          // match 枚举原因
        Trap::Exception(Exception::UserEnvCall) => {        // 用户层执行Trap指令
            cx.sepc += 4;               // 在 Trap 返回之后，我们希望应用程序控制流从 ecall 的下一条指令开始执行
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;  //系统调用，传入a0~a2与syscall的ID号a7，并用a0接受返回值
        }
        
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            run_next_app();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::TrapContext;
