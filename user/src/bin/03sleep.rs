#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
// use::log::*;
use user_lib::{get_time, yield_};
use user_lib::task_info;
use user_lib::logging;
use user_lib::taInfo::*;
use user_lib::syscall::{SYSCALL_EXIT,SYSCALL_GET_TIME,SYSCALL_TASK_INFO,SYSCALL_WRITE,SYSCALL_YIELD};
// #[no_mangle]
// fn main() -> i32 {
//     logging::init();
//     let appbegin = get_time();
//     debug!("the 03_APP start at  {}ms on user",appbegin);
//     let current_timer = get_time();
//     let wait_for = current_timer + 1000;
//     while get_time() < wait_for {
//         yield_();
//     }
//     println!("Test sleep OK!");
//     let append = get_time();
//     debug!("the 03_APP end at  {}ms on user",append);
//     0
// }
static  mut x : TaskInfo =TaskInfo{
    id : 3,
    status : TaskStatus::UnInit,
    call : [SyscallInfo{sysid :0,times : 0 }; MAX_SYSCALL_NUM],
    time : 0
};
#[no_mangle]
fn main() -> i32{
    logging::init();
    let current_timer = get_time();
    let wait_for = current_timer + 1000;
    while get_time() < wait_for {
        yield_();
    }
    println!("Test sleep OK!");


    unsafe{task_info(&x as *const TaskInfo as usize)};
    println!("--------------------------03user------------------------------------");
    unsafe{ 
        println!("app id                    =====> {}",x.id);
        println!("app status                =====> {:?}",x.status);
        println!("app time                  =====> {:?}",x.time);
        println!("SYSCALL_WRITE times       =====> {:?}",x.call[SYSCALL_WRITE]);
        println!("SYSCALL_EXIT times        =====> {:?}",x.call[SYSCALL_EXIT]);
        println!("SYSCALL_YIELD times       =====> {:?}",x.call[SYSCALL_YIELD]);
        println!("SYSCALL_GET_TIME times    =====> {:?}",x.call[SYSCALL_GET_TIME]);
        println!("SYSCALL_TASK_INFO times   =====> {:?}",x.call[SYSCALL_TASK_INFO]);
    };
    0
}
