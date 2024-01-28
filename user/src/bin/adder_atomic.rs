#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::vec::Vec;
use core::ptr::addr_of_mut;
use core::sync::atomic::{AtomicBool, Ordering};
use user_lib::{exit, get_time, thread_create, waittid, yield_};

static mut A: usize = 0;
static OCCUPIED: AtomicBool = AtomicBool::new(false);
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

// pub fn compare_exchange(
//     &self,
//     current: bool,
//     new: bool,
//     success: Ordering,
//     failure: Ordering,
// ) -> Result<bool, bool>;

// 其功能为：如果原子变量当前的值与 current 相同，则将原子变量的值修改为 new ，否则不进行修改。无论是否进行修改，都会返回原子变量在操作之前的值。
// 可以看到返回值是一个 Result ，如果修改成功的话这个值会用 Ok 包裹，否则则会用 Err 包裹。关于另外两个内存顺序参数 success 和 failure 不必深入了解，
// 在单核环境下使用 Ordering::Relaxed 即可。注意 compare_exchange 作为一个基于硬件的原子操作， 它不会被操作系统的调度打断 。


fn lock() {
    while OCCUPIED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        yield_();
    }
}

// unlock 的实现则比较简单，离开临界区的线程同样通过原子存储操作 store 将 OCCUPIED 修改为 false 表示已经没有线程在临界区中了，
// 此后线程可以进入临界区了。尝试运行一下 adder_atomic.rs ，可以看到它能够满足互斥访问需求。
fn unlock() {
    OCCUPIED.store(false, Ordering::Relaxed);
}

unsafe fn f() -> ! {
    let mut t = 2usize;
    for _ in 0..PER_THREAD {
        lock();
        critical_section(&mut t);
        unlock();
    }
    exit(t as i32)
}

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
