///
#[repr(C)]
#[derive(Debug)]


pub struct Stat {
    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}

bitflags! {
    /// 
    pub struct StatMode: u32 {
        ///
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

impl Stat {
    ///
    pub fn new(ino:u64,mode:StatMode,nlink:u32) -> Self{
        Self{
            dev :0,
            ino :ino,
            mode:mode,
            nlink : nlink,
            pad : [0;7],
        }
        
    }
    

}
