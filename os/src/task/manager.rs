//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {                                                  // 任务管理器
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),                           //新建一个任务管理器，其实全局也就一个
        }
    }
    ///Add a task to `TaskManager`
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {            // 往任务管理器中添加任务，Arc智能指针，浅拷贝
        self.ready_queue.push_back(task);
    }
    ///Remove the first task and return it,or `None` if `TaskManager` is empty
    // pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {          // 从队头中取出一个任务来执行
    //     self.ready_queue.pop_front()
    // }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>>{
        
        let mut smallest_index = 0;
        let mut smallest_value = self.ready_queue[0].inner_exclusive_access().stride;        
        for (index, value) in self.ready_queue.iter().enumerate() {
            if value.inner_exclusive_access().stride < smallest_value {
                smallest_value = value.inner_exclusive_access().stride;
                smallest_index = index;
            }
        }
        
        let ret = Some(self.ready_queue[smallest_index].clone());
        self.ready_queue.remove(smallest_index);
        
        ret
    }

}




lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}
///Interface offered to add task
pub fn add_task(task: Arc<TaskControlBlock>) {                      // 全局任务管理器中增加一个任务块
    TASK_MANAGER.exclusive_access().add(task);
}
///Interface offered to pop the first task
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {              
    TASK_MANAGER.exclusive_access().fetch()
}
