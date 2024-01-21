//! File system in os
mod inode;
mod stdio;
mod stat;
use crate::mm::UserBuffer;

use alloc::sync::Arc;
use easy_fs::{Inode};
pub use inode::{list_apps, open_file, OSInode, OpenFlags,ROOT_INODE};
pub use stdio::{Stdin, Stdout};
pub use stat::*;
/// File trait
pub trait File: Send + Sync {
    /// If readable
    fn readable(&self) -> bool;
    /// If writable
    fn writable(&self) -> bool;
    /// Read file to `UserBuffer`
    fn read(&self, buf: UserBuffer) -> usize;
    /// Write `UserBuffer` to file
    fn write(&self, buf: UserBuffer) -> usize;

    /// get OSInode
    fn get_osi(&self) -> Arc<Inode>;
    
    /// get nlink
    fn get_nlink(&self) ->usize;
}

