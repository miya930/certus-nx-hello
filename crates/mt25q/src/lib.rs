//! SPI Flash の Micron MT25QU128 を、SpiDevice の上で読み書きする。

#![no_std]

mod commands;

use commands::*;
use embedded_hal::spi::{Operation, SpiDevice};

/// MT25QU128 は 128 Mbit ある。
pub const CAPACITY_BYTES: u32 = 128 * 1024 * 1024 / 8;
/// SUBSECTOR ERASE で消せる最小の区画の大きさ。
pub const SUBSECTOR_BYTES: u32 = 4 * 1024;
/// 1 回の PAGE PROGRAM で書ける大きさ。書く範囲はこの境界をまたげない。
pub const PAGE_BYTES: usize = 256;

pub struct Mt25q<S> {
    spi: S,
}

impl<S: SpiDevice> Mt25q<S> {
    pub fn new(spi: S) -> Self {
        Mt25q { spi }
    }

    /// 命令と 3 バイトのアドレス。128 Mbit の Flash は 3 バイトで全体を指せる。
    fn command(op: u8, address: u32) -> [u8; 4] {
        let [_, high, middle, low] = address.to_be_bytes();
        [op, high, middle, low]
    }

    pub fn read(&mut self, address: u32, buffer: &mut [u8]) -> Result<(), S::Error> {
        self.spi.transaction(&mut [Operation::Write(&Self::command(READ, address)), Operation::Read(buffer)])
    }

    fn write_enable(&mut self) -> Result<(), S::Error> {
        self.spi.write(&[WRITE_ENABLE])
    }

    fn wait_ready(&mut self) -> Result<(), S::Error> {
        loop {
            let mut status = [0];
            self.spi.transaction(&mut [Operation::Write(&[READ_STATUS_REGISTER]), Operation::Read(&mut status)])?;
            if status[0] & STATUS_WRITE_IN_PROGRESS == 0 {
                return Ok(());
            }
        }
    }

    /// 4 KB の区画を消し、全てのバイトを 0xFF にする。
    pub fn erase_subsector(&mut self, address: u32) -> Result<(), S::Error> {
        self.write_enable()?;
        self.spi.write(&Self::command(SUBSECTOR_ERASE_4KB, address))?;
        self.wait_ready()
    }

    pub fn program(&mut self, address: u32, data: &[u8]) -> Result<(), S::Error> {
        self.write_enable()?;
        self.spi.transaction(&mut [Operation::Write(&Self::command(PAGE_PROGRAM, address)), Operation::Write(data)])?;
        self.wait_ready()
    }
}
