use crate::task::{SignalFlags, MAX_SIG};

/// Action for a signal
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,                              // 有一个进程的全局mask，还有一个SignalAction mask，前者代表进程所不允许执行的signal，后者代表本signal
                                                        // 所不允许执行的signal，即局部mask
}

impl Default for SignalAction {
    fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::from_bits(40).unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct SignalActions {
    pub table: [SignalAction; MAX_SIG + 1],
}

impl Default for SignalActions {
    fn default() -> Self {
        Self {
            table: [SignalAction::default(); MAX_SIG + 1],          // 每一项都记录进程如何响应对应的信号(因为可能有多个signal，所以都存起来)
        }
    }
}
