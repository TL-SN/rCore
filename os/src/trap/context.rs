use riscv::register::sstatus::{self, Sstatus, SPP};

/// Trap Context
#[repr(C)]
pub struct TrapContext {                // Trap 结构体
    /// general regs[0..31]             // 32个寄存器
    pub x: [usize; 32],
    /// CSR sstatus      
    pub sstatus: Sstatus,               // 当前所处的模式
    /// CSR sepc
    pub sepc: usize,                    // trap前的地址
}

impl TrapContext {
    /// set stack pointer to x_2 reg (sp)
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// init app context
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus   // 处于哪个特权级
        // use log::debug;
        // debug!("sstatus is : {:x}",sstatus.bits());
        sstatus.set_spp(SPP::User); //previous privilege mode: user mode  // 设置之前的特权模式 : user特权模式
        // debug!("sstatus is : {:x}",sstatus.bits());
        let mut cx = Self {
            x: [0; 32], 
            sstatus,
            sepc: entry, // entry point of app          // 记录 Trap 发生之前执行的最后一条指令的地址
        };
        cx.set_sp(sp); // app's user stack pointer
        cx // return initial Trap Context of app
    }
}
