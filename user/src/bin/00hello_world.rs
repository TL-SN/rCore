#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("\n----------00_app----------");
    println!("Hello, world!");
    0
}
