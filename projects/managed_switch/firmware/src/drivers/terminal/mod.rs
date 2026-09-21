//! FT2232H の Port B につながる UART0 の端末。
//! 文字、数、アドレス、表の見出しを端末に合わせた形で書き、届いた文字を 1 バイトずつ読む。

mod style;

pub use style::Style;

use neorv32_hal::uart::{Uart, UartRx, UartTx};

/// u64 の最大値の桁数。
const DECIMAL_DIGITS: usize = 20;

const MSEC_PER_SEC: u64 = 1000;
const SEC_PER_MIN: u64 = 60;
const MIN_PER_HOUR: u64 = 60;
const HOUR_PER_DAY: u64 = 24;

pub struct Terminal {
    tx: UartTx,
    rx: UartRx,
}

impl Terminal {
    pub fn new(uart: Uart) -> Self {
        let (tx, rx) = uart.split();
        Terminal { tx, rx }
    }

    /// 届いたバイトを 1 つ取り出す。届いていなければ None を返す。
    pub fn read_byte(&mut self) -> Option<u8> {
        self.rx.read_byte()
    }

    pub fn put(&mut self, byte: u8) {
        self.tx.write_byte(byte);
    }

    /// 端末は改行に CR と LF の両方を求めるため、LF の前に CR を足す。
    pub fn puts(&mut self, text: &str) {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.put(b'\r');
            }
            self.put(byte);
        }
    }

    /// 表の列をそろえるため、書いた文字数から幅までを空白で埋める。
    fn pad(&mut self, written: usize, width: usize) {
        for _ in written..width {
            self.put(b' ');
        }
    }

    pub fn puts_padded(&mut self, text: &str, width: usize) {
        self.puts(text);
        self.pad(text.len(), width);
    }

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

    pub fn put_dec(&mut self, value: impl Into<u64>) {
        self.put_dec_padded(value, 0);
    }

    pub fn put_dec_padded(&mut self, value: impl Into<u64>, width: usize) {
        let mut digits = [0; DECIMAL_DIGITS];
        let text = Self::decimal(value.into(), &mut digits);
        for &digit in text {
            self.put(digit);
        }
        self.pad(text.len(), width);
    }

    pub fn put_hex(&mut self, value: u32, digits: u32) {
        for shift in (0..digits).rev() {
            let nibble = (value >> (shift * 4)) & 0xF;
            self.put(b"0123456789ABCDEF"[nibble as usize]);
        }
    }

    /// これ以降に書く文字の見た目を変える。戻すときは Style::NORMAL を渡す。
    pub fn set_style(&mut self, style: Style) {
        self.puts(style.code());
    }

    pub fn puts_styled(&mut self, style: Style, text: &str) {
        self.set_style(style);
        self.puts(text);
        self.set_style(Style::NORMAL);
    }

    pub fn puts_styled_padded(&mut self, style: Style, text: &str, width: usize) {
        self.puts_styled(style, text);
        self.pad(text.len(), width);
    }

    /// 表の見出しを、列の幅に合わせて並べる。最後の列は幅を持たない。
    pub fn header(&mut self, columns: &[(&str, usize)]) {
        self.set_style(Style::HEADING);
        for &(title, width) in columns {
            self.puts_padded(title, width);
        }
        self.set_style(Style::NORMAL);
        self.puts("\n");
    }

    pub fn put_ip(&mut self, ip: &[u8; 4]) {
        for (index, &octet) in ip.iter().enumerate() {
            if index > 0 {
                self.put(b'.');
            }
            self.put_dec(octet);
        }
    }

    pub fn put_mac(&mut self, mac: &[u8; 6]) {
        for (index, &byte) in mac.iter().enumerate() {
            if index > 0 {
                self.put(b':');
            }
            self.put_hex(byte as u32, 2);
        }
    }

    /// 経過時間を、日、時、分、秒で出す。1 日に満たなければ日を省く。
    pub fn put_duration(&mut self, msec: u64) {
        let seconds = msec / MSEC_PER_SEC;
        let minutes = seconds / SEC_PER_MIN;
        let hours = minutes / MIN_PER_HOUR;
        let days = hours / HOUR_PER_DAY;
        if days > 0 {
            self.put_dec(days);
            self.puts("d ");
        }
        for (value, unit) in
            [(hours % HOUR_PER_DAY, "h "), (minutes % MIN_PER_HOUR, "m "), (seconds % SEC_PER_MIN, "s")]
        {
            if value < 10 {
                self.put(b'0');
            }
            self.put_dec(value);
            self.puts(unit);
        }
    }
}
