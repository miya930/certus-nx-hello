#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};
use defmt_rtt as _;

// NEORV32 のメモリマップ。
const GPIO_PORT_OUT: *mut u32 = 0xFFFC_0004 as *mut u32;
const UART0_CTRL: *mut u32 = 0xFFF5_0000 as *mut u32;
const UART0_DATA: *mut u32 = 0xFFF5_0004 as *mut u32;
const CLINT_MTIME: *mut u32 = 0xFFF4_BFF8 as *mut u32;

// CTRL のビット 0 は UART を有効にし、ビット 6 から 15 には分周比から 1 を引いた値を置く。
// ビット 19 は、送信 FIFO に空きがあることを示す。
const UART_CTRL_EN: u32 = 1 << 0;
const UART_CTRL_BAUD_LSB: u32 = 6;
const UART_CTRL_TX_NFULL: u32 = 1 << 19;

const CLK_HZ: u32 = 25_000_000;
const UART_BAUD: u32 = 19_200;
const STEP_MSEC: u32 = 250;

// UART はクロックを 2 分周してから、この分周比で 1 ビットの長さを作る。
// 25 MHz で 19200 baud なら 651 で、分周比の 10 ビットに収まるため、それ以上の前置分周は使わない。
const UART_BAUD_DIV: u32 = CLK_HZ / (2 * UART_BAUD);

/// ブートローダを通らずに起動するため、UART は自分で設定する。
fn uart_init() {
    unsafe {
        write_volatile(
            UART0_CTRL,
            UART_CTRL_EN | (UART_BAUD_DIV - 1) << UART_CTRL_BAUD_LSB,
        )
    };
}

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

/// パニックした場所を defmt で送ってから止まる。
/// メッセージの整形には core::fmt が要り、16 KB の命令メモリを圧迫するため、場所だけを送る。
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        defmt::error!("panicked at {=str}:{=u32}", location.file(), location.line());
    }
    loop {}
}

#[riscv_rt::entry]
fn main() -> ! {
    uart_init();
    uart_puts("\nHello from Rust on NEORV32.\n");
    uart_puts("Counting up on the LEDs.\n");
    defmt::info!("Counting up on the LEDs every {=u32} ms.", STEP_MSEC);

    let mut value: u32 = 0;
    loop {
        unsafe { write_volatile(GPIO_PORT_OUT, value) };
        defmt::debug!("LED {=u32:08b}", value & 0xFF);
        wait_msec(STEP_MSEC);
        value = value.wrapping_add(1);
    }
}
