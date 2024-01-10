#![allow(missing_docs)]

use super::super::task::task::TaskStatus;  // 第一个task 代表take模块,第二个task硕源到task.rs
/// 定义最大的syscall id
pub const MAX_SYSCALL_NUM:usize = 500;
/// 定义
#[derive(Copy, Clone)]
pub struct TaskInfo {
    ///
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; MAX_SYSCALL_NUM],
    pub time: usize
}
#[derive(Copy, Clone)]
pub struct SyscallInfo {
    pub sysid: usize,
    pub times: usize
}


use crate::task::TASK_MANAGER;

pub fn get_task_info(tsk :&mut TaskInfo) ->isize{
    let x = TASK_MANAGER.inner.exclusive_access();
    let id = tsk.id;
    
    let time = x.tasks[id].task_continue;       // 任务总运行时长
    let status = x.tasks[id].task_status;   // 任务状态
    let sys_statistics = x.tasks[id].sys_statistics.clone();
    tsk.status = status;
    tsk.time = time;
    tsk.call = sys_statistics.clone();
    0
}