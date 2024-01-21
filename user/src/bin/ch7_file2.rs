#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{close, fstat, linkat, open, read, unlinkat, write, OpenFlags, Stat,StatMode};

/// 测试 link/unlink，输出　Test link OK! 就算正确。

#[no_mangle]
pub fn main() -> i32 {
    let test_str = "Hello, world!";
    let fname = "fname2\0";
    let (lname0, lname1, lname2) = ("linkname0\0", "linkname1\0", "linkname2\0");
    let fd = open(fname, OpenFlags::CREATE | OpenFlags::WRONLY);
    linkat(fname, lname0);
    let stat = Stat::new(0, StatMode::NULL, 0);
    fstat(fd, &stat);
    assert_eq!(stat.nlink, 2);
    linkat(fname, lname1);
    linkat(fname, lname2);
    fstat(fd, &stat);
    assert_eq!(stat.nlink, 4);
    write(fd as usize, test_str.as_bytes());
    close(fd as usize);

    unlinkat(fname);
    let fd = open(lname0, OpenFlags::RDONLY);
    let stat2 = Stat::new(0, StatMode::NULL, 0);
    let mut buf = [0u8; 100];
    let read_len = read(fd as usize, &mut buf) as usize;
    assert_eq!(test_str, core::str::from_utf8(&buf[..read_len]).unwrap(),);
    fstat(fd, &stat2);
    assert_eq!(stat2.dev, stat.dev);
    assert_eq!(stat2.ino, stat.ino);
    assert_eq!(stat2.nlink, 3);
    unlinkat(lname1);
    unlinkat(lname2);
    fstat(fd, &stat2);
    assert_eq!(stat2.nlink, 1);
    close(fd as usize);
    unlinkat(lname0);
    // It's Ok if you don't delete the inode and data blocks.
    println!("Test link OK!");
    0
}