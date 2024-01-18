//!Implementation of [`TaskControlBlock`]
use super::TaskContext;
use super::{pid_alloc, KernelStack, PidHandle};
use crate::config::TRAP_CONTEXT;
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefMut;

pub struct TaskControlBlock {                       // 这个叫任务控制块，而不是任务管理器，任务管理器在manager.rs
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,                   // Trap上下文的物理页号，TrapContext
    pub base_size: usize,                           // 应用数据仅有可能出现在应用地址空间低于 base_size 字节的区域中。借助它我们可以清楚的知道应用有多少数据驻留在内存中。
    pub task_cx: TaskContext,                       // TaskContext
    pub task_status: TaskStatus,                    // task status
    pub memory_set: MemorySet,                      // 进程内存管理
    pub parent: Option<Weak<TaskControlBlock>>,     // 指向父进程控制块，(用weak包裹着不影响父进程的引用计数)
    pub children: Vec<Arc<TaskControlBlock>>,       // 指向子进程的任务控制块
    pub exit_code: i32,                             // 退出码，会由exit_code回收
    pub stride :isize,                              // 当前进程已经运行的长度
    pub priority: isize,                            // 当前进程的优先级（进程优先级>=2并初始优先级为16）
}

impl TaskControlBlockInner {
    /*
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
    */
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {             // 
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
    pub fn new(elf_data: &[u8]) -> Self {                                               // 创建一个任务控制块，也就是创建一个新进程，目前仅用于内核中手动创建唯一一个初始进程 initproc
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();                                                                      // Trap上下文的物理页号
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();                                          // 分配一个pid
        let kernel_stack = KernelStack::new(&pid_handle);                   // 新分配内核栈
        let kernel_stack_top = kernel_stack.get_top();                              
        // push a task context which goes to trap_return to the top of kernel stack
        let task_control_block = Self {                                 // 初始化任务控制块
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    stride : 0,
                    priority :16                                        // 初始化的时候优先级设置为16
                })
            },
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();            // 准备TrapContext
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    pub fn spawn(self: &Arc<Self> ,elf_data: &[u8]) -> Arc<Self> {
        let (memort_set,user_sp,entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memort_set
        .translate(VirtAddr::from(TRAP_CONTEXT).into())
        .unwrap()
        .ppn();                                             // 获取COntext ppn 的物理页号
    
        let pid_handle = pid_alloc();                       // 获取一个pid
        let kernel_stack = KernelStack::new(&pid_handle);                              // 分配内核栈
        let kernel_stack_top = kernel_stack.get_top();
        


        // 1、为task_control_block 初始化
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack:kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,                                                  // 甚至base_size都一致
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set : memort_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    stride : 0,
                    priority :16                                        // 初始化的时候优先级设置为16
                })
            },
        });

        let mut parent_inner = self.inner_exclusive_access();
        
        parent_inner.children.push(task_control_block.clone());                                     // 父进程记录子进程
        let trap_cx: &mut TrapContext = task_control_block.inner_exclusive_access().get_trap_cx();
        
        
        // 2、为TrapContext赋值 --- 1、入口点、用户栈、token、内核栈、traphander地址
        *trap_cx = TrapContext::app_init_context(entry_point, user_sp, KERNEL_SPACE.exclusive_access().token(), kernel_stack_top, trap_handler as usize);

        // 3、之后还要把该任务插入任务管理器中

        task_control_block

    }


    pub fn exec(&self, elf_data: &[u8]) {                       // 主要用于实现sys_exec系统调用，当前进程加载并执行另一个 ELF 格式可执行文件
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // **** access inner exclusively
        let mut inner = self.inner_exclusive_access();
        // substitute memory_set
        inner.memory_set = memory_set;                                                      // 直接是地址空间的替换
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;                                                    // trapCOntext物理页的替换
        // initialize base_size
        inner.base_size = user_sp;                                                          // base_size的替换
        // initialize trap_cx
        let trap_cx = inner.get_trap_cx();                                  // TrapContext更换
        *trap_cx = TrapContext::app_init_context(                                             // 赋予
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        // **** release inner automatically
    }
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {                            // fork进程
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();
        // copy user space(include trap context)
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);        // 复制父进程空间
        
        // println!("=> me: {:x}    father: {:x}  <=",memory_set.page_table.token(),parent_inner.memory_set.page_table.token());  // 好吧，token是不一样的，没有实现COW

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();                                                                                 // 获取trapContext的ppn，即trapContext的物理页，这里就记录了父进程的trapContext (因为目前处于父进程状态，用的是父进程的cr3)
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();                                                   // 分配一个pid
        let kernel_stack = KernelStack::new(&pid_handle);                              // 分配内核栈
        let kernel_stack_top = kernel_stack.get_top();
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,                                                  // 甚至base_size都一致
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    stride : 0,
                    priority :16                                        // 初始化的时候优先级设置为16
                })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());                                     // 父进程记录子进程
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx: &mut TrapContext = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;


        // return
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }
    pub fn getpid(&self) -> usize {                             // 返回pid
        self.pid.0
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}
