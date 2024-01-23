mod action;
mod context;
mod manager;
mod pid;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::fs::{open_file, OpenFlags};
use crate::sbi::shutdown;
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
use manager::fetch_task;
use manager::remove_from_pid2task;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use action::{SignalAction, SignalActions};
pub use manager::{add_task, pid2task};
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
pub use signal::{SignalFlags, MAX_SIG};

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);              
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        if exit_code != 0 {
            //crate::sbi::shutdown(255); //255 == -1 for err hint
            shutdown(true)
        } else {
            //crate::sbi::shutdown(0); //0 for success hint
            shutdown(false)
        }
    }

    // remove from pid2task
    remove_from_pid2task(task.getpid());
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    // drop file descriptors
    inner.fd_table.clear();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    // println!(
    //     "[K] check_signals_error_of_current {:?}",
    //     task_inner.signals
    // );
    task_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
    // println!(
    //     "[K] current_add_signal:: current task sigflag {:?}",
    //     task_inner.signals
    // );
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            task_inner.frozen = true;                            //  frozen = true表示暂停进程
            task_inner.signals ^= SignalFlags::SIGSTOP;             // 清除掉待处理的signals信号(一定会存在的，因为先前判断了)
        }                                                           // 表示暂停进程
        SignalFlags::SIGCONT => {                                   // 表示运行进程执行
            if task_inner.signals.contains(SignalFlags::SIGCONT) {
                task_inner.signals ^= SignalFlags::SIGCONT;         
                task_inner.frozen = false;          // frozen=0表示继续进程
            }
        }
        _ => {
            // println!(
            //     "[K] call_kernel_signal_handler:: current task sigflag {:?}",
            //     task_inner.signals
            // );
            task_inner.killed = true;                           // 其他命令表示杀死当前进程
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let handler = task_inner.signal_actions.table[sig].handler;
    if handler != 0 {                                   // 1、检查是否设置了该signal的历程，没有设置的话则直接忽略
        // user handler

        // handle flag
        task_inner.handling_sig = sig as isize;         // 2、更新当前正在处理的历程
        task_inner.signals ^= signal;                   // 3、从待处理signal中删除本signal

        // backup trapframe
        let trap_ctx = task_inner.get_trap_cx();    
        task_inner.trap_ctx_backup = Some(*trap_ctx);   // 4、保存trapContext上下文

        // modify trapframe
        trap_ctx.sepc = handler;                        // 5、设置trapCOntext.spec = handler处理历程

        // put args (a0)
        trap_ctx.x[10] = sig;                           // 6、令a0 = sig 使得信号类型能够作为参数被例程接收。（把参数赋值给回调函数）
    } else {
        // default action
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
}

fn check_pending_signals() {
    for sig in 0..(MAX_SIG + 1) {                                                       // 0、遍历所有信号
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if task_inner.signals.contains(signal) && (!task_inner.signal_mask.contains(signal)) {          // 1、如果task的待处理signals列表里包含了该signal(条件1)并且
            let mut masked = true;                                                                // mask没有掩盖该siganl(条件2)
            let handling_sig = task_inner.handling_sig;
            if handling_sig == -1 {
                masked = false;
            } else {
                let handling_sig = handling_sig as usize;
                if !task_inner.signal_actions.table[handling_sig]                       // 2、检查该信号是否未被当前正在执行的信号处理例程屏蔽(条件3)
                    .mask
                    .contains(signal)
                {
                    masked = false;                                 // makeed = false 说明通过了上面三个条件
                }
            }
            if !masked {
                drop(task_inner);
                drop(task);
                if signal == SignalFlags::SIGKILL    // 终止某个进程，由内核或其他进程发送给被终止进程   // 3、如果信号类型为 SIGKILL/SIGSTOP/SIGCONT/SIGDEF 四者之一，则该信号只能由内核来处理
                    || signal == SignalFlags::SIGSTOP               // 也用于暂停进程，与 SIGTSTP 的区别在于 SIGSTOP 不能被忽略或捕获，即 SIGTSTP 更加灵活
                    || signal == SignalFlags::SIGCONT               // 恢复暂停的进程继续执行
                    || signal == SignalFlags::SIGDEF                // 默认
                {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {                                            // 4、否则调用 call_user_signal_handler 函数尝试使用进程提供的信号处理例程来处理。
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}


// 这个循环的意义在于：只要进程还处于暂停且未被杀死的状态就会停留在循环中等待 SIGCONT 信号的到来。如果 frozen 为真，证明还没有收到 SIGCONT 信号，进程仍处于暂停状态，
// 循环的末尾我们调用 suspend_current_and_run_next 函数切换到其他进程期待其他进程将 SIGCONT 信号发过来。
pub fn handle_signals() {
    loop {
        check_pending_signals();
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let task_inner = task.inner_exclusive_access();
            (task_inner.frozen, task_inner.killed)
        };
        if !frozen || killed {              // 如果进程被杀死 / 进程不停止就发生进程轮换，否则，就一直等待signal，一般的signal都是进程轮换，除非碰到SIGSTOP，这时只能用SIGCONT解
            break;
        }
        suspend_current_and_run_next();     // 发生进程轮换
    }
}
