//! MT25Q の命令の番号と、状態レジスタのビット。

pub const PAGE_PROGRAM: u8 = 0x02;
pub const READ: u8 = 0x03;
pub const READ_STATUS_REGISTER: u8 = 0x05;
pub const WRITE_ENABLE: u8 = 0x06;
pub const SUBSECTOR_ERASE_4KB: u8 = 0x20;

pub const STATUS_WRITE_IN_PROGRESS: u8 = 1 << 0;
