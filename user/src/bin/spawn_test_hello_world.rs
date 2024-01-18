#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
pub fn main() ->isize{
    println!("Hello rCore OS World ~~~");

    0
}