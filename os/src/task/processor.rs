//!Implementation of [`Processor`] and Intersection of control flow
use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
///Processor management structure
pub struct Processor {                                          // 处理器管理结构 Processor 负责从任务管理器 TaskManager 中分出去的维护 CPU 状态的职责
    ///The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,                     // 当前处理器上正在执行的任务
    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,                                  // 表示当前处理器上的 idle 控制流的任务上下文
}
/// 谈谈对Processor的理解
/// Processor记录了目前运行着的程序的TaskControlBlock与TaskContext
/// Processor在进行new的时候初始话TaskContext为0，但我觉得他初始化为多少都无所谓，反正只会初始化一个全局Processor，并且在第一次switch的时候，其值会被刷新
/// Processor再每次switch的时候都会刷新(只要切换了任务就会刷新)





impl Processor {                                // Processor 是描述CPU 执行状态 的数据结构。在单核CPU环境下，我们仅创建单个 Processor 的全局实例 PROCESSOR 
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}
///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
///
const BIGSTRIDE:isize = 10000;
///
pub fn run_tasks() {
    // 实验二、实现stride调度算法
    

    loop {
        let mut processor = PROCESSOR.exclusive_access();
        // println!("~~~~~~~~~~~");
        if let Some(task) = fetch_task() {                      // 循环从队列中取出task控制块
            
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();        // 获取 当前的TaskContext
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;                                      // 取出next TaskContext，并赋status
            task_inner.stride += BIGSTRIDE/task_inner.priority;

            /*
            为什么这块代码会出错    
            // let pri = processor.current.as_ref().unwrap().pid.0.clone();
            // println!("++++++++++++++++++++++++++++++");
            // let later = task.pid.0;
            // println!("PID+++   {:x}  =>  {:x} ",pri,later);
            
            
            // let pri1 = processor.current.as_mut().unwrap().inner_exclusive_access().stride;
            // let later = task.inner_exclusive_access().stride;
            // println!("STRIDE+++   {:x}  =>  {:x}",pri1,later);
            // println!("\n");
            */

            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task); 
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        }
    }
}
///Take the current task,leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()                         // 取出当前正在执行的任务控制块
}
///Get running task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {                // 返回当前执行的任务的一份拷贝
    PROCESSOR.exclusive_access().current()
}
///Get token of the address space of current task
pub fn current_user_token() -> usize {                                  // 获取用户token
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}
///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {                  // 获取可变计数的 TrapContext
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}
///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {               // 返回到空闲控制流进行新的调度,上面介绍了从 idle 控制流通过任务调度切换到某个任务开始执行的过程。
                                                                        // 而反过来，当一个应用用尽了内核本轮分配给它的时间片或者它主动调用 yield 系统调用交出 CPU 使用权之后，
                                                                        // 内核会调用 schedule 函数来切换到 idle 控制流并开启新一轮的任务调度。
    

    
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr); 
    }
}

// 对schedule的理解
// 对于他的参数 switched_task_cx_ptr ， 当在exit函数中，其被初始化为0，其作用就是占位，进行切换
// 在suspend函数中，其被赋值为当前这个要suspend任务的trapContext，然后进行切换
// scahdule其实就是用于yeild、exit之类的切换，主要的任务运行/切换，还得看run_tasks
