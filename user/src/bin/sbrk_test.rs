// #![no_std]
// #![no_main]

// #[macro_use]
// extern crate user_lib;

// use core::ptr::slice_from_raw_parts_mut;
// use user_lib::sbrk;

// #[no_mangle]
// fn main() -> i32 {
//     println!("Test sbrk start.");
//     const PAGE_SIZE: usize = 0x1000;
//     let origin_brk = sbrk(0);
//     println!("origin break point = {:x}", origin_brk);
//     let brk = sbrk(PAGE_SIZE as i32);
//     if brk != origin_brk {
//         return -1;
//     }
//     let brk = sbrk(0);
//     println!("one page allocated,  break point = {:x}", brk);
//     println!("try write to allocated page");
//     let new_page = unsafe {
//         &mut *slice_from_raw_parts_mut(origin_brk as usize as *const u8 as *mut u8, PAGE_SIZE)
//     };
//     for pos in 0..PAGE_SIZE {
//         new_page[pos] = 1;
//     }
//     println!("write ok");
//     sbrk(PAGE_SIZE as i32 * 10);
//     let brk = sbrk(0);
//     println!("10 page allocated,  break point = {:x}", brk);
//     sbrk(PAGE_SIZE as i32 * -11);
//     let brk = sbrk(0);
//     println!("11 page DEALLOCATED,  break point = {:x}", brk);
//     println!("try DEALLOCATED more one page, should be failed.");
//     let ret = sbrk(PAGE_SIZE as i32 * -1);
//     if ret != -1 {
//         println!("Test sbrk failed!");
//         return -1;
//     }
//     println!("Test sbrk almost OK!");
//     println!("now write to deallocated page, should cause page fault.");
//     for pos in 0..PAGE_SIZE {
//         new_page[pos] = 2;
//     }

//     0
// }


#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{mmap,munmap,MREAD,MWRITE,MEXEC};
/// test mmap and munmap
#[no_mangle]
fn main() -> i32{
    println!("++++++++++++++++++++++++++++++++++++++++   a mmap and munmap tests begin ++++++++++++++++++++++++++++++++++++++++");

// test1 测试能不能正常的mmap与munmap
    // let start = 0x40000000;     // 基地址是0x40000000
    // let len = 0x10000;          // 长度是10页
    // let port = MREAD | MWRITE |MEXEC;
    
    // let ret = mmap(start, len, port);     //分配一个可读可写可执行的页面
    // if ret == 0{
    //     println!("no , mmap error");
    // }
    
    
    
    // let ret = munmap(start, len);
    // if ret == 0{
    //     println!("no , munmap error");
    // }



// test2、测试能否写入内存
    let start = 0x10000000;     // 基地址是0x40000000
    let len = 0x10000;          // 长度是10页
    let port = MREAD | MWRITE |MEXEC;

    let ret = mmap(start, len, port);     //分配一个可读可写可执行的页面
    if ret == 0{
        println!("++++++++++++++++++++++++++++++   no , mmap error  ++++++++++++++++++++++++++++++ ");
    }
    
    let test_data = "Hello , rCore OS";
    
    println!("++++++++++++++++++++++++++++++ test wirite ++++++++++++++++++++++++++++++ ");
    (start..start + test_data.len() as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(test_data.chars().nth(a - start).unwrap() as u8 )}
        
    });
    

    println!("++++++++++++++++++++++++++++++  test read  ++++++++++++++++++++++++++++++ ");

    (start..start + test_data.len() as usize).for_each(|a| {
        let x = unsafe { (a as *mut u8).read_volatile()};
        print!("{}",x as char);
    });
    println!("");

    println!("++++++++++++++++++++++++++++++  test munmap  ++++++++++++++++++++++++++++++ ");

    let ret = munmap(start, len);
    if ret == 0{
        println!("no , munmap error");
    }

// 3、测试double munmap

    println!("++++++++++++++++++++++++++++++  test double munmap  ++++++++++++++++++++++++++++++ ");
    let len = 0x10000000;       // 分配过量的数据
    let ret = munmap(start, len);
    if ret == 0{
        println!("no , munmap error");
    }

// 4、测试double mmap

    println!("++++++++++++++++++++++++++++++  test double munmap  ++++++++++++++++++++++++++++++ ");
    let start = 0x40000000;     // 基地址是0x40000000
    let len = 0x10000;          // 长度是10页
    let port = MREAD | MWRITE |MEXEC;

    let ret = mmap(start, len, port);     //分配一个可读可写可执行的页面
    if ret == 0{
        println!("no , mmap error");
    }
    
    let ret = mmap(start, 0x1000, port);
    if ret == 0{
        println!("no , double mmap is error");
    }


    0
}