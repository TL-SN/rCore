use super::BlockDevice;
use crate::mm::{
    frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    StepByOne, VirtAddr,
};
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static, VirtioHal>>);  // 我们将 virtio-drivers crate 提供的 VirtIO 块设备抽象 VirtIOBlk 包装为我们自己的 VirtIOBlock
                                                                    // 只是加了一层互斥锁而已

lazy_static! {
    static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> = unsafe { UPSafeCell::new(Vec::new()) };
}

// 在 qemu 上，我们使用 VirtIOBlock 访问 VirtIO 块设备；而在 k210 上，我们使用 SDCardWrapper 来访问插入 k210 
// 开发板上真实的 microSD 卡，它们都实现了 easy-fs 要求的 BlockDevice Trait 

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

impl VirtIOBlock {                  
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),     //  VirtIOHeader 实际上就代表以 MMIO 方式访问 VirtIO 设备所需的一组设备寄存器
            ))                      // 因此我们从 qemu-system-riscv64 平台上的 Virtio MMIO 区间左端 VIRTIO0 开始转化为一个 &mut VirtIOHeader 就可以在该平台上访问这些设备寄存器了。
        }
    }
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let mut ppn_base = PhysPageNum(0);                          
        for i in 0..pages {
            let frame = frame_alloc().unwrap();                 // 分配物理帧
            if i == 0 {
                ppn_base = frame.ppn;                                          // 记录物理帧的物理页号
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            QUEUE_FRAMES.exclusive_access().push(frame);                        // 插入Qemu保存的物理帧容器中QUEUE_FRAMES
        }
        let pa: PhysAddr = ppn_base.into();                                     // 返回物理地址(把物理帧转换为物理地址)
        pa.0
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {                        // 参数1: 要dealloc的物理页帧地址
        let pa = PhysAddr::from(pa);                              // 参数2: 要dealloc的物理页帧数
        let mut ppn_base: PhysPageNum = pa.into();                          // 严格来说并不保证分配的连续性。幸运的是，这个过程只会发生在内核初始化阶段，因此能够保证连续性。
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    fn phys_to_virt(addr: usize) -> usize {                                     // 物理地址转虚拟地址 （因为是直接映射方式）
        addr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        PageTable::from_token(kernel_token())
            .translate_va(VirtAddr::from(vaddr))
            .unwrap()
            .0
    }
}
