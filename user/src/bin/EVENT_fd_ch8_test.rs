#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{eventfd,read,write,fork,sleep};
const EMPTY : i32 =0;
const EFD_SEMAPHORE : i32 = 1;
const EFD_NONBLOCK : i32 = 1 << 11;
const EFD_SEMAPHORE_NONBLOCK : i32 = EFD_SEMAPHORE | EFD_NONBLOCK;

// EVENT_fd_ch8_test
#[no_mangle]
fn main() -> i32{
    // println!("------------------------ eventfd test1 start ------------------------");
    // let evfd = eventfd(0, 0) as usize;
    // println!("evfd: {:x}",evfd);
    // let mut buf:[u8;4] = [0x12,0x34,0x56,0x78];
    // let ret = write(evfd, &mut buf);
    // println!("ret : {:x}",ret);
    
    // let mut buf:[u8;4] = [0;4];
    // let ret = read(evfd,&mut buf);
    // println!("ret : {:x}",ret);
    // println!("buf : {:?}",buf);
    // println!("------------------------ eventfd test1 end ------------------------");


    // println!("------------------------ eventfd test2 start ------------------------");
    // let evfd = eventfd(0, 0) as usize;
    
    // let pid = fork();
    // if pid == 0{                    // 子进程，消费者
    //     sleep(20);
    //     let mut buf:[u8;4] = [0;4];
    //     let ret = read(evfd, &mut buf);
    //     println!("ret=>: {:x}",ret);
    // }else {                         // 父进程，生产者
    //     let mut buf:[u8;4] = [0x12,0x34,0x56,0x78];
    //     write(evfd, &mut buf);

    // }


    // println!("------------------------ eventfd test2 end ------------------------");

    println!("------------------------ eventfd test3 start ------------------------");
    let evfd = eventfd(0, EFD_SEMAPHORE) as usize;
    
    let pid = fork();
    if pid == 0{                    // 子进程，消费者
        
        let mut buf:[u8;4] = [0;4];
        println!("we will pass the test, if the 'ret=>' occured after 2000ms");
        let ret = read(evfd, &mut buf);         // 用read堵塞进程
        println!("ret=>: {:x}",ret);
    
    }else {                         // 父进程，生产者
        sleep(2000);
        let mut buf:[u8;4] = [0x12,0x34,0x56,0x78];         // write之后，回复read进程(子进程)
        write(evfd, &mut buf);
    }


    println!("------------------------ eventfd test3 end ------------------------");




    0
}