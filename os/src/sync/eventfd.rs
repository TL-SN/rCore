const EMPTY : i32 =0;
const EFD_SEMAPHORE : i32 = 1;
const EFD_NONBLOCK : i32 = 1 << 11;
const EFD_SEMAPHORE_NONBLOCK : i32 = EFD_SEMAPHORE | EFD_NONBLOCK;

use crate::task::{current_process,block_current_and_run_next,wakeup_task,TaskControlBlock,current_task};
use crate::fs::File;
use alloc::sync::Arc;
use crate::sync::{UPSafeCell};
use core::cell::RefMut;
use alloc::{collections::VecDeque};



// 等我明天回来重写结构体吧...

pub struct Event{
    flag:i32,
    inner: UPSafeCell<EvCounterInner>,  
}


pub struct EvCounterInner{
    count: usize,                       // 计数器
    wait_queue: VecDeque<Arc<TaskControlBlock>>,            // 每个event_fd都对应了一个队列
    wait_queuew :VecDeque<Arc<TaskControlBlock>>, 
}



impl Event  {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, EvCounterInner> {
        self.inner.exclusive_access()
    }
    

    pub fn new(flag :i32,count :usize) ->Self{
        Self {
            flag: flag,
            inner: unsafe {
                UPSafeCell::new( EvCounterInner {
                    count: count,
                    wait_queue: VecDeque::new(),
                    wait_queuew : VecDeque::new(),
                })
            },
        }
    }
}





impl File for Event {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        true
    }
    fn write(&self, buf: crate::mm::UserBuffer) -> usize {

        let mut countinner = self.inner_exclusive_access();
        let count = countinner.count;

        // 判断长度
        let len = buf.len();
        if len > 8{
            return  usize::MAX;                                 // 1、长度错误
        }


        let mut new_count:usize = 0;
        let uit = buf.into_iter();
        let mut du : usize = 0;
        for num in uit{
            unsafe{
                let y = (*num as usize) << du;
                new_count += y;
                du += 8;
            }
        }
        
        if self.flag ==EMPTY{

            drop(countinner);
            
            loop {
                let mut countinner = self.inner_exclusive_access();
                let count = countinner.count;
                
                let (sum, overflowed) = count.overflowing_add(new_count);
                if overflowed == false{
                    countinner.count = sum;
                    while let Some(task) = countinner.wait_queue.pop_front() {
                        wakeup_task(task);                  // // 从等待队列中唤醒Read task任务
                    }
                    return 1;
                }else{
                    
                    countinner.wait_queuew.push_back(current_task().unwrap());
                    drop(countinner);//////
                    block_current_and_run_next();
                }
            }
                                                    //1、非信号量模式 => 增加值并wakeup read
        }else if self.flag == EFD_SEMAPHORE {       // 2、信号量模式下，count值加1
            
            if countinner.count == usize::MAX{          // // 在信号模式下，计数器加1发生溢出的话，就报错
                return usize::MAX;
            }             

            while let Some(task) = countinner.wait_queue.pop_front() {
                wakeup_task(task);                  // // 从等待队列中唤醒Read task任务
            }

            countinner.count += 1;
            return 1;
        }else if self.flag == EFD_NONBLOCK {        // 3、非信号量 + 不堵塞
            let (sum, overflowed) = count.overflowing_add(new_count);
            if overflowed{
                return usize::MAX;
            }
            
            countinner.count = sum;
        }else if self.flag == EFD_SEMAPHORE_NONBLOCK {        
            if countinner.count == usize::MAX{          // // 3、信号量 + 不堵塞，计数器加1发生溢出的话，就报错
                return usize::MAX;
            }             
            countinner.count += 1;
            return 1;
        }

        0
    }
    fn read(&self, buf: crate::mm::UserBuffer) -> usize {
        let mut countinner = self.inner_exclusive_access();
        let count = countinner.count;
        
                          //1_1、非信号量模式且计数器值不为0 => 清零
                         //1_2、非信号量模式且计数器为0  => 堵塞，直到出现其他数值  
        if self.flag == EMPTY{
            drop(countinner);
            loop {
                let mut countinner = self.inner_exclusive_access();
                let count = countinner.count;
                if count != 0{
                    countinner.count = 0;
                    // 唤醒被堵塞的write进程
                    while let Some(task) = countinner.wait_queuew.pop_front() {
                        wakeup_task(task);                  // // 从等待队列中唤醒task任务
                    }
                    return count;
                }
                else{
                    countinner.wait_queue.push_back(current_task().unwrap());   
                    drop(countinner);    
                    block_current_and_run_next();   
                }
            } 
        }else if self.flag == EFD_SEMAPHORE {  // 信号量条件下
            drop(countinner);
                                                // 2_1、信号量模式且计数器不为0 => 自减1
            loop{                               // //2_2、信号量模式且计数器为0    => 堵塞，一直到读写成功
                let mut countinner = self.inner_exclusive_access();
                let count = countinner.count;
                if count != 0{
                    countinner.count -= 1;
                    


                    return 1;
                }else{
                    countinner.wait_queue.push_back(current_task().unwrap());
                    drop(countinner);
                    block_current_and_run_next();
                }
            }
        }else if self.flag == EFD_NONBLOCK {
            if count != 0{
                countinner.count = 0;           //3_1、非堵塞且无信号量模式，且计数器值不为0 => 清零
                return count;
            }else{
                return usize::MAX;      //3_2、非堵塞且无信号量模式，且计数器值为0 => g
            }
        }else if self.flag == EFD_SEMAPHORE_NONBLOCK {
            if count != 0{                      //4_1、非堵塞且信号量模式下，且计数器非零  =>  计数器减一
                countinner.count -= 1;
                return 1;
            }else{                              //4_2、非堵塞且信号量模式下。且计数器为0  =>  g
                return usize::MAX;  
            }
        }


        
        0
    }
}


pub fn eventfd(initval: u32, flags: i32) -> i32{            // initval: 计数器的初值。
    
    // 检查标志位
    if flags != EMPTY && flags != EFD_SEMAPHORE && flags != EFD_NONBLOCK && flags != EFD_SEMAPHORE_NONBLOCK{
        return -1;
    }

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let fd = process_inner.alloc_fd();
    
    process_inner.fd_table[fd] = Some(Arc::new(Event::new(flags, initval as usize)));
                
    fd as i32
}