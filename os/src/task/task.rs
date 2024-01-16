//! Types related to task management
use super::TaskContext;
use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::trap::{trap_handler, TrapContext};

/// task control block structure
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,           // 位于应用地址空间次高页的 Trap 上下文被实际存放在物理页帧的物理页号 trap_cx_ppn
    pub base_size: usize,                   // base_size 统计了应用数据的大小，也就是在应用地址空间中从0x0开始到用户栈结束一共包含多少字节
    pub heap_bottom: usize,
    pub program_brk: usize,
}

impl TaskControlBlock {     // 任务控制块
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()          //get_mut 是个泛型函数，可以获取一个恰好放在一个物理页帧开头的类型为 T 的数据的可变引用
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn new(elf_data: &[u8], app_id: usize) -> Self {        // new 一个任务控制块
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);   // 为elf分配物理地址空间，并返回一些数据地址
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();     // 获得各自trapContext对应的物理页号，即次虚页号对应的物理页号
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);      // 各自以此紧挨着最高物理页搞了个内核栈(注意一页的间隙)
        KERNEL_SPACE.exclusive_access().insert_framed_area(                         // framed方式为该逻辑段分配物理页
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),   // 初始化taxtContext并赋予值，并压入 trap_return做ra，lab3中压入的是__restore
            memory_set,     // app的地址空间
            trap_cx_ppn,                // 位于应用地址空间次高页的 Trap 上下文被实际存放在物理页帧的物理页号 trap_cx_ppn
            base_size: user_sp,
            program_brk: user_sp,
            heap_bottom: user_sp,
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();   // 获取TrapContext的地址
        *trap_cx = TrapContext::app_init_context(                           // 为TrapContext赋初始值
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),            // 保存内核cr3的值
            kernel_stack_top,                                       // 内核栈顶虚拟地址
            trap_handler as usize,                                          // trap_handler的虚拟地址
        );
        task_control_block
    }
    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}
