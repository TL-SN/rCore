mod inode;
mod pipe;
mod stdio;
mod mailbox;
use crate::mm::UserBuffer;

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}

pub use inode::{list_apps, open_file, OpenFlags};
pub use pipe::make_pipe;
pub use stdio::{Stdin, Stdout};
pub use mailbox::{Mail};