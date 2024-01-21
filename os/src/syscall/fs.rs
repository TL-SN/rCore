//! File and filesystem-related syscalls

use crate::fs::{open_file, OpenFlags, OSInode};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer,translated_refmut};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}


// 我们只需将进程控制块中的文件描述符表对应的一项改为 None 代表它已经空闲即可
// 同时这也会导致内层的引用计数类型 Arc 被销毁，会减少一个文件的引用计数，当引用计数减少到 0 之后文件所占用的资源就会被自动回收。
pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}



// olddirfd，newdirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。
// flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。

// oldpath：原有文件路径
// newpath: 新的链接文件路径。

// 在文件系统中创建一个新的目录项，该目录项与原始文件共享相同的inode。
use crate::fs::ROOT_INODE;
use alloc::sync::Arc;
use easy_fs::DirEntry;
pub fn sys_linkat(oldpath: *const u8,  newpath: *const u8) -> isize{

    let token = current_user_token();
    let old_path = translated_str(token, oldpath);
    let new_path = translated_str(token, newpath);
    // 1、先检查路径是否存在
    let all_path = ROOT_INODE.ls();
    if !all_path.contains(&old_path){
        return -1;
    }
    
    // 2、找到文件节点
    // let old_inode: alloc::sync::Arc<easy_fs::Inode> = ROOT_INODE.find(old_path.as_str()).unwrap();
    let old_inode  = ROOT_INODE.find_inode_id_by_root(old_path.as_str()).unwrap();
        


    // 3、为newpath创建一个目录项，并且Inode节点指向old_inode
    let dirent = DirEntry::new(new_path.as_str(),old_inode );
    ROOT_INODE.write_an_dir(dirent);
    
    // 4、调用ls命令检测一下
    println!("\n++++++++++++++++++ ls +++++++++++++++++++++++++");
    let all_file = ROOT_INODE.ls();
    for i in all_file{
        println!("{:?}",i);
    }
    println!("++++++++++++++++++++ ls end ++++++++++++++++++++++++\n");


    0
}



// dirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。
// flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。
// path：文件路径。
// linkat_test
pub fn sys_unlinkat(path: *const u8) -> isize{
    let token = current_user_token();
    let path = translated_str(token, path);
    // 1、检测路径是否存在
    let all_path = ROOT_INODE.ls();
    if !all_path.contains(&path){
        return -1;
    }
    
    // // 2、找到对应的inode号
    let old_inode  = ROOT_INODE.find_inode_id_by_root(path.as_str()).unwrap();


    // 3、删除对应目录项
    
    let ret = ROOT_INODE.delete_dir_enter_by_inode_and_name(path.as_str(),old_inode);
    
    
    // 4、检测一下是否真正的删除了
    println!("lslslslslslslslslslslslslslslslslslslslslslslslslslslslslslslslsls :");           
    let na = ROOT_INODE.ls();
    for i in na{
        println!("{:?}",i);
    }
    ret
}



// pub struct Stat {
//     /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
//     pub dev: u64,
//     /// inode 文件所在 inode 编号
//     pub ino: u64,
//     /// 文件类型
//     pub mode: StatMode,
//     /// 硬链接数量，初始为1
//     pub nlink: u32,
//     /// 无需考虑，为了兼容性设计
//     pad: [u64; 7],
// }



use crate::fs::Stat;
use crate::fs::{File,StatMode};
pub fn sys_stat(fd: isize, st: *mut Stat) -> isize{
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();
    
    if fd == -1{
        return  -1;
    }
    // 1、判断 fd长度是否正常(因为fs实际上就是进程中文件打开表的idx)
    if fd as usize >= inner.fd_table.len() {
        return -1;
    }

    // 2、判断fd有没有被open过(判断fd是否合法，因为只有open文件后fd_table[fd]中才有值)
    if inner.fd_table[fd as usize].is_none(){
        return -1;
    }

    // 3、取出inode节点
    let file =  inner.fd_table[fd as usize].clone().unwrap();
    let inode = file.get_osi();
    
     // 4、获取inode 号
    let ino = inode.get_inode_id();
    
    // 5、取出 nlink
    let nlink = ROOT_INODE.get_nlink(ino);


    
    // 6、查看是什么文件
    let st_mode = inode.is_dir();
    let mode ;
    if st_mode == 1{
        mode = StatMode::DIR;
    }else{
        mode = StatMode::FILE;
    }
    
    // 7、地址空间转换
    let stat = translated_refmut(token, st as *mut Stat);         // 地址空间转化
    
    stat.dev = 0;
    stat.ino = ino as u64;
    stat.mode = mode;
    stat.nlink = nlink as u32;
    
    
    0
}