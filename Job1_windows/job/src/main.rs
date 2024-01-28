use std::{thread,time};
use rand::Rng;
use std::sync::{Arc, Mutex, Condvar};
use crossbeam_queue::SegQueue;
use once_cell::sync::Lazy;

const BIGPOINTS : usize = 30;
const MIDPOINTS : usize = 20;
const LITTLEPOINTS : usize  = 10;
const TIEGAMEPOINTS : usize = 0;
struct Lombard{
    p : usize,              // 选手个数(筷子数)
    m : usize,              // 评测机线程 (生产者,线程个数) , 给出随机的评测结果（两位不同选手的编号以及胜负结果，结果可能为平局）
    n : usize,              // worker线程(消费者，哲学家数量), 获取结果队列并更新数据库（全局变量等共享数据）记录的分数
    k : usize,              // 每个线程执行的对局数目
}

static GLOBAL_MUTEX_LOCK1: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));     // 全局互斥锁1
static GLOBAL_MUTEX_LOCK2: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));     // 全局互斥锁2
static GLOBAL_COUNTER: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));        // 全局互斥计数器
#[derive(Debug)]
struct Player{
    is_using: bool,
    point : isize,
}

struct Competitors{
    player1: usize,
    player2: usize,
}

#[derive(Debug)]
struct Score{
    score: Vec<Mutex<Player>>,
    condvars: Vec<Condvar>,
    competitors: SegQueue<Competitors>,         // SegQueue本身就支持高并发
}

impl Score {
    fn new(people:usize) -> Score{
        Score { 
            score: (0..people).map(|_| Mutex::new(Player{is_using: false,point: 1000})).collect(),
            condvars: (0..people).map(|_| Condvar::new()).collect(),
            competitors: SegQueue::new(),
        }
    }

    fn get_point(&self){
        let mut sum = 0;
        for i in 0..self.score.len(){
            let sc = self.score[i].lock().unwrap();
            sum += sc.point;
            println!("Player[{:?}] score is {:?}",i,sc.point);
        }
        println!("All_points : {:?}",sum);
    }

    // 一次失败的尝试
    // fn join_in(&self,player1:usize,player2:usize){
    //     let left = player1;
    //     let right = player2;
        
    //     {
    //         let mut left_lock = self.score[left].lock().unwrap();
    //         while left_lock.is_using{                                       // 错误原因1: 它在 while 循环中用于条件判断时，会发生所有权移动的问题。Rust 的 MutexGuard 是一个 RAII（Resource Acquisition Is Initialization）守卫，它在作用域结束时自动释放锁。逆天错误
    //             left_lock = self.condvars[left].wait(left_lock).unwrap();
    //         }
    //         left_lock.is_using = true;
    //     }
        
    //     {
    //         let mut right_lock = self.score[right].lock().unwrap();
    //         while right_lock.is_using {
    //             right_lock = self.condvars[right].wait(right_lock).unwrap();
    //         }
    //         right_lock.is_using = true;
            
    //     }

    //     self.competitors.push( Competitors{player1: left,player2:right} );  // 
    //     thread::sleep(time::Duration::from_millis(20));  // 比赛                                                                         // V(player1)
    //                                                                                         // V(player2)
    // }                                                                                       // drop
    

    // 上面的方法任然会导致死锁，测试了2000次，死锁了13次
    // 原因如下:
    // 虽然我们把锁放到了作用域里面，进行了及时的销毁，但是这里不同与record_points 函数，依然会出现交叉占用，形成互锁
    // 考虑这种情况:
    // 线程1: join_in(10,20); 
    // 线程2: join_in(20,10);
    // 线程1先锁住10号，并准备去锁20号
    // 线程2同时锁住了20号，准备去锁10号
    // 这时线程1、2会陷入while循环中，不断让出CPU空间,无法再前进一步，这就由陷入了哲学家进餐问题的经典错误之中，我们可以用两种方法来解决:
    // 1、永远都是先锁号小的，再锁号大的
    // 2、添加一个全局互斥锁，参考2023年王道操作系统 P103页。
    
    // 第一个解决方法
    // fn join_in(&self, player1: usize, player2: usize) {
    //     let (first, second) = if player1 < player2 { (player1, player2) } else { (player2, player1) };
        
    //     let mut first_lock = self.score[first].lock().unwrap();
    //     while first_lock.is_using {
    //         first_lock = self.condvars[first].wait(first_lock).unwrap();
    //     }
    //     first_lock.is_using = true;
    
    //     let mut second_lock = self.score[second].lock().unwrap();
    //     while second_lock.is_using {
    //         second_lock = self.condvars[second].wait(second_lock).unwrap();
    //     }
    //     second_lock.is_using = true;
    
    //     self.competitors.push(Competitors { player1: first, player2: second });
    //     thread::sleep(time::Duration::from_millis(20));
    // }

    // 第二个解决方法
    fn join_in(&self,player1:usize,player2:usize){
        let left = player1;
        let right = player2;
        
        let _lock = GLOBAL_MUTEX_LOCK2.lock().unwrap();
        {
            let mut left_lock = self.score[left].lock().unwrap();
            while left_lock.is_using{                                       // 错误原因1: 它在 while 循环中用于条件判断时，会发生所有权移动的问题。Rust 的 MutexGuard 是一个 RAII（Resource Acquisition Is Initialization）守卫，它在作用域结束时自动释放锁。
                left_lock = self.condvars[left].wait(left_lock).unwrap();
            }
            left_lock.is_using = true;
        }
        
        {
            let mut right_lock = self.score[right].lock().unwrap();
            while right_lock.is_using {
                right_lock = self.condvars[right].wait(right_lock).unwrap();
            }
            right_lock.is_using = true;
            
        }
        drop(_lock);
        self.competitors.push( Competitors{player1: left,player2:right} );  // 
        thread::sleep(time::Duration::from_millis(20));  // 比赛                                                                         // V(player1)
                                                                                            // V(player2)
    } 
                                                                               
    

    // 这里我们通过作用域的方式，把mutex锁进行了及时的销毁，避免出现交叉占用锁的情况
    fn record_points(&self,player1:usize,player2:usize){
        let left = player1;
        let right = player2;        
        
        let mut score1 ;
        let mut score2 ;
        {
            let left_lock = self.score[left].lock().unwrap();   // P(player1)
            score1 = left_lock.point;
        }
        

        {
            let right_lock = self.score[right].lock().unwrap(); // P(player2)
            score2 = right_lock.point;   
        }
        

        let points = get_point(score1,score2);
        score1 += points.0;
        score2 += points.1;
        
        
        {
            let mut left_lock = self.score[left].lock().unwrap();   // P(player1)
            left_lock.is_using = false;
            left_lock.point = score1;
        }
        
        {
            let mut right_lock = self.score[right].lock().unwrap(); // P(player2)
            right_lock.is_using = false;
            right_lock.point = score2;
        }

        
        

        self.condvars[left].notify_all();
        self.condvars[right].notify_all(); 
        
        let mut count = GLOBAL_COUNTER.lock().unwrap();
        *count += 1;

        // println!("pop {:?}",count);         // 正常情况下，最后count是Lombard.m * Lombard.k的大小
        if *count == 200{
            // self.get_point();
            println!("pop {:?}",count);
        }
    }
}


fn start_contest(contest : Arc<Score>,times:usize,number : usize){
    
    for _ in 0..times{
        let num = get_two_random_nums(number);
        let player1 = num.0 ;
        let player2 = num.1 ;

        contest.join_in(player1, player2);
    }
    
}

fn end_contest(contest : Arc<Score>,expect_times:usize){
    
    while true{
        // 加锁 
        let _lock = GLOBAL_MUTEX_LOCK1.lock().unwrap();
        if contest.competitors.len() != 0{
            
            // drop(_lock)                                         // 错因: 得等到pop之后再drop
            let players = contest.competitors.pop().unwrap();
            let left = players.player1;
            let right = players.player2;
            drop(_lock);                                        
            contest.record_points(left, right);
        }else{
            
            drop(_lock);  

            let _count = GLOBAL_COUNTER.lock().unwrap();
            if *_count >= expect_times{
                drop(_count);
                break;
            }
            drop(_count);
            thread::sleep(time::Duration::from_millis(20));
        
        }
    }

}


fn get_two_random_nums(number : usize) -> (usize, usize){
    let mut rng = rand::thread_rng();
    let num1 = rng.gen_range(0..number);
    let mut num2:usize;
    loop {
        num2 =  rng.gen_range(0..number);
        if num1 != num2 {
            break;
        }
    }
    return (num1,num2)
}

fn get_winer() -> usize{
    let mut rng = rand::thread_rng();
    let fin = rng.gen_range(0..3);
    fin as usize
}
fn get_point(score1 :isize,score2:isize) ->(isize,isize) {
// 0代表 平局,1代表player1胜利，2代表player2胜利 
    let win = get_winer();
    if score1 == score2 && win == 0{                            // 1_1、平局且分数相等
        return (TIEGAMEPOINTS as isize,TIEGAMEPOINTS as isize);
    }else if score1 > score2 && win == 0  {                     // 1_2、平局但第一个玩家的分数基数大
        return (LITTLEPOINTS as isize,-(LITTLEPOINTS as isize));
    }else if score1 < score2 && win == 0 {
        return (-(LITTLEPOINTS as isize),LITTLEPOINTS as isize); // // 1_3、平局但第一个玩家的分数基数大
    }

    if score1 >= score2 && win == 1{
        return (MIDPOINTS as isize,-(MIDPOINTS as isize));
    }else if score1 >= score2 && win == 2  {
        return (-(BIGPOINTS as isize),BIGPOINTS as isize);
    }

    if score1 <= score2 && win == 1{
        return (BIGPOINTS as isize,-(BIGPOINTS as isize));
    }else if score1 <= score2 && win == 2 {
        return (-(MIDPOINTS as isize),MIDPOINTS as isize);
    }
    
    (-1,-1)
}

fn main(){
    let saiblo  = Lombard{
        p : 100,
        m : 20,
        n : 100,
        k : 10,
    };
    

    let contests = Arc::new(Score::new(saiblo.p));
    

    let mut cp = Vec::new();
    for _ in 0..saiblo.m{
        let contest = Arc::clone(&contests);
        cp.push(thread::spawn(move ||{
            
            start_contest(contest,saiblo.k,saiblo.p);

        }));
    }

    let mut workers = Vec::new();
    let expect_times = saiblo.m * saiblo.k;
    for _ in 0..saiblo.n{
        let contest = Arc::clone(&contests);
        workers.push(thread::spawn(move ||{
            end_contest(contest,expect_times);
        }));
    }


    for thd in cp{
        thd.join().unwrap();
    }
    
    for wk in workers{
        wk.join().unwrap();
    }
    
    
}