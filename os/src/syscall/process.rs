use crate::loader::get_app_data_by_name;
use crate::mm::{translated_refmut, translated_str};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next,
};
use crate::timer::get_time_ms;
use alloc::sync::Arc;

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {              // 父进程等待子进程、回收子进程(主要是回收僵尸进程)
    let task = current_task().unwrap();
    // find a child process

    // ---- access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())           // 判断pid是否符合条件，当传入的 pid 为 -1 的时候，任何一个子进程都算是符合要求；但 pid 不为 -1 的时候，则只有 PID 恰好与 pid 相同的子进程才算符合条件
    {
        return -1;                                                      // 如果当前的进程不存在一个进程 ID 为 pid（pid==-1 或 pid > 0）的子进程，则返回 -1
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB lock exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });                                                                                 // 寻找子进程的僵尸进程
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);             // 子进程从向量中移除并置于当前上下文中
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);                                       // 确认这是对于该子进程控制块的唯一一次强引用
        let found_pid = child.getpid();                                                 // 得到子进程pid
        // ++++ temporarily access child TCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;                   // 得到子进程退出码
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;         // 写入到当前进程的应用地址空间中  //由于应用传递给内核的仅仅是一个指向应用地址空间中保存子进程返回值的内存区域的指针，我们还需要在 translated_refmut 中手动查页表找到应该写入到物理内存中的哪个位置，这样才能把子进程的退出码 exit_code 返回给父进程
        found_pid as isize                                                              // 主要就是将 exit_code 通过translated_refmut 写入父进程 // 这是sys_waitpid为用户保留的参数
    } else {
        -2                                          // 它的含义是子进程还没退出
    }
    // ---- release current PCB lock automatically
}



pub fn sys_spawn(path: *const u8) -> isize{             // 成功返回子进程id，否则返回 -1    // 至于为什么是 *const 类型，因为值
    let token = current_user_token();
    let path = translated_str(token, path);         // 把*const u8类型的字符串转化成 String类型的字符串
    if let Some(data) = get_app_data_by_name(path.as_str()){
        let task = current_task().unwrap();     // 拿到当前任务控制块

        
        let new_task = task.spawn(data);  
        let new_pid = new_task.pid.0;
        add_task(new_task);
        return new_pid as isize
    } 
    -1                              // spawn失败

}

// syscall ID：140
// 设置当前进程优先级为 prio
// 参数：prio 进程优先级，要求 prio >= 2
// 返回值：如果输入合法则返回 prio，否则返回 -1
pub fn sys_set_priority(prio :isize) -> isize{ 
    if prio <= 1{
        return -1;
    }
    
    
    let task = current_task().unwrap();
    println!("+++ pri:   {}",task.inner_exclusive_access().priority);
    task.inner_exclusive_access().priority = prio;                      
    println!("--- later:   {}",task.inner_exclusive_access().priority);
                                                                         // 设置优先级  // 另外，不加，mut的原因 :当你的代码中使用类似 
                                                                         //inner_exclusive_access() 这样的方法时，这个方法可能返回一个具有内部可变性的类型的可变引用（
                                                                         // 例如 RefCell<T>、Cell<T>、Mutex<T>、RwLock<T> 等）。这些类型的特点是它们允许你在拥有不可变引用的情况下改变存储的值。
                                                                         // 这是通过封装数据访问来实现的，以确保安全性和一致性。
    drop(task);

    prio
}