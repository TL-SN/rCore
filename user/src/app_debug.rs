#![feature(asm)]

use core::arch::asm;

pub fn debug() {
    
    let reg_x0: u64;
    unsafe {
        asm!("mv {}, x0", out(reg) reg_x0);
    }
    println!("Register x0: {}", reg_x0);
    

    let reg_x1: u64;
    unsafe {
        asm!("mv {}, x1", out(reg) reg_x1);
    }
    println!("Register x1: {}", reg_x1);
    

    let reg_x2: u64;
    unsafe {
        asm!("mv {}, x2", out(reg) reg_x2);
    }
    println!("Register x2: {}", reg_x2);
    

    let reg_x3: u64;
    unsafe {
        asm!("mv {}, x3", out(reg) reg_x3);
    }
    println!("Register x3: {}", reg_x3);
    

    let reg_x4: u64;
    unsafe {
        asm!("mv {}, x4", out(reg) reg_x4);
    }
    println!("Register x4: {}", reg_x4);
    

    let reg_x5: u64;
    unsafe {
        asm!("mv {}, x5", out(reg) reg_x5);
    }
    println!("Register x5: {}", reg_x5);
    

    let reg_x6: u64;
    unsafe {
        asm!("mv {}, x6", out(reg) reg_x6);
    }
    println!("Register x6: {}", reg_x6);
    

    let reg_x7: u64;
    unsafe {
        asm!("mv {}, x7", out(reg) reg_x7);
    }
    println!("Register x7: {}", reg_x7);
    

    let reg_x8: u64;
    unsafe {
        asm!("mv {}, x8", out(reg) reg_x8);
    }
    println!("Register x8: {}", reg_x8);
    

    let reg_x9: u64;
    unsafe {
        asm!("mv {}, x9", out(reg) reg_x9);
    }
    println!("Register x9: {}", reg_x9);
    

    let reg_x10: u64;
    unsafe {
        asm!("mv {}, x10", out(reg) reg_x10);
    }
    println!("Register x10: {}", reg_x10);
    

    let reg_x11: u64;
    unsafe {
        asm!("mv {}, x11", out(reg) reg_x11);
    }
    println!("Register x11: {}", reg_x11);
    

    let reg_x12: u64;
    unsafe {
        asm!("mv {}, x12", out(reg) reg_x12);
    }
    println!("Register x12: {}", reg_x12);
    

    let reg_x13: u64;
    unsafe {
        asm!("mv {}, x13", out(reg) reg_x13);
    }
    println!("Register x13: {}", reg_x13);
    

    let reg_x14: u64;
    unsafe {
        asm!("mv {}, x14", out(reg) reg_x14);
    }
    println!("Register x14: {}", reg_x14);
    

    let reg_x15: u64;
    unsafe {
        asm!("mv {}, x15", out(reg) reg_x15);
    }
    println!("Register x15: {}", reg_x15);
    

    let reg_x16: u64;
    unsafe {
        asm!("mv {}, x16", out(reg) reg_x16);
    }
    println!("Register x16: {}", reg_x16);
    

    let reg_x17: u64;
    unsafe {
        asm!("mv {}, x17", out(reg) reg_x17);
    }
    println!("Register x17: {}", reg_x17);
    

    let reg_x18: u64;
    unsafe {
        asm!("mv {}, x18", out(reg) reg_x18);
    }
    println!("Register x18: {}", reg_x18);
    

    let reg_x19: u64;
    unsafe {
        asm!("mv {}, x19", out(reg) reg_x19);
    }
    println!("Register x19: {}", reg_x19);
    

    let reg_x20: u64;
    unsafe {
        asm!("mv {}, x20", out(reg) reg_x20);
    }
    println!("Register x20: {}", reg_x20);
    

    let reg_x21: u64;
    unsafe {
        asm!("mv {}, x21", out(reg) reg_x21);
    }
    println!("Register x21: {}", reg_x21);
    

    let reg_x22: u64;
    unsafe {
        asm!("mv {}, x22", out(reg) reg_x22);
    }
    println!("Register x22: {}", reg_x22);
    

    let reg_x23: u64;
    unsafe {
        asm!("mv {}, x23", out(reg) reg_x23);
    }
    println!("Register x23: {}", reg_x23);
    

    let reg_x24: u64;
    unsafe {
        asm!("mv {}, x24", out(reg) reg_x24);
    }
    println!("Register x24: {}", reg_x24);
    

    let reg_x25: u64;
    unsafe {
        asm!("mv {}, x25", out(reg) reg_x25);
    }
    println!("Register x25: {}", reg_x25);
    

    let reg_x26: u64;
    unsafe {
        asm!("mv {}, x26", out(reg) reg_x26);
    }
    println!("Register x26: {}", reg_x26);
    

    let reg_x27: u64;
    unsafe {
        asm!("mv {}, x27", out(reg) reg_x27);
    }
    println!("Register x27: {}", reg_x27);
    

    let reg_x28: u64;
    unsafe {
        asm!("mv {}, x28", out(reg) reg_x28);
    }
    println!("Register x28: {}", reg_x28);
    

    let reg_x29: u64;
    unsafe {
        asm!("mv {}, x29", out(reg) reg_x29);
    }
    println!("Register x29: {}", reg_x29);
    

    let reg_x30: u64;
    unsafe {
        asm!("mv {}, x30", out(reg) reg_x30);
    }
    println!("Register x30: {}", reg_x30);
    

    let reg_x31: u64;
    unsafe {
        asm!("mv {}, x31", out(reg) reg_x31);
    }
    println!("Register x31: {}", reg_x31);
    

}
