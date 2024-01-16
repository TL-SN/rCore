//! Implementation of [`FrameAllocator`] which
//! controls all the frames in the operating system.

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// manage a frame which has the same lifecycle as the tracker
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();        // 初始化页帧，clear
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

trait FrameAllocator {          // 描述一个物理页帧管理器需要提供哪些功能
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// an implementation for frame allocator
pub struct StackFrameAllocator {    // 物理页帧管理器
    pub current: usize,             //空闲内存的起始物理页号
    end: usize,                 //空闲内存的结束物理页号
    recycled: Vec<usize>,
}

impl StackFrameAllocator {          // 物理页帧管理器初始化
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        // println!("{:x}  ==================  {:x}",l.0,r.0);
    }

    #[allow(unused)]
    pub fn is_addr_space_sufficient(&self,len:usize) -> usize{          // 判断地址空间是否够
        let lenn = PhysAddr::from(len).ceil();
        if self.current +lenn.0 > self.end{
            return  0;                      //地址空间不够
        }  
        return 1;                           // 地址空间足够
    }


}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {  
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),           // 回收页帧，方便回收利用
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())                // 首先会检查栈 recycled 内有没有之前回收的物理页号，如果有的话直接弹出栈顶并返回，不过这里只判断了栈顶，如果可以的话，我觉得可以设置个大小适中的集合
        } else if self.current == self.end {
            None                            // 页帧耗尽，用完了
        } else {
            self.current += 1;              // 分配一帧
            Some((self.current - 1).into())
        }
    }
    
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // println!("ppn---> {:?}",ppn);
        // validity check       // 首先检查回收页帧的合法性---该页面之前一定被分配出去过，因此它的物理页号一定<current
                                //  该页面没有正处在回收状态，即它的物理页号不能在栈 recycled 中找到。
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }


    
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {                  // 每次对该分配器进行操作之前，我们都需要先通过 FRAME_ALLOCATOR.exclusive_access() 拿到分配器的可变借用。
    /// frame allocator instance through lazy_static!
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// initiate the frame allocator using `ekernel` and `MEMORY_END`
pub fn init_frame_allocator() {     // 初始化FRAME_ALLOCATOR
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(            // 获取可变空闲内存的起始物理页号与终止物理页号
        PhysAddr::from(ekernel as usize).ceil(),        // 以ekernel为启示页号
        PhysAddr::from(MEMORY_END).floor(),             // 所有可分配空间的最后页号
    );
}

/// allocate a frame
pub fn frame_alloc() -> Option<FrameTracker> {   // 分配/回收物理页帧的接口
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

/// deallocate a frame
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
/// a simple test for frame allocator
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
