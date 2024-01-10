#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use::log::*;
use user_lib::{get_time};
const LEN: usize = 100;
use user_lib::logging;
#[no_mangle]
fn main() -> i32 {
    logging::init();
    let appbegin = get_time();
    debug!("the 02_APP start at  {}ms on user",appbegin);
    let p = 7u64;
    let m = 998244353u64;
    let iter: usize = 160000;
    let mut s = [0u64; LEN];
    let mut cur = 0usize;
    s[cur] = 1;
    for i in 1..=iter {
        let next = if cur + 1 == LEN { 0 } else { cur + 1 };
        s[next] = s[cur] * p % m;
        cur = next;
        if i % 10000 == 0 {
            println!("power_7 [{}/{}]", i, iter);
        }
    }
    println!("{}^{} = {}(MOD {})", p, iter, s[cur], m);
    println!("Test power_7 OK!");
    let append = get_time();
    debug!("the 02_APP end at  {}ms on user",append);
    0
}
