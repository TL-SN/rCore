use crate::sync::{Mutex, UPSafeCell};
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};

pub struct Condvar {
    pub inner: UPSafeCell<CondvarInner>,
}

pub struct CondvarInner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(CondvarInner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn signal(&self) {      // 从阻塞队列中移除一个线程并调用唤醒原语 wakeup_task 将其唤醒。注意如果此时阻塞队列为空则此操作不会有任何影响；
        let mut inner = self.inner.exclusive_access();
        if let Some(task) = inner.wait_queue.pop_front() {
            wakeup_task(task);                                  // 从等待队列中唤醒task任务
        }
    }

    pub fn wait(&self, mutex: Arc<dyn Mutex>) {     // wait 接收一个当前线程持有的锁作为参数。首先将锁释放，然后将当前线程挂在条件变量阻塞队列中，
                                    //之后调用阻塞原语 block_current_and_run_next 阻塞当前线程。在被唤醒之后还需要重新获取锁，这样 wait 才能返回。
        mutex.unlock();
        let mut inner = self.inner.exclusive_access();
        inner.wait_queue.push_back(current_task().unwrap());                // 插入到等待队列中
        drop(inner);
        block_current_and_run_next();
        mutex.lock();
    }
}
