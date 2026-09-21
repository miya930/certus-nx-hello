//! NEORV32 の UART0 を、FT2232H の Port B につながるコンソールとして使う。

use core::ptr::{read_volatile, write_volatile};

const CTRL: *mut u32 = 0xFFF5_0000 as *mut u32;
const DATA: *mut u32 = 0xFFF5_0004 as *mut u32;

// CTRL のビット 0 は UART を有効にし、ビット 6 から 15 には分周比から 1 を引いた値を置く。
// ビット 16 は受信 FIFO にデータがあること、ビット 19 は送信 FIFO に空きがあることを示す。
const CTRL_EN: u32 = 1 << 0;
const CTRL_BAUD_LSB: u32 = 6;
const CTRL_RX_NEMPTY: u32 = 1 << 16;
const CTRL_TX_NFULL: u32 = 1 << 19;

/// UART はクロックを 2 分周してから、この分周比で 1 ビットの長さを作る。
/// 分周比は 10 ビットに収まる範囲で使い、それ以上の前置分周は使わない。
pub fn init(clk_hz: u32, baud: u32) {
    let divider = clk_hz / (2 * baud);
    unsafe { write_volatile(CTRL, CTRL_EN | (divider - 1) << CTRL_BAUD_LSB) };
}

pub fn put(byte: u8) {
    while unsafe { read_volatile(CTRL) } & CTRL_TX_NFULL == 0 {}
    unsafe { write_volatile(DATA, byte as u32) };
}

pub fn get() -> Option<u8> {
    if unsafe { read_volatile(CTRL) } & CTRL_RX_NEMPTY == 0 {
        return None;
    }
    Some(unsafe { read_volatile(DATA) } as u8)
}

/// 端末は改行に CR と LF の両方を求めるため、LF の前に CR を足す。
pub fn puts(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            put(b'\r');
        }
        put(byte);
    }
}

/// 表の列をそろえるため、文字列の後ろを空白で埋める。
pub fn puts_padded(text: &str, width: usize) {
    puts(text);
    for _ in text.len()..width {
        put(b' ');
    }
}

/// u64 の最大値の桁数。
const DECIMAL_DIGITS: usize = 20;

/// 10 進の桁を、上の桁から並べて返す。
fn decimal(mut value: u64, digits: &mut [u8; DECIMAL_DIGITS]) -> &[u8] {
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            return &digits[start..];
        }
    }
}

pub fn put_dec(value: impl Into<u64>) {
    let mut digits = [0; DECIMAL_DIGITS];
    for &digit in decimal(value.into(), &mut digits) {
        put(digit);
    }
}

/// 表の列をそろえるため、数の後ろを空白で埋める。
pub fn put_dec_padded(value: impl Into<u64>, width: usize) {
    let mut digits = [0; DECIMAL_DIGITS];
    let text = decimal(value.into(), &mut digits);
    for &digit in text {
        put(digit);
    }
    for _ in text.len()..width {
        put(b' ');
    }
}

pub fn put_hex(value: u32, digits: u32) {
    for shift in (0..digits).rev() {
        let nibble = (value >> (shift * 4)) & 0xF;
        put(b"0123456789ABCDEF"[nibble as usize]);
    }
}
