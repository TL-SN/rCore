#![allow(missing_docs)]
#[derive(Copy, Clone,Debug)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

/// 定义最大的syscall id
pub const MAX_SYSCALL_NUM:usize = 500;
/// 定义
#[derive(Copy, Clone,Debug)]
pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub time: usize
}
#[derive(Copy, Clone,Debug)]
pub struct SyscallInfo {
    pub sysid: usize,
    pub times: usize
}
