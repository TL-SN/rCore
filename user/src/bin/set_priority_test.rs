#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::set_priority;

#[no_mangle]
pub fn main() ->isize{
    
    set_priority(1);
    

    
    0
}