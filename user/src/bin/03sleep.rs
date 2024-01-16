#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_time, yield_};

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 3000;
    println!("{}",current_timer);
    while get_time() < wait_for {
        // println!("{}",get_time());
        yield_();
    }
    println!("{}",get_time());
    println!("Test sleep OK!");
    0
}
