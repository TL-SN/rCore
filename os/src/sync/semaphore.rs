use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};

pub struct Semaphore {
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,                               // 信号量 信号量初始可用资源数量N  
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,              // 信号量初始可用资源数量N  
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn up(&self) {                                  // v操作
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {                       // 注意这个唤醒进程的条件
            if let Some(task) = inner.wait_queue.pop_front() {      // 唤醒一个线程
                wakeup_task(task);
            }
        }
    }

    pub fn down(&self) {                                // P操作
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {                            
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();               // 阻塞-唤醒
        }
    }
}
