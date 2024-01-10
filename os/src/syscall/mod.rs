//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.


const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_TASK_INFO : usize = 410;




mod fs;
mod process;
#[allow(non_snake_case)]
pub mod taInfo;

use fs::*;
use process::*;
use taInfo::TaskInfo;
use crate::task::TASK_MANAGER;


/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let mut x = TASK_MANAGER.inner.exclusive_access();
    let id = x.current_task;
    x.tasks[id].sys_statistics[syscall_id].sysid = syscall_id;
    x.tasks[id].sys_statistics[syscall_id].times += 1;
    // println!("\n[id] =>  {}------  syscall_id => {}  --------",id,syscall_id);
    // if id == 3 && syscall_id == SYSCALL_TASK_INFO{
    //     println!("args[0] :{:x}",args[0]);
    // }
    drop(x);
    
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_TASK_INFO => sys_task_info(unsafe { &mut *(args[0] as *mut TaskInfo) }),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
