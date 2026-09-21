//! NEORV32 の SPI で、FPGA のコンフィグに使う SPI Flash の Micron MT25QU128 を読み書きする。

use core::ptr::{read_volatile, write_volatile};

const CTRL: *mut u32 = 0xFFF8_0000 as *mut u32;
const DATA: *mut u32 = 0xFFF8_0004 as *mut u32;

// CTRL のビット 0 は SPI を有効にし、ビット 3 から 5 がクロックの前置分周の選択になる。
// ビット 31 は、送受信の途中であることを示す。
const CTRL_EN: u32 = 1 << 0;
const CTRL_PRSC_LSB: u32 = 3;
const CTRL_BUSY: u32 = 1 << 31;

// SCK は、25 MHz を前置分周の 128 と 2 で割った約 98 kHz にする。
// この基板の Flash の線は、1 MHz では READ ID が半分ほど失敗し、100 kHz では失敗しなかった。
const PRSC_SELECT_128: u32 = 4;

// DATA のビット 31 を立てて書くと、チップセレクトの操作になる。
// ビット 3 が選ぶかどうかで、ビット 2 から 0 がチップセレクトの番号になる。
const DATA_CMD: u32 = 1 << 31;
const DATA_CS_ENABLE: u32 = 1 << 3;
/// Flash は、SPI の 0 番のチップセレクトにつながる。
const FLASH_CS: u32 = 0;

const CMD_PAGE_PROGRAM: u8 = 0x02;
const CMD_READ: u8 = 0x03;
const CMD_READ_STATUS: u8 = 0x05;
const CMD_WRITE_ENABLE: u8 = 0x06;
const CMD_SUBSECTOR_ERASE_4KB: u8 = 0x20;
const STATUS_WRITE_IN_PROGRESS: u8 = 1 << 0;

/// 1 回の PAGE PROGRAM で書ける大きさ。書く範囲はこの境界をまたげない。
pub const PAGE_BYTES: usize = 256;

pub fn init() {
    unsafe { write_volatile(CTRL, CTRL_EN | PRSC_SELECT_128 << CTRL_PRSC_LSB) };
}

fn wait_idle() {
    while unsafe { read_volatile(CTRL) } & CTRL_BUSY != 0 {}
}

fn select() {
    unsafe { write_volatile(DATA, DATA_CMD | DATA_CS_ENABLE | FLASH_CS) };
}

fn deselect() {
    unsafe { write_volatile(DATA, DATA_CMD) };
    wait_idle();
}

fn transfer(byte: u8) -> u8 {
    unsafe { write_volatile(DATA, byte as u32) };
    wait_idle();
    unsafe { read_volatile(DATA) as u8 }
}

/// 命令と 3 バイトのアドレスを送る。128 Mbit の Flash は 3 バイトで全体を指せる。
fn send_command(command: u8, address: u32) {
    transfer(command);
    for shift in [16, 8, 0] {
        transfer((address >> shift) as u8);
    }
}

pub fn read(address: u32, buffer: &mut [u8]) {
    select();
    send_command(CMD_READ, address);
    for byte in buffer.iter_mut() {
        *byte = transfer(0);
    }
    deselect();
}

fn write_enable() {
    select();
    transfer(CMD_WRITE_ENABLE);
    deselect();
}

fn wait_ready() {
    loop {
        select();
        transfer(CMD_READ_STATUS);
        let status = transfer(0);
        deselect();
        if status & STATUS_WRITE_IN_PROGRESS == 0 {
            return;
        }
    }
}

/// 4 KB の区画を消し、全てのバイトを 0xFF にする。
pub fn erase_subsector(address: u32) {
    write_enable();
    select();
    send_command(CMD_SUBSECTOR_ERASE_4KB, address);
    deselect();
    wait_ready();
}

pub fn program(address: u32, data: &[u8]) {
    write_enable();
    select();
    send_command(CMD_PAGE_PROGRAM, address);
    for &byte in data {
        transfer(byte);
    }
    deselect();
    wait_ready();
}
