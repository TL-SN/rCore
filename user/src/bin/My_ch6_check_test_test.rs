#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{linkat,fstat,open,OpenFlags,Stat,StatMode, unlinkat};
#[no_mangle]
pub fn main() -> i32 {
    // 1、测试linkat
    println!("------------------------linkat_test start------------------------");
    
    let oldstr = "exit\0";
    let new_str = "new_exit\0";
    let ret = linkat(oldstr, new_str);
    if ret == -1{
        panic!("No No No ! linkat is error");
    }
    println!("+++++++++++++++++++++++++linkat_test end+++++++++++++++++++++++++");

    println!("");    
    

    // 2、测试stat
    println!("------------------------stat test start-----------------------------");

    
    let fd = open(oldstr, OpenFlags::RDWR);
    let sta = Stat::new(0, StatMode::NULL, 0);
    
    let ret = fstat(fd as isize, &sta );
    if ret == -1{
        panic!("No No No ! fstat is error");
    }

    println!("{:?}",sta);
    println!("-------------------------stat test end----------------------------");

    // 3、测试unlinkat
    println!("------------------------ unlinkat_test start------------------------");
    let ret = unlinkat(oldstr);
    if ret == -1{
        panic!("No No No ! unlinkat is error");
    }
    
    println!("---- again stat start ----");
    let sta = Stat::new(0, StatMode::NULL, 0);
    
    let ret = fstat(fd as isize, &sta );
    if ret == -1{
        panic!("No No No ! fstat is error");
    }
    println!("{:?}",sta);
    println!("-----again stat end -----");
    


    println!("+++++++++++++++++++++++++ unlinkat_test end+++++++++++++++++++++++++");


    0
}