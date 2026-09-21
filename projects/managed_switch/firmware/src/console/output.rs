//! 端末への文字と数の出力。どのモジュールからも出せるように、共有した UART に書く。

use super::with_uart;
use embedded_io::Write;

pub fn put(byte: u8) {
    with_uart(|uart| {
        let Ok(()) = uart.write_all(&[byte]);
    });
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
