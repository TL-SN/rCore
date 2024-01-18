#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::spawn;
use user_lib::exec;
#[no_mangle]
pub fn main() ->isize{
    println!("+++++ spawn_test_start....................");
    spawn("spawn_test_hello_world\0");
    // exec()
    println!("----- spawn_test_end.....................");
    
    0
}