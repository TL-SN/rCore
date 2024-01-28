use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

pub trait Mutex: Sync + Send {                  // 定义Mutex trait，(因为我们要实现多种锁)
    fn lock(&self);
    fn unlock(&self);
}

pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,                           // locked 作用和 之前介绍的单标记软件实现 相同，表示目前是否有线程进入临界区
    wait_queue: VecDeque<Arc<TaskControlBlock>>,        // wait_queue 作为阻塞队列记录所有等待 locked 变为 false 而被阻塞的线程控制块。
}

impl MutexBlocking {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {                                             // 1、首先检查是否已经有线程在临界区中
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();           // 2、如果 locked 为 true ，则将当前线程复制一份到阻塞队列中，然后调用 block_current_and_run_next 阻塞当前线程；否则当前线程可以进入临界区，将 locked 修改为 true 。
        } else {
            mutex_inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);        // 简单起见我们假定当前线程一定持有锁（也就是所有的线程一定将 lock 和 unlock 配对使用）
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() { // 尝试从阻塞队列中取出一个线程，如果存在的话就将这个线程唤醒。
            // 被唤醒的线程将继续执行 lock 并返回，进而回到用户态进入临界区。在此期间 locked 始终为 true ，相当于 释放锁的线程将锁直接移交给这次唤醒的线程 。
            // 反之，如果阻塞队列中没有线程的话，我们则将 locked 改成 false ，让后来的线程能够进入临界区。
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
