
// use std::println;

use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    /// 
    pub block_id: usize,                                    // block_id 和 block_offset 记录该 Inode 对应的 DiskInode 保存在磁盘上的具体位置方便我们后续对它进行访问
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,                     // fs 是指向 EasyFileSystem 的一个指针，因为对 Inode 的种种操作实际上都是要通过底层的文件系统来完成。
    block_device: Arc<dyn BlockDevice>,
    nlink: usize,
}
//EasyFileSystem 实现了磁盘布局并能够将磁盘块有效的管理起来。但是对于文件系统的使用者而言，他们往往不关心磁盘布局是如何实现的，
//而是更希望能够直接看到目录树结构中逻辑上的文件和目录。为此需要设计索引节点 Inode 暴露给文件系统的使用者，让他们能够直接对文件和目录进行操作

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
        nlink:usize
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
            nlink,
        }
    }
    /// Call a function over a disk inode to read it                            // 读取 block_id 块对应block_offset的偏移对应的值，即dikeinode
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name               // 文件索引的查找比较简单，仅需在根目录的目录项中根据文件名找到文件的 inode 编号即可。由于没有子目录的存在，这个过程只会进行一次。
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {        // 返回一个inode id
        // assert it is a directory
        assert!(disk_inode.is_dir());                                              // 1、判断是不是文件
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;            // 2、判断文件目录中文件的数目
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),  // 3、读取数据到DirEntry
                DIRENT_SZ,
            );
            if dirent.name() == name {                                             // 4、判断名字是否相同
                return Some(dirent.inode_number() as u32);                          // 相同则返回inode节点号
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {    // 返回一个Inode              // find 方法只会被根目录 Inode 调用             
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {               
            self.find_inode_id(name, disk_inode).map(|inode_id| {       // 调用 find_inode_id 方法，尝试从根目录的 DiskInode 上找到要索引的文件名对应的 inode 编号
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                    self.nlink,
                ))
            })
        })
    }

    // 实验
    /// find inode_id               // linkat_test
    pub fn find_inode_id_by_root(&self,name: &str) -> Option<u32>{

        self.read_disk_inode(|disk_inode| {   
            self.find_inode_id(name, disk_inode)
        })
    

    }
    
    /// 写入DirEntry项
    pub fn write_an_dir(&self,dirent:DirEntry) -> isize{
        
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|root_inode| {                        // 3、将待创建文件的目录项插入到根目录的内容中，使得之后可以索引到。
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        
        0
    }

    /// 通过 block_id 与 block_offset还原 inode编号
    pub fn get_inode_id(&self) -> usize {
        self.fs.lock().get_inode_id_by_blockid_and_offset(self.block_id, self.block_offset)
    }

    /// 判断是否是目录
    pub fn is_dir(&self) -> usize{
        self.read_disk_inode(|disk_inode| {   
            if disk_inode.is_dir(){
                return 1;
            }else{
                return 0;
            }
        })
    }
    
    /// 返回nlink  // 本来我的思路三维护一个BTreeMap，但发现不好实现，最后借鉴的https://github.com/zhaiqiming/rCore-V3/blob/ch7/easy-fs/src/vfs.rs
    pub fn get_nlink(&self,ino:usize) -> usize{
        let mut ans: usize = 0;
        self.read_disk_inode(|disk_inode|{
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut dirent = DirEntry::empty();
            
            for i in 0..file_count {
                assert_eq!(
                    disk_inode.read_at(
                        DIRENT_SZ * i,
                        dirent.as_bytes_mut(),
                        &self.block_device,
                    ),
                    DIRENT_SZ,
                );

                if dirent.inode_number() as usize == ino {
                    ans += 1;
                }
            }
            
        });
        return ans;
    }

    ///  遍历根节点，删除目录项  linkat_test
    pub fn delete_dir_enter_by_inode_and_name(&self,path: &str,inode :u32)->isize{
        // let mut tag = 0;
        // self.read_disk_inode(|disk_inode|{
        
        //     let mut dirent = DirEntry::empty();
        //     let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        //     for i in 0..file_count{
        //         assert_eq!(
        //             disk_inode.read_at(
        //                 DIRENT_SZ * i,
        //                 dirent.as_bytes_mut(),
        //                 &self.block_device,
        //             ),
        //             DIRENT_SZ,
        //         );
        //         // println!("{:?} => {:?}",dirent.name(),dirent.inode_number());
        //         if dirent.inode_number() as u32 == inode && dirent.name() == path {     // 找到了这一目录项
        //             // let buf = "0"
        //             // assert_eq!(
        //             //     disk_inode.write_at(
        //             //         DIRENT_SZ * i,
        //             //         buf,
        //             //         &self.block_device,
        //             //     ),
        //             //     DIRENT_SZ
        //             // );
        //             tag = 1;
        //             // println!("1");
        //             // self.clear();       // 1、出错原因，clear()中调用的modify_disk_inode函数与这里调用的read_disk_inode函数互锁了
        //         }
        //         if tag == 1{
        //             break;
        //         }

        //     }
        // });
        // // self.clear();                      // 2、不能直接使用clear()的原因，他把所有DiskNode节点都删除了，相当于删除了所有文件，相当于直接回收了根节点
        // if tag == 0{
        //     return 0;
        // }
        // // let node = self.find(path).unwrap(); // 3、clear是回收文件内容的(对于目录来说是某一目录项，对文件来说是文件内容)，我们需要写一个对于文件来说回收目录项的函数
        // // node.clear();                        // 4、感觉不是很好写，这里走个捷径，直接把DirEntry目录项里的内容置空得了，同时ls的时候跳过空目录项
        

        let mut tag = 0;
        self.modify_disk_inode(|disk_inode| {
            let mut dirent = DirEntry::empty();
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count{
                assert_eq!(
                    disk_inode.read_at(
                        DIRENT_SZ * i,
                        dirent.as_bytes_mut(),
                        &self.block_device,
                    ),
                    DIRENT_SZ,
                );
                if dirent.inode_number() as u32 == inode && dirent.name() == path {
                    tag = 1;

                    // 写入
                    let emp = DirEntry::empty();
                    assert_eq!(
                        disk_inode.write_at(
                            DIRENT_SZ * i,
                            emp.as_bytes(),
                            &self.block_device,
                        ),
                        DIRENT_SZ
                    );
  
                }
                if tag == 1{
                    break;
                }
            }

        });

        tag
    }

    /// Increase the size of a disk inode
    fn increase_size(       // linkat_test
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name        // 在根目录下创建一个文件,该方法只有根目录的 Inode 会调用：
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {         // 1、检查是否在根目录
            // assert it is a directory                                               
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();                                    // 2_1、分配一个索引节点                         
        // initialize inode                                                          // 2_2、获取索引节点的块次序与偏移
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);                    //2_3、初始化索引节点
            });
        self.modify_disk_inode(|root_inode| {                        // 3、将待创建文件的目录项插入到根目录的内容中，使得之后可以索引到。
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
            self.nlink,
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {                                       // ls 方法可以收集根目录下的所有文件的文件名并以向量的形式返回，这个方法只有根目录的 Inode 才会调用：
        let _fs = self.fs.lock();
        let ans = DirEntry::empty();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;    // 1、计算根目录所包含的目录项
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                
                if dirent.name() == ans.name() && dirent.inode_number() == ans.inode_number(){  // 实验，没办法，回收某一目录项有点难操作...
                    continue;
                }
                v.push(String::from(dirent.name()));                            // 2、遍历目录项，记录目录项的Name
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode。 在以某些标志位打开文件（例如带有 CREATE 标志打开一个已经存在的文件）的时候，需要首先将文件清空。
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            // println!("size: {:?}",size);
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);        // 清空节点
            
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);         // 释放资源
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
