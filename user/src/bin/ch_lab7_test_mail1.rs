#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{fork,mailread,mailwrite,sleep};


// ch_lab7_test_mail1
#[no_mangle]
pub fn main() -> i32 {
    println!("------------------------------ mail test1 start ------------------------------ ");

    let pid = fork();
    if pid == 0{                // 子进程
        sleep(1000);           //先让子进程睡几秒，父进程先操作
        println!("------------------------- mailread start ------------------------- ");
        let buf = [0 as u8;30];
        let ret = mailread(buf.as_ptr() as usize as *mut u8,buf.len());
        
        

        println!("read bytes number is {}",ret);
        for i in 0..ret{
            print!("{}",buf[i as usize] as char);
        }
        println!("");
        

        
        println!("------------------------- mailread end ------------------------- ");
    }
    else{
        println!("------------------------- mailwrite start ------------------------- ");
        let buf = "Hello_World_rCore_OS";
        let ret = mailwrite(pid as usize,buf.as_ptr() as usize as *mut u8,buf.len());
        println!("write bytes number is {}",ret);
        println!("------------------------- mailwrite end ------------------------- ");
    }





    println!("------------------------------ mail test1 end ------------------------------ ");

    0
}