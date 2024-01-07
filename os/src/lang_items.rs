use crate::sbi::shutdown;
use core::panic::PanicInfo;
use crate::stack_trace::print_stack_trace;




// 10,处理致命错误
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe { print_stack_trace(); }
    if let Some(location) = info.location() {
        println!(
            "|- Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Panicked: {}", info.message().unwrap());
    }
    
    shutdown(true)
}