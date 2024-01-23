use super::File;
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::sync::{Arc, Weak};

use crate::task::suspend_current_and_run_next;

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
    pub fn read_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {      // 创建读端 
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }
    pub fn write_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {     // 创建写端
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

const RING_BUFFER_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq,Debug)]
enum RingBufferStatus {
    Full,                       // FULL 表示缓冲区已满不能再继续写入
    Empty,                      // EMPTY 表示缓冲区为空无法从里面读取
    Normal,                     // 而 NORMAL 则表示除了 FULL 和 EMPTY 之外的其他状态。
}

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],    // arr、head、tail维护一个循环队列
    head: usize,
    tail: usize,
    status: RingBufferStatus,       // RingBufferStatus 记录了缓冲区目前的状态
    write_end: Option<Weak<Pipe>>,  // 确认该管道所有的写端是否都已经被关闭了
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }
    pub fn write_byte(&mut self, byte: u8) {            // 向循环队列中写入一字节，并控制写指针移动
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }
    pub fn read_byte(&mut self) -> u8 {             // 在循环队列中读取一字节，并控制读指针移动
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            // println!("here~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
            self.status = RingBufferStatus::Empty;
        }
        c
    }
    // pipetest
    pub fn available_read(&self) -> usize {                 // 可以计算管道中还有多少个字符可以读取


        // println!("read_self.status: {:?}",self.status);
        if self.status == RingBufferStatus::Empty {
            // println!("is empty?");
            0
        } else if self.tail > self.head {
            // println!("is tail > head?");
            self.tail - self.head
            
        } else {
            // println!("is tail + RING_BUFFER_SIZE - head?");
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }
    pub fn available_write(&self) -> usize {
        // println!("write_self.status: {:?}",self.status);
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()            // 可以判断管道的所有写端是否都被关闭了
        // 这是通过尝试将管道中保存的写端的弱引用计数升级为强引用计数来实现的。如果升级失败的话，说明管道写端的强引用计数为 0 ，
        // 也就意味着管道所有写端都被关闭了，从而管道中的数据不会再得到补充，待管道中仅剩的数据被读取完毕之后，管道就可以被销毁了。
    }
}

/// Return (read_end, write_end)
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {                      // 创建一个管道并返回它的读端和写端：
    let buffer = Arc::new(unsafe { UPSafeCell::new(PipeRingBuffer::new()) });
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));            // 浅拷贝！！！
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);
    (read_end, write_end)
}

impl File for Pipe {                                // 为pipe实现串口，目前已经为文件（OSInode）、输入流(Stdin/Stdout)实现了串口
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, buf: UserBuffer) -> usize {
        assert!(self.readable());
        let t = self.buffer.exclusive_access();
        
        println!("read: {:?}",t.status);
        drop(t);
        let want_to_read = buf.len();               // 这个是buf的长度，不是真要读的
        let mut buf_iter = buf.into_iter();             // 将传入的应用缓冲区 buf 转化为一个能够逐字节对于缓冲区进行访问的迭代器,每次调用 buf_iter.next() 即可按顺序取出用于访问缓冲区中一个字节的裸指针。
        let mut already_read = 0usize;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let loop_read = ring_buffer.available_read();           // 1、计算有多少可以读入的
            // println!("read: => {} <=",loop_read);
            if loop_read == 0 {
                if ring_buffer.all_write_ends_closed() {                   // 2、如果写入端/读入端没有关闭的话，说明还有数据等待填充
                    return already_read;    
                }
                drop(ring_buffer);
                suspend_current_and_run_next();                             // 我们先挂起一会等待填充
                continue;
            }
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();                // 3、循环读入，读完之后设置 RingBufferStatus::Empty;
                    }
                    already_read += 1;
                    // println!("{:?} <==> {:?}",already_read,want_to_read);
                    if already_read == want_to_read {
                        return want_to_read;
                    }
                } else {
                    return already_read;
                }
            }
        }
    } // pipetest
    fn write(&self, buf: UserBuffer) -> usize {
        assert!(self.writable());
        let want_to_write = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_write = 0usize;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let loop_write = ring_buffer.available_write();
            // println!("write: => {} <=",loop_write);
            if loop_write == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            // write at most loop_write bytes
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    // println!("111");
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    // println!("already_write: {} want_to_write: {}",already_write,want_to_write);
                    already_write += 1;
                    if already_write == want_to_write {
                        // println!("1111");
                        // ring_buffer.status = RingBufferStatus::Full;
                        // println!("{:?}",ring_buffer.status);
                        return want_to_write;
                    }
                } else {
                    return already_write;
                }
            }
        }
    }
}
