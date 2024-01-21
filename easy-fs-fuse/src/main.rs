use clap::{App,Arg};
use easy_fs::{BlockDevice, EasyFileSystem};
use std::fs::{read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::sync::Mutex;

const BLOCK_SZ: usize = 512;

// std::file::File 由 Rust 标准库 std 提供，可以访问 Linux 上的一个文件。我们将它包装成 BlockFile 类型来模拟一块磁盘，为它实现 BlockDevice 接口
struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {                                // read_block,读取某一块block
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))                // 1、seek定位
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");                         //2、read读取
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {                                // 1、seek定位
        let mut file = self.0.lock().unwrap();                      // 2、write写入
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
    // efs_test().expect("111");
}

// #[test]
fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
        .arg(
            Arg::with_name("source")
                .short("s")                                     // -s 指定用户的源代码目录
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")                                     // -t 指定保存应用的目录
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();         // 1、读取source路径,source就是用户的源码路径
    let target_path = matches.value_of("target").unwrap();      // 2、读取target路径，target路径就是用户生成的程序路径
    println!("----------------------------------------------------------------------------------");
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);    
    println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
    let block_file = Arc::new(BlockFile(Mutex::new({        // 3、创建 \\fs.img对象
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * 512).unwrap();                            //4、指定fs.img大小为16MIB
        f
    })));
                                                                              // 5、创建efs文件系统对象
    // 16MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file, 16 * 2048, 1);
                                                                                                
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));      // 6、获取根目录 inode
    let apps: Vec<_> = read_dir(src_path)                        // 7、获取source目录中的每个应用的源代码文件并去掉后缀名
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {                                       // 8、枚举 apps 中的每个应用，从放置应用执行程序的目录中找到对应应用的 ELF 文件
                                                                    //（这是一个 Linux 上的文件），并将数据读入内存。接着需要在 easy-fs 中创建一个同名文件并将 ELF 数据写
                                                                    // 入到这个文件中。这个过程相当于将 Linux 上的文件系统中的一个文件复制到我们的 easy-fs 中。
        // load app data from host file system
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();  // 8_1、这是把user app的全部数据都读入了
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let inode = root_inode.create(app.as_str()).unwrap();           // 8_2、在文件系统的根目录中创建file，并写入app数据
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    //list apps
    // for app in root_inode.ls() {
    //     println!("{}", app);
    // }
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({        // 第一步我们需要打开虚拟块设备。这里我们在 Linux 上创建文件 easy-fs-fuse/target/fs.img 来新建一个虚拟块设备，并将它的容量设置为 8192 个块即 4MiB 。在创建的时候需要将它的访问权限设置为可读可写。
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(block_file.clone(), 4096, 1);    // 1、创建文件系统
    let efs = EasyFileSystem::open(block_file.clone());           // 2、open文件系统
    let root_inode = EasyFileSystem::root_inode(&efs);                                               // 3、获取根目录的 Inode
    root_inode.create("filea");                                                                       // 4、创建文件 filea与fileb                                                                        
    root_inode.create("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }

    let inode = root_inode.find_inode_id_by_root("filea").unwrap();
    // println!("{:?}",inode);
    root_inode.delete_dir_enter_by_inode_and_name("filea",inode );
    
    for name in root_inode.ls() {
        println!("{}", name);
    }
    // let filea = root_inode.find("filea").unwrap();                                       // 5、在根目录下寻找文件 filea
    // let greet_str = "Hello, world!";
    // filea.write_at(0, greet_str.as_bytes());                                                   // 6、在文件开头写入"Hello World"
    // //let mut buffer = [0u8; 512];
    // let mut buffer = [0u8; 233];
    // let len = filea.read_at(0, &mut buffer);                                            // 7、在文件开头读取数据
    // assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);


    
    // let mut random_str_test = |len: usize| {
    //     filea.clear();
    //     assert_eq!(filea.read_at(0, &mut buffer), 0,);
    //     let mut str = String::new();
    //     use rand;
    //     // random digit
    //     for _ in 0..len {
    //         str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
    //     }
    //     filea.write_at(0, str.as_bytes());
    //     let mut read_buffer = [0u8; 127];
    //     let mut offset = 0usize;
    //     let mut read_str = String::new();
    //     loop {
    //         let len = filea.read_at(offset, &mut read_buffer);
    //         if len == 0 {
    //             break;
    //         }
    //         offset += len;
    //         read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
    //     }
    //     assert_eq!(str, read_str);
    // };

    // random_str_test(4 * BLOCK_SZ);
    // random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    // random_str_test(100 * BLOCK_SZ);
    // random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    // random_str_test((12 + 128) * BLOCK_SZ);
    // random_str_test(400 * BLOCK_SZ);
    // random_str_test(1000 * BLOCK_SZ);
    // random_str_test(2000 * BLOCK_SZ);

    Ok(())
}
