use crate::task::{current_task, current_user_token};
use crate::mm::translated_refmut;
use crate::task::pid2task;

// ch_lab7_test_mail1
pub fn sys_mailread(buf: *mut u8,len: usize) -> isize{                  // 读是读自己的信箱
    if len > 256{
        return  -1;
    }
    if len == 0{
        return 0;
    }
    
    let token = current_user_token();
    let task = current_task().unwrap();
    
    let mut inner  = task.inner_exclusive_access();
    
    let wbuf = translated_refmut(token, buf as *mut usize); 
    inner.mail.read_mail(wbuf as *mut usize as usize)
}

// ch_lab7_test_mail1
// ch_lab7_test_mail1
// ch_lab7_test_mail1
pub fn sys_mailwrite(pid: usize, buf: *mut u8,len: usize) -> isize{     // 写是写给别人
    

    if let Some(tsk) = pid2task(pid){
        let mut inner = tsk.inner_exclusive_access();
        let token = current_user_token();
        let wbuf = translated_refmut(token, buf as *mut usize);        // 地址转化,用户地址转换为内核地址
        // println!("buf *mut u8 :{:x}",buf as usize);
        // println!("wbuf_addr: {:x}",wbuf);
        inner.mail.write_mail(wbuf as *mut usize as usize, len)
    }else{
        return -2;                              // 错误的pid
    }
    
}