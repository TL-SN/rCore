#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::vec::Vec;
use core::ptr::addr_of_mut;
use user_lib::{exit, get_time, thread_create, waittid};

static mut A: usize = 0;
const PER_THREAD_DEFAULT: usize = 10000;
const THREAD_COUNT_DEFAULT: usize = 16;
static mut PER_THREAD: usize = 0;

unsafe fn critical_section(t: &mut usize) {
    let a = addr_of_mut!(A);
    let cur = a.read_volatile();
    for _ in 0..500 {
        *t = (*t) * (*t) % 10007;
    }
    a.write_volatile(cur + 1);
}

unsafe fn f() -> ! {
    let mut t = 2usize;
    for _ in 0..PER_THREAD {
        critical_section(&mut t);
    }
    exit(t as i32)
}


// 这里共享资源仍然为全局变量 A ，具体操作为开 thread_count 个线程，每个线程执行 A=A+1 操作 PER_THREAD 次。
// 线程数和每个线程上的操作次数默认值分别由 THREAD_COUNT_DEFAULT 和 PER_THREAD_DEFAULT 给出，也可以通过命令行参数设置
#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    let mut thread_count = THREAD_COUNT_DEFAULT;
    let mut per_thread = PER_THREAD_DEFAULT;
    if argc >= 2 {
        thread_count = argv[1].parse().unwrap();
        if argc >= 3 {
            per_thread = argv[2].parse().unwrap();
        }
    }
    unsafe {
        PER_THREAD = per_thread;
    }
    let start = get_time();
    let mut v = Vec::new();
    for _ in 0..thread_count {
        v.push(thread_create(f as usize, 0) as usize);
    }
    for tid in v.into_iter() {
        waittid(tid);
    }
    println!("time cost is {}ms", get_time() - start);
    assert_eq!(unsafe { A }, unsafe { PER_THREAD } * thread_count);
    0
}
