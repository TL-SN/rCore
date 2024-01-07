//! App management syscalls
use crate::batch::run_next_app;

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {      // 每次调用sys_exit后都会run_next_app
    println!("[kernel] Application exited with code {}", exit_code);
    run_next_app()
}
