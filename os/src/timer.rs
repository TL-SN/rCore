//! RISC-V timer-related functionality

use crate::config::CLOCK_FREQ;  //时钟频率
use crate::sbi::set_timer;
use riscv::register::time;

use log::*;
const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;
// const USEC_PER_SEC: usize = 1000000;
// const PSEC_PER_SEC: usize = 1000000000;
/// read the `mtime` register
pub fn get_time() -> usize {
    time::read()            // 它们都是 M 特权级的 CSR ，而我们的内核处在 S 特权级，是不被允许直接访问它们的。好在运行在 M 特权级的 SEE （这里是RustSBI）已经预留了相应的接口，我们可以调用它们来间接实现计时器的控制
}

/// get current time in milliseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

/// set the next timer interrupt
pub fn set_next_trigger() {         // 目前是10ms一时钟中断
    let x = get_time() + CLOCK_FREQ / TICKS_PER_SEC;
    debug!("set next trigger : at {}ms",x / (CLOCK_FREQ / MSEC_PER_SEC));
    set_timer(x);
}




// get current time in milliseconds
// pub fn get_time_us() -> usize {
//     time::read() / (CLOCK_FREQ / USEC_PER_SEC)
// }

// pub fn get_time_ps() ->usize{
//     time::read() / (CLOCK_FREQ / PSEC_PER_SEC)
// }