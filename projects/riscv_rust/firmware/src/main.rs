#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};
use panic_halt as _;

// NEORV32 のメモリマップ。
const GPIO_PORT_OUT: *mut u32 = 0xFFFC_0004 as *mut u32;
const UART0_CTRL: *mut u32 = 0xFFF5_0000 as *mut u32;
const UART0_DATA: *mut u32 = 0xFFF5_0004 as *mut u32;
const CLINT_MTIME: *mut u32 = 0xFFF4_BFF8 as *mut u32;

// CTRL のビット 19 は、送信 FIFO に空きがあることを示す。
const UART_CTRL_TX_NFULL: u32 = 1 << 19;

const CLK_HZ: u32 = 25_000_000;
const STEP_MSEC: u32 = 250;

/// ブートローダが設定した速度をそのまま使うため、CTRL は書き換えない。
fn uart_put(byte: u8) {
    while unsafe { read_volatile(UART0_CTRL) } & UART_CTRL_TX_NFULL == 0 {}
    unsafe { write_volatile(UART0_DATA, byte as u32) };
}

fn uart_puts(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            uart_put(b'\r');
        }
        uart_put(byte);
    }
}

/// マシンタイマはシステムクロックで進むため、待ち時間をクロック周波数から求める。
/// 下位 32 ビットだけを見ると 171 秒で一周するが、差を取るので待ち時間が短ければ問題ない。
fn wait_msec(msec: u32) {
    let ticks = CLK_HZ / 1000 * msec;
    let start = unsafe { read_volatile(CLINT_MTIME) };
    while unsafe { read_volatile(CLINT_MTIME) }.wrapping_sub(start) < ticks {}
}

#[riscv_rt::entry]
fn main() -> ! {
    uart_puts("\nHello from Rust on NEORV32.\n");
    uart_puts("Counting up on the LEDs.\n");

    let mut value: u32 = 0;
    loop {
        unsafe { write_volatile(GPIO_PORT_OUT, value) };
        wait_msec(STEP_MSEC);
        value = value.wrapping_add(1);
    }
}
