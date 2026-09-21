//! SPI Flash の Micron MT25QU128 を、SpiDevice の上で読み書きする。

use embedded_hal::spi::{Operation, SpiDevice};

const CMD_PAGE_PROGRAM: u8 = 0x02;
const CMD_READ: u8 = 0x03;
const CMD_READ_STATUS: u8 = 0x05;
const CMD_WRITE_ENABLE: u8 = 0x06;
const CMD_SUBSECTOR_ERASE_4KB: u8 = 0x20;
const STATUS_WRITE_IN_PROGRESS: u8 = 1 << 0;

/// 1 回の PAGE PROGRAM で書ける大きさ。書く範囲はこの境界をまたげない。
pub const PAGE_BYTES: usize = 256;

pub struct Mt25q<S> {
    spi: S,
}

/// 命令と 3 バイトのアドレス。128 Mbit の Flash は 3 バイトで全体を指せる。
fn command(op: u8, address: u32) -> [u8; 4] {
    let [_, high, middle, low] = address.to_be_bytes();
    [op, high, middle, low]
}

impl<S: SpiDevice> Mt25q<S> {
    pub fn new(spi: S) -> Self {
        Mt25q { spi }
    }

    pub fn read(&mut self, address: u32, buffer: &mut [u8]) -> Result<(), S::Error> {
        self.spi
            .transaction(&mut [Operation::Write(&command(CMD_READ, address)), Operation::Read(buffer)])
    }

    fn write_enable(&mut self) -> Result<(), S::Error> {
        self.spi.write(&[CMD_WRITE_ENABLE])
    }

    fn wait_ready(&mut self) -> Result<(), S::Error> {
        loop {
            let mut status = [0];
            self.spi
                .transaction(&mut [Operation::Write(&[CMD_READ_STATUS]), Operation::Read(&mut status)])?;
            if status[0] & STATUS_WRITE_IN_PROGRESS == 0 {
                return Ok(());
            }
        }
    }

    /// 4 KB の区画を消し、全てのバイトを 0xFF にする。
    pub fn erase_subsector(&mut self, address: u32) -> Result<(), S::Error> {
        self.write_enable()?;
        self.spi.write(&command(CMD_SUBSECTOR_ERASE_4KB, address))?;
        self.wait_ready()
    }

    pub fn program(&mut self, address: u32, data: &[u8]) -> Result<(), S::Error> {
        self.write_enable()?;
        self.spi
            .transaction(&mut [Operation::Write(&command(CMD_PAGE_PROGRAM, address)), Operation::Write(data)])?;
        self.wait_ready()
    }
}
