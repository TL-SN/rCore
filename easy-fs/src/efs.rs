use super::{
    block_cache_sync_all, get_block_cache, Bitmap, BlockDevice, DiskInode, DiskInodeType, Inode,
    SuperBlock,
};
use crate::BLOCK_SZ;
use alloc::sync::Arc;
use spin::Mutex;
///An easy file system on block
pub struct EasyFileSystem {
    ///Real device
    pub block_device: Arc<dyn BlockDevice>,                 // 块设备指针
    ///Inode bitmap
    pub inode_bitmap: Bitmap,                               // inode位图
    ///Data bitmap
    pub data_bitmap: Bitmap,                                // 数据块位图
    inode_area_start_block: u32,                            // 索引开始块号
    data_area_start_block: u32,                             // 数据开始块号

}

type DataBlock = [u8; BLOCK_SZ];
/// An easy fs over a block device
impl EasyFileSystem {                                              // 创建并初始化一个 easy-fs 文件系统：
    /// A data block of block size      
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // calculate block size of areas & create bitmaps
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);// 0、选取下表为1的块作为inode位图开始的地方    
        let inode_num = inode_bitmap.maximum();                                         // 1_1、inode_bitmap可以存放的最大bit数
        let inode_area_blocks =                                                           // 1_2、inode_area_blocks区域可占最大快数(最大所占块数)
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;                 // 1_3、索引节点所占块总数
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;                    // 2_1、数据总快数
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;                       // 2_2、数据块所需位图块数
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;                    // 2_3、数据块区域所占块数
        let data_bitmap = Bitmap::new(                                                 // 2_3、为数据块分配位图
            (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        let mut efs = Self {                                                   // 3_1、创建efs文件系统
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // clear all blocks
        for i in 0..total_blocks {                                                        // 3_2、初始化EasyFileSystem
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // initialize SuperBlock                                                                // 3_3、初始化超级块(id=0)
        get_block_cache(0, Arc::clone(&block_device)).lock().modify( 
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        // write back immediately
        // create a inode for root node "/"
        assert_eq!(efs.alloc_inode(), 0);                                                      // 4_1、efs在InodeMap上分配一个bit(bit的id号一定为0，因为这是efs第一次调用InodeMap)
                                                                                               // 4_2、返回值 root_inode_block_id: 0号id在文件系统中对应的block的次序,root_inode_offset则是Inode相对于块的偏移
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0); 
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);                        // 4_3、初始化root目录索引节点
            });
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }


    /// Open a block device as a filesystem
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {                 //通过 open 方法可以从一个已写入了 easy-fs 镜像的块设备上打开我们的 easy-fs ：
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))        // 1、从block_device的0号块中读取出soperblock
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =                                                                    // 2、得到block_device的节点总快数
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {                                                                 // 3、根据block_device中superblock中记录的数据来初始化elf文件系统
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }
    /// Get the root inode of the filesystem
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {                      // 获取根目录的 inode ,因为根目录对应于文件系统中第一个分配的 inode ，因此它的 inode_id 总会是 0
        let block_device = Arc::clone(&efs.lock().block_device);
        // acquire efs lock temporarily
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        // release efs lock
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device,1)
    }
    /// Get inode by id
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {               // 返回值1: inode_id对应的是该id的diskInode索引节点对应的该文件系统的block号(次序)
        let inode_size = core::mem::size_of::<DiskInode>();                  // 返回值2: 该diskinode索引节点在块中的偏移
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;      // 每个块占有的inode数
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * inode_size,
        )
    }
    // 实验
    /// 通过block_id 与 offset 还原inode id
    pub fn get_inode_id_by_blockid_and_offset(&self, block_id: usize, offset: usize) -> usize {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_size) as usize;
        return (block_id - self.inode_area_start_block as usize) * inodes_per_block + offset / inode_size;
    }
    

    /// Get data block by id
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {              // 算出各个存储inode和数据块的磁盘块在磁盘上的实际位置。
        self.data_area_start_block + data_block_id
    }




    /// Allocate a new inode
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// Allocate a data block
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }
    /// Deallocate a data block
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }



}
