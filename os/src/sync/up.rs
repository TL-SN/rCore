// l2-1
use core::cell::{RefCell, RefMut};
pub struct UPSafeCell<T> {              ////UPSafeCell 对于 RefCell 简单进行封装，它和 RefCell 一样提供内部可变性和运行时借用检查，只是更加严格：
                                            //调用 exclusive_access 可以得到它包裹的数据的独占访问权
                                            //相比 RefCell 它不再允许多个读操作同时存在。
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}        

impl<T> UPSafeCell<T> {
    /// User is responsible to guarantee that inner struct is only used in
    /// uniprocessor.
    pub unsafe fn new(value: T) -> Self {
        Self { inner: RefCell::new(value) }
    }
    /// Panic if the data has been borrowed.
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()     
    }
}