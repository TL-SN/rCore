//! Implementation of [`PageTableEntry`] and [`PageTable`].

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {             // 用于处理页表项中的标志位 PTEFlags 
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone,PartialEq)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {     // 传入的两个参数，一个所物理页号，一个所PTE标志位
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,            // ppn.0 << 10是因为在pte结构中PPN就是从第10位开始的
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {              // 转化为 PhysPageNum 
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {               // 获取PTE的标志号
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()   // 判断标V志号时候存在，下面几个函数同理
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {      // 页表管理器
    root_ppn: PhysPageNum,     // 根节点
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap(); // 分配一个物理页帧
        PageTable {
            root_ppn: frame.ppn,                           // 根_物理页号
            frames: vec![frame],                            // frame作为局部变量，变量的生命周期虽然结束，但其所有权进行了转移，转移到了PageTable上，资源的生命周期依然存在
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {            //  可以临时创建一个专用来手动查页表的 PageTable，查页目录表的物理页号                    
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)), // 取出satp寄存器的低44位，即物理页号位数，实际上三// 根据用户的根页目录表页号
            frames: Vec::new(),
        }
    }
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;    // 根物理页号地址
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {        // 发现有节点尚未创建则会新建一个节点
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);// 注意看，pte的类型为: &mut PageTableEntfy，那么*pte的意思就是相当于c语言的指针。修改它指向的地址的值
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;    // 获取页表目录的页表物理页号（根页表的物理页号）
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx]; // 获取ppn的页表中第 *idx的页表项
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();        // 获取下一级页表的物理页号
        }
        result
    }

    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {  //在多级页表中插入一个键值对，注意这里将物理页号 ppn 和页表项标志位 flags 作为不同的参数传入；
        let pte: &mut PageTableEntry = self.find_pte_create(vpn).unwrap();   // 首先先寻找该虚拟页号有没有被map过
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);    // 原来map的本质就是赋予一些符号位
        // println!("vpn: {:?}   ppn: {:?}   flags:  {:?}",vpn,ppn,flags|PTEFlags::V);
    }
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();  // 首先校测该虚拟地址是否合法，是否之前被map过
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();         //  PTE置0，那么标志位之类的也就全为0了
    }

    pub fn is_pte_valid(&self,vpn:VirtPageNum) -> usize{
        let x = self.find_pte(vpn);
        let y : Option<&mut PageTableEntry> = None;
        if x == y{
            return 0;           // 不合法
        }
        1                       // 合法
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)  // 查找虚拟地址对应的页表项的值，即页表项指向页的物理地址
    }
    pub fn token(&self) -> usize {              
        8usize << 60 | self.root_ppn.0
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {     // 将应用地址空间中一个缓冲区转化为在内核空间中能够直接访问的形式的辅助函数：
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);   // 起始地址的虚拟地址
        let mut vpn = start_va.floor();     // 起始地址的虚拟页号
        let ppn = page_table.translate(vpn).unwrap().ppn(); // 起始地址的物理页号
        vpn.step();
        
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}
