#![no_std]
#![no_main]

use defmt_rtt as _;
use embedded_hal::delay::DelayNs;
use embedded_io::Write;
use neorv32_hal::{gpio::Gpio, mtime::Mtime, pac, uart::Uart};

const CLK_HZ: u32 = 25_000_000;
const UART_BAUD: u32 = 19_200;
const STEP_MSEC: u32 = 250;

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
    let peripherals = pac::Peripherals::take().unwrap();
    let mut uart = Uart::new(peripherals.uart0, CLK_HZ, UART_BAUD);
    let mut gpio = Gpio::new(peripherals.gpio);
    let mut mtime = Mtime::new(peripherals.clint, CLK_HZ);

    let Ok(()) = uart.write_all(b"\r\nHello from Rust on NEORV32.\r\nCounting up on the LEDs.\r\n");
    defmt::info!("Counting up on the LEDs every {=u32} ms.", STEP_MSEC);

    let mut value: u32 = 0;
    loop {
        gpio.write(value);
        defmt::debug!("LED {=u32:08b}", value & 0xFF);
        mtime.delay_ms(STEP_MSEC);
        value = value.wrapping_add(1);
    }
}
