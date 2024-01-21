use super::{BlockDevice, BLOCK_SZ};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;
/// Cached block inside memory
pub struct BlockCache {
    /// cached block data
    cache: [u8; BLOCK_SZ],                                      // cache大小与磁盘块大小相同的
    /// underlying block id
    block_id: usize,                                            // 记录了这个块缓冲来自磁盘的块的编号
    /// underlying block device
    block_device: Arc<dyn BlockDevice>,                         // 是一个底层块设备的引用，可通过它进行块读写；
    /// whether the block is dirty
    modified: bool,                                             // 脏位，记录这个块从磁盘载入内存缓存之后，它有没有被修改过。
}


impl BlockCache {
    /// Load a new BlockCache from disk.
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {               // 1、创建一个BlockCache
        let mut cache = [0u8; BLOCK_SZ];                                         // 2、将触发一次 read_block 将一个块上的数据从磁盘读到缓冲区 cache
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }
    /// Get the address of an offset inside the cached block data
    fn addr_of_offset(&self, offset: usize) -> usize {                                      // 1、操作磁盘内存缓存---获取缓存块数据中偏移量的地址
        &self.cache[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T                                           // 2、操作磁盘内存缓存---它可以获取缓冲区中的位于偏移量 offset 的一个类型为 T 的磁盘上数据结构的不可变引用
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }

    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T                                   // 3、操作磁盘内存缓存---get_mut 会获取磁盘上数据结构的可变引用，由此可以对数据结构进行修改
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {             // FnOnce作为参数的闭包，所有权被传递到闭包，外部可以调用`FnOnce`类型的闭包至多为一次
        f(self.get_ref(offset))                                                         // 该闭包是接受T类型，返回V类型
    }

    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

// 在 Linux 中，通常有一个后台进程负责定期将内存中缓冲区的内容写回磁盘。另外有一个 sys_fsync 系统调用可以让应用主动通知内核将一个文件的修改同步回磁盘。由于我们的实现比较简单， sync 仅会在 BlockCache 被 drop 时才会被调用。

    pub fn sync(&mut self) {                                                                   // 当 BlockCache 的生命周期结束之后缓冲区也会被从内存中回收，这个时候 modified 标记将会决定数据是否需要写回磁盘：
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}


impl Drop for BlockCache {                                                                     // RAII思想
    fn drop(&mut self) {
        self.sync()
    }
}
/// Use a block cache of 16 blocks
const BLOCK_CACHE_SIZE: usize = 16;                                             // 最多允许内存驻留16个磁盘块缓冲区

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,                           // 同时提供共享引用和互斥访问
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn get_block_cache(                                                     // 尝试从块缓存管理器中获取一个编号为 block_id 的块的块缓存
                                                                                // 如果找不到，会从磁盘读取到内存中，还有可能会发生缓存替换：

        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Arc::clone(&pair.1)
        } else {                                                                // 1、如果找不到
            // substitute
            if self.queue.len() == BLOCK_CACHE_SIZE {                           // 2_1、并且当前当前queue已满，需要执行缓存替换算法，丢掉某个块缓存并空出一个空位
                // from front to tail
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                {
                    self.queue.drain(idx..=idx);                         // 2_2、这里使用一种类 FIFO 算法：每加入一个块缓存时要从队尾加入；要替换时则从队头弹出。
                                                                               // 但此时队头对应的块缓存可能仍在使用：判断的标志是其强引用计数 
                                                                               // 即除了块缓存管理器保留的一份副本之外，在外面还有若干份副本正在使用
                                                                               // 即删除一个强引用为一的cache块
                } else {
                    panic!("Run out of BlockCache!");
                }
            }                                                                       
            // load block into mem and push back                               // 2_3、我们创建一个新的块缓存（会触发 read_block 进行块读取）并加入到队尾，最后返回给请求者。
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {                                                                      // 块缓冲全局管理器
    /// The global block cache manager
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}
/// Get the block cache corresponding to the given block id and block device
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}
/// Sync all block cache to block device
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
