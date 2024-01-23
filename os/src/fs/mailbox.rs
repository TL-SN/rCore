pub const MAX_BUFF_SIZE: usize = 256;
pub const MAX_MAIL_NUM: usize = 16;

#[derive(Debug)]
pub struct Mail{
    head:usize,
    tail:usize,
    message: [MessBuff; MAX_MAIL_NUM],               // 维护一长度为16的循环队列
}


#[derive(Clone, Copy,Debug)]
pub struct MessBuff{
    arr:[u8;MAX_BUFF_SIZE],
    len:usize,              // 记录缓冲区的长度
}

impl MessBuff{
    pub fn new()->Self{
        Self{
            arr: [0;MAX_BUFF_SIZE],
            len : 0,                   
        }
    }

}


impl Mail{
    pub fn new()->Self{
        Self{
            head : 0,
            tail : 0,
            message : [MessBuff::new();MAX_MAIL_NUM],
        }
    }

    pub fn write_mail(&mut self,buf : usize,len:usize) -> isize{
        // 1、判断循环队列是否已满
        if (self.tail + 1) % MAX_MAIL_NUM == self.head{
            return -1;
        }
        // 2、判断长度
        if len ==0{
            return 0;
        }

        if len > 256{
            return -1;
        }
        
        // 3、插入
        for i in 0..len{
            
            let x;
            // println!("x: {:x}",buf+i);
            unsafe{x = ((buf+i) as *const u8).read_volatile()}

            self.message[self.tail].arr[i] = x;
        }
        self.message[self.tail].len = len;
        // 4、入队
        self.tail = (self.tail + 1) % MAX_MAIL_NUM;

        
        len as isize
    }


    pub fn read_mail(&mut self,buf : usize) -> isize{
        // 1、判空
        if self.tail == self.head{
            return -1
        }
        let idx = self.head;                        // 读取的话，肯定从队头开始读，因为FIFO
        let len = self.message[idx].len;
        // println!("\n\n=> {:?} <=\n\n",idx);
        // 2、读取
        for i in 0..len as i32{
            let x = self.message[idx].arr[i as usize];
            // println!("{}",x);
            unsafe{((buf+i as usize) as *mut u8).write_volatile(x) }
        }
        
        // 3、出队
        self.head  =(self.head + 1) % MAX_MAIL_NUM;
        len as isize
    }
}