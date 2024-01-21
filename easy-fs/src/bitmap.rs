use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;
/// A bitmap block
type BitmapBlock = [u64; 64];                                   // BitmapBlock 是一个磁盘数据结构，它将位图区域中的一个磁盘块解释为长度为 64 的一个 u64 数组
                                                                // 每个 u64 打包了一组 64 bits
/// Number of bits in a block
const BLOCK_BITS: usize = BLOCK_SZ * 8;
/// A bitmap
/// 每个位图都由若干个块组成，每个块大小为 512 bytes，即 4096 bits。每个 bit 都代表一个索引节点/数据块的分配状态， 0 意味着未分配，而 1 则意味着已经分配出去。
pub struct Bitmap {
    start_block_id: usize,                                          // 区域的起始块编号
    blocks: usize,                                                  // 区域长度
}
/// 这里一个Bitmap结构体包含了好几个块，每个块都用来做位视图
// Bitmap 自身是驻留在内存中的


/// Decompose bits into (block_pos, bits64_pos, inner_pos)
fn decomposition(mut bit: usize) -> (usize, usize, usize) {    // decomposition
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    /// A new bitmap from start block id and number of blocks
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }
    /// Allocate a new block from a block device
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {         // 0、返回值是bit相对于该BitMap的第几位
        for block_id in 0..self.blocks {
            let pos = get_block_cache(            // 获取块缓冲              // 1、block_id + self.start_block_id 是为了遍历该BitMap的每个块
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX)                   //2、 **bits64 != u64::MAX的意思是找到一个尚未完全分配出去的组，若完全分配，那就是64个1了，即u64::MAX
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize)) //3、 最后得到的 bits64_pos 是BLOCK的下表，inner_pos是，连续1开始的下表
                {
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;                                          //4、在对应 BitmapBlock 中增加一个1bit(变1)
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)                      //5、返回值是增加bit在这个Bitmap的的第几位，即返回分配的bit编号
                } else {
                    None
                }
            });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }
    /// Deallocate a block
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);            // 把bit序列分解 
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;                                      // 复位
            });
    }
    /// Get the max number of allocatable blocks
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
