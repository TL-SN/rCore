pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x88000000;

pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine     
    (0x1000_1000, 0x00_1000), // Virtio Block in virt machine       // 在 config 子模块中我们硬编码 Qemu 上的 VirtIO 总线的 MMIO 地址区间（起始地址，长度）
];

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
