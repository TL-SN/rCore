#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

use syscall::*;

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}

pub fn sbrk(size: i32) -> isize {
    sys_sbrk(size)
}

/// 要与kernel 的 MapPermission一致
pub const MREAD:usize = 1;
pub const MWRITE:usize = 2;
pub const MEXEC :usize = 4;

pub fn mmap(start : usize,len : usize,port : usize)  -> isize{
    sys_mmap(start, len, port)
}

pub fn munmap(start : usize,len : usize)  -> isize{
    sys_munmap(start, len)
}