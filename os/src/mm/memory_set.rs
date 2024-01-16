//! Implementation of [`MapArea`] and [`MemorySet`].

use super::{frame_alloc, FrameTracker};
use super::{PTEFlags, PageTable, PageTableEntry};
use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::{StepByOne, VPNRange};
use crate::config::{MEMORY_END, MMIO, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE};
use crate::sync::UPSafeCell;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::*;
use riscv::register::satp;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {              // 创建内核地址空间
    /// a memory set instance through lazy_static! managing kernel space
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });      // 这里使用 Arc<UPSafeCell<T>> 组合是因为我们既需要 Arc<T> 提供的共享 引用，也需要 UPSafeCell<T> 提供的内部可变引用访问。
}

/// memory set structure, controls virtual-memory space
pub struct MemorySet {                      // 地址空间，是一系列有关联的不一定连续的逻辑段
                                            // 这种关联一般是指这些逻辑段组成的虚拟内存空间与一个运行的程序绑定，
                                            // 即这个运行的程序对代码和数据的直接访问范围限制在它关联的虚拟地址空间之内。
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub fn new_bare() -> Self {             // 创建一个新的地址空间
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }


    /// Assume that no conflicts.
    pub fn insert_framed_area(              // 可以在当前地址空间插入一个 Framed 方式映射到物理内存的逻辑段。注意该方法的调用者要保证同一地址空间内的任意两个逻辑段不能存在交集，从后面即将分别介绍的内核和应用的地址空间布局可以看出这一要求得到了保证；
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission), 
            None,
        );
    }
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {        // 插入一个逻辑段MapArea
        map_area.map(&mut self.page_table);             // 1、MapArea的new只提供了虚拟地址，这个push函数就是调用MapArea的map函数来分配物理地址空间
                                                        // 2、并把这个MapArea逻辑段放入自己的vec容器中
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data); // 3、同时把虚拟页上的数据同步到物理页上
        }                                                // map函数内部还插入了虚页与物理页的键值对
        self.areas.push(map_area);
    }

        // 实验、插入一个逻辑段
    pub fn push_mmap(&mut self,start_add: VirtAddr,len:usize,port:usize){
        
        let end:usize = usize::from(start_add) +len;

        // 出错原因1:我以为 MapPermission R是1，w是2，x是4呢，转换一下吧
        // 出错原因2:遗漏了U位! ，U位三决定用户能不能访问的一个标志位
        let port = port << 1;
        let map_perm = MapPermission::from_bits(port as u8).unwrap() | MapPermission::U;
        


        self.push(
            MapArea::new(
                start_add,
                VirtAddr::from(end),
                MapType::Framed,
                map_perm,
            ),
            None,
        );
        
   
    }   
    // 实现、剔除一个逻辑段  ，写push后面
    pub fn remove_munmap(&mut self,start: VirtAddr,len:usize) -> usize{
        // 我们要干的是 :1、删除键值对 2、删除地址空间对应Area的Vec 3、修改符号位 4、调用dealloc 5、判断长度
        // 一个unmap函数可以解决 1、3
        // 使用frame_alloc函数使用了RAII的思想，利用了生命周期能自动调用dealloc，释放资源，所以4也被解决了
        // 我们只需要解决2与5即可

        // for MpAa in self.areas{
        //     for i in MpAa.vpn_range{
                
        //     }
        // }

        // 感觉用引用计数比较稳
        
        let page_len = usize::from(VirtAddr::from(len).ceil()) ;

        let mut ret = 0;
        let mut tag = 0;
        let mut count = 0;
        for mp_aa in self.areas.iter_mut(){  
            for vpn in mp_aa.vpn_range{
                if vpn == start.floor(){            // 找到要删除的地方了
                    // 判断长度
                    let mut lenn = 0;
                    let x = mp_aa.vpn_range.clone();
                    for _ in x{
                        lenn += 1;
                    }
                    // println!("{:x}",lenn);
                    // println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");                    
                    if page_len > lenn{      //妄想释放内存过大
                        return 0;
                    }
                
                    // unmap 掉
                    mp_aa.unmap(&mut self.page_table);
                    tag = 1;
                    ret = 1;    // 找到了
                    break;
                }
            }
            if tag == 1{
                break;
            }

            count +=1;
        }

        self.areas.remove(count);
        
        ret as usize
        
    }




    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {                      // 这里相当于把最大的虚拟页号映射到了trap跳板
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),      // pub const TRAMPOLINE: usize = 18446744073709547520 (0xFFFFFFFFFFFFF000)
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }
    /// Without kernel stacks.
    pub fn new_kernel() -> Self {   //可以生成内核的地址空间，映射跳板和地址空间中最低256G中的内核逻辑段
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();        // 最大虚拟页号 => 跳板
        // map kernel sections
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        println!("mapping .text section");
        
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        println!("mapping .rodata section");


        // use crate::mm::frame_allocator::FRAME_ALLOCATOR;
        // let x = FRAME_ALLOCATOR.exclusive_access().current;
        // println!("{:x}",x);

        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );





        println!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // let y = FRAME_ALLOCATOR.exclusive_access().current;
        // println!("{:x}",y);


        println!("mapping memory-mapped registers");
        for pair in MMIO {
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }
        memory_set
    }
    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {      // 分析应用的 ELF 文件格式的内容，解析出各数据段并生成对应的地址空间
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();        // 都以最高虚拟地址为跳板，也就是说所有的app包括os，其最高虚拟页号都映射着trap跳板
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");            // elf文件的魔数
        let ph_count = elf_header.pt2.ph_count();                           // 得到 program header 的数目
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {            // 如果是load 段
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);  // 为Load段分配Fremed类型的物理地址
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        // use crate::mm::frame_allocator::FRAME_ALLOCATOR;
        // let x = FRAME_ALLOCATOR.exclusive_access().current;
        // println!("{:x}",x);


        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;                 // 加这个PAGE_SIZE是为了设置保护页
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );




        // used in sbrk
        memory_set.push(
            MapArea::new(
                user_stack_top.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );


        // // use crate::mm::frame_allocator::FRAME_ALLOCATOR;
        // let y = FRAME_ALLOCATOR.exclusive_access().current;
        // println!("{:x}",y);

        // map TrapContext
        memory_set.push(                        // 为TrapContext 配置一物理页，这里固定在最高地址根页表的下面
            MapArea::new(
                TRAP_CONTEXT.into(),    // 注意这里，这里把为TrapContext都映射到了次高虚页，但注意，这里没有与任何的固定的物理页号进行映射，也就是说，这里的物理页号是被随机分配的，
                                                 // 也就是说每个app都把自己次高续页当做TrapContext，即app内核栈
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        (                                           // 我们不仅返回应用地址空间 memory_set ，也同时返回用户栈虚拟地址 user_stack_top 以及从解析 ELF 得到的该应用入口点地址，它们将被我们用来创建应用的任务控制块。
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }
    pub fn activate(&self) {
        let satp = self.page_table.token();     // token实则是  8usize << 60 | self.root_ppn.0 
                                                        // 这里主要是，设置修改satp的mode字段，使其启动SV39 分页机制
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");                         // sfence.vma指令清空TLB，防止使用过期的键值对
        }
    }

    // 实验、判断pte是否有效
    pub fn is_pte_valid(&self,vpn: VirtPageNum) -> usize{  // 使用translate来判断的话太致命了,这里可以模拟translate写一个不致命的函数
        self.page_table.is_pte_valid(vpn)
    }


    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
    #[allow(unused)]
    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.shrink_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
    #[allow(unused)]
    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.append_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
}

/// map area structure, controls a contiguous piece of virtual memory
pub struct MapArea {        // 逻辑段, 该区间内包含的所有虚拟页面都以一种相同的方式映射到物理页帧
                            // 阅读后面的代码可以发现，这里就是为一个一个app的逻辑段(.text、.data等等)分配相应的物理地址空间的
    vpn_range: VPNRange,            // VPNRange 描述一段虚拟页号的连续区间,可以使用 Rust 的语法糖 for-loop 进行迭代。
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,       // 虚拟页号与对应物理页号的键值对 
    map_type: MapType,              // 描述该逻辑段内的所有虚拟页面映射到物理页帧的同一种方式，它是一个枚举类型
    map_perm: MapPermission,        // 页的四个标志位
}
// 注意 PageTable 下挂着所有多级页表的节点所在的物理页帧，而每个 MapArea 下则挂着对应逻辑段中的数据所在的物理页帧，这两部分合在一起构成了一个地址空间所需的所有物理页帧。
// 这同样是一种 RAII 风格，当一个地址空间 MemorySet 生命周期结束后，这些物理页帧都会被回收。


impl MapArea {          // new 含税提供虚拟起始终止地址、map类型、权限即可
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),   
            map_type,
            map_perm,
        }
    }
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);           // 恒等映射下获取虚拟地址的物理页号，当以恒等映射 Identical 方式映射的时候，物理页号就等于虚拟页号
                                                    // 不过这里却没有让alloc的计数+1
            }                              
            MapType::Framed => {                    // 虚地址与物理地址的映射关系是相对随机的
                let frame = frame_alloc().unwrap();  // 只要没物理内存，立马g
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);

                // let x = self.data_frames.get(&vpn);
                // println!("vpn: {:?}   =>  ppn{:?} ",vpn , x);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        // println!("vpn: => {:?}  ppn: => {:?}  pte_flags: => {:?}",vpn,ppn,pte_flags);
        page_table.map(vpn, ppn, pte_flags);
    }
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    #[allow(unused)]
    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
            self.unmap_one(page_table, vpn)
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }
    #[allow(unused)]
    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
            self.map_one(page_table, vpn)
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {      // 把data数据复制到物理页表上，此后，用户访问虚拟页号就能访问到物理页号
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapType {
    Identical,      // dentical 表示上一节提到的恒等映射方式
    Framed,         //  Framed 则表示对于每个虚拟页面都有一个新分配的物理页帧与之对应，虚地址与物理地址的映射关系是相对随机的
}

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    println!("remap_test passed!");
}
