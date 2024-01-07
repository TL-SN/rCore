//! batch subsystem

use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::*;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],        // 内核栈空间
};
static USER_STACK: UserStack = UserStack {      
    data: [0; USER_STACK_SIZE],         // 用户栈空间
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;    // 计算 core::mem::size_of::<TrapContext>()的大小
        unsafe {
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }                     // 返回值所栈指针，其指向了刚刚被压栈的TrapCOntext结构体
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {     // 获取栈顶
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

struct AppManager {
    num_app: usize,             //  app的数量
    current_app: usize,         // 字段表示当前执行的是第几个应用
    app_start: [usize; MAX_APP_NUM + 1],    // 所有app的起始地址
}

impl AppManager {
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {          // 0..self.num_app  =>  [0,self.num.app)
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }
    // 1、清零从 0x80400000 地址，大小为 APP_SIZE_LIMIT的内存
    // 2、从app_start中读取数据加载到以0x80400000 为开始的地址上
    // 3、刷新i-cache

    unsafe fn load_app(&self, app_id: usize) {                  // 加载app
                                                                // 1、如果要加载的app_id 号大于最大的app号，则直接shutdown，因为所有app已载入完毕
        if app_id >= self.num_app {
            println!("All applications completed!");
            shutdown(false);
        }
        println!("[kernel] Loading app_{}", app_id);
        // clear app area
        core::slice::from_raw_parts_mut(                        // 2、由于我们每次都把app加载到同一位置，因此我们每次load app的时候都要把APP_BASE_ADDRESS这个地址到其最大长度进行初始化()
            APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT
        ).fill(0);
        let app_src = core::slice::from_raw_parts(          // 3、从self.app_start 中导入app二进制文件
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        let app_dst = core::slice::from_raw_parts_mut(   
            APP_BASE_ADDRESS as *mut u8, app_src.len()
        );
        app_dst.copy_from_slice(app_src);                           // 4、把该app的数据导进 APP_BASE_ADDRESS 这个地址中

        asm!("fence.i");                                    //  刷新指令cache
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}


// 
lazy_static! {  // lazy_static! 宏提供了全局变量的运行时初始化功能/这里我们借助 lazy_static! 声明了一个 AppManager 结构的名为 APP_MANAGER 的全局实例，且只有在它第一次被使用到的时候，才会进行实际的初始化工作。
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();  // num_app来自link_app.S，link_ap.S中，；链接器把所有要运行的的app的二进制数据都连续的嵌入了.data数据段内，而_num_app 就是这些app数据的起始位置
            }
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();                                  // 这里就是读取_num_app 的前8字节，其前八字节记录了要装入的app的数量
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];         // app_start记录了所有app数据在.data段的起始位置
            let app_start_raw: &[usize] =                           
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);           // num_app_ptr.add(1) ，num_app_ptr 的地址＋1看似＋1，实则+8
                                                                                        // from_raw_parts函数其第二个参数代表长度，但不是字节长度，是元素长度，比如如果切片的类型所 *const i32类型，那么其切片长度的单位就是4字节
                                                                                        // 如果切片的类型所 *const i64，那么其切片长度的单位就是8字节
            app_start[..=num_app].copy_from_slice(app_start_raw);                       //// 函数结束之后，app_start_raw里记录了各个app的起始地址
            
                                                                                        // 记录一下..=num_app 的切片用法， ..=5 代表着下标从0到5切片
            AppManager {                                                        
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

/// init batch subsystem
pub fn init() {
    print_app_info();
}

/// print apps info
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// run next app
pub fn run_next_app() -> ! {            // 写着run_next，实际上是run当前下标的app
    let mut app_manager = APP_MANAGER.exclusive_access();  // 获取exclusive_access的读写权限
    let current_app = app_manager.get_current_app();   // 获取当前正在运行的app下标
    unsafe {
        app_manager.load_app(current_app);              // 1、把当前app加载到APP_BASE_ADDRESS 地址中
    }
    app_manager.move_to_next_app();                            // app 下标号加1
    drop(app_manager);                                          // 手动销毁app_manager


    // before this we have to drop local variables related to resources manually
    // and release the resources
    extern "C" {                                                // __restore 在trap.S中，下面代码的功能应该就是保存一下上下文然后 trap 返回 U模式
                                                                // 这个被定义在trap.S的全局汇编__restore带了一个参数，结合RISC-V的函数调用传参法则可以知道，第一个参数被放在了a0寄存器中，
                                                                // 这也是为什么__restore里第一条汇编指令就是 mv sp, a0 
        fn __restore(cx_addr: usize);             
    }
    unsafe {                                                                                // 2、trap从S模式切换到U模式
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(      // 在内核栈中压入一个trap上下文
            APP_BASE_ADDRESS,           // app运行地址
            USER_STACK.get_sp(),                // 用户栈地址
        )) as *const _ as usize);                                                           // 这里的作用就是把用户trap的上下文压入内核栈中，并在__restore汇编中(此时还在内核栈)把这些数据都退还给寄存器，
                                                                                            // 再之后就调用csrrw切换为用户栈并ret到U模式
                                                                                            // 这就相当于S模式在做trap回U模式前的准备
                                                                                            // 3、一旦切回到用户模式，用户就会直接在APP_BASE_ADDRESS地址上执行数据，这就达到了执行app的目的
    }
    panic!("Unreachable in batch::run_current_app!");
}
