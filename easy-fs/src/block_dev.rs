use core::any::Any;

// 在 easy-fs 库的最底层声明了一个块设备的抽象接口 BlockDevice 
/// Trait for block devices
/// which reads and writes data in the unit of blocks
pub trait BlockDevice: Send + Sync + Any {
    ///Read data form block to buffer
    fn read_block(&self, block_id: usize, buf: &mut [u8]);                      // 将编号为 block_id 的块从磁盘读入内存中的缓冲区 buf ；
    ///Write data from buffer to block
    fn write_block(&self, block_id: usize, buf: &[u8]);                         // 将内存中的缓冲区 buf 中的数据写入磁盘编号为 block_id 的块
}

// 通常每个扇区为 512 字节。  
// 之前提到过 Linux 的Ext4文件系统的单个块大小默认为 4096 字节。在我们的 easy-fs 实现中一个块和一个扇区同为 512 字节，因此在后面的讲解中我们不再区分扇区和块的概念。