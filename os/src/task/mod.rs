//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;

#[allow(clippy::module_inception)]
pub mod task;  // 加上pub就可以让其他目录的.rs文件访问到task模块下task.rs的内容了

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};
use log::*;
pub use context::TaskContext;
use crate::timer::{get_time,get_time_ms};
use crate::syscall::taInfo::SyscallInfo;
use crate::syscall::taInfo::MAX_SYSCALL_NUM;
/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    pub num_app: usize,
    /// use inner value to get mutable access
    pub inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    pub tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    pub current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            task_begin:0,
            task_stop :0,
            task_continue : 0,
            sys_statistics: [SyscallInfo{sysid :666666666,times :0};MAX_SYSCALL_NUM],
            id:0,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
 
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        
        task0.task_status = TaskStatus::Running;
        task0.task_begin = get_time();
        task0.id = 0;
        task0.task_continue = 0;
        task0.task_stop = get_time();
        


        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        let kbegin = get_time_ms();
        // 2
        
        debug!("the First APP start at  {}ms on kernel",kbegin);
        
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
        let x = get_time();
        inner.tasks[current].task_continue += x - inner.tasks[current].task_stop;
        inner.tasks[current].task_stop = x;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
        let kend = get_time_ms();
        let x = get_time();
        inner.tasks[current].task_continue += x - inner.tasks[current].task_stop;
        inner.tasks[current].task_stop = x;
        debug!("the 0{}_APP end at  {}ms on kernel",current,kend);
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;

            let x = get_time();
            inner.tasks[current].task_continue += x - inner.tasks[current].task_stop;
            inner.tasks[current].task_stop = x;
            
            // inner.tasks[next].task_continue +=x - inner.tasks[next].task_stop;
            inner.tasks[next].task_stop = x;
            inner.tasks[next].id = next;

            
            let kend = get_time_ms();
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            // 编程题---T1  扩展内核，能够显示操作系统切换任务的过程
            debug!("task{}  =>   task{}",current,next);    
            // 编程题---T2  扩展内核，能够统计每个应用执行后的完成时间：用户态完成时间和内核态完成时间
            
            debug!("the 0{}_APP start at  {}ms on kernel",next,kend);
            
            // println!("111111111111111111111");
            // let pc: usize;
            // use core::arch::asm;
            // unsafe {
            //     asm!(
            //         "auipc {0}, 0",
            //         out(reg) pc
            //     );
            // }

            // println!("Program Counter: {:#x}", pc);
            // 任务四---统计__switch的消耗
            let kbegin = get_time();
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            let kend = get_time();
            debug!("__switch cost {}",kend-kbegin);
            // println!("222222222222222222222");
            // go back to user mode
        } else {
            println!("All applications completed!");
            shutdown(false);
        }
    }
}

/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// suspend current task
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// exit current task
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
