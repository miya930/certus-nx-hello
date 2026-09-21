#![no_std]
#![no_main]

mod config;
mod console;
mod dp83867;
mod host;
mod leds;
mod links;
mod memory_map;
mod mt25q;
mod ports;
mod satcat5;
mod settings;
mod ticker;
mod traffic;

use defmt_rtt as _;
use neorv32_hal::{gpio::Gpio, mtime::Mtime, pac, spi::Spi, uart::Uart};
use smoltcp::iface::SocketStorage;

use config::Config;
use console::{Console, Style};
use dp83867::Dp83867;
use host::Host;
use leds::Leds;
use links::Links;
use memory_map::{MAILMAP, MDIO};
use mt25q::Mt25q;
use satcat5::mailmap::MailMap;
use ticker::Ticker;
use traffic::Traffic;

/// コアは、ボードの 25 MHz の SYSTEM_25M_CLK で動く。
const CLK_HZ: u32 = 25_000_000;
const CONSOLE_BAUD: u32 = 115_200;

/// この基板の Flash の線は、1 MHz では READ ID が半分ほど失敗し、100 kHz では失敗しなかった。
const FLASH_SCK_HZ: u32 = 100_000;
/// Flash は、SPI の 0 番のチップセレクトにつながる。
const FLASH_CS: u8 = 0;

/// ボードの DP83867 は、PHY アドレス 0 で応答する。
const DP83867_PHY_ADDR: u32 = 0;

/// GPIO の入力の最下位は、PHY のリセットの解除から MDIO を使えるまでの待ちが終わったことを示す。
const GPIO_IN_PHY_READY: u32 = 1 << 0;

/// リンクは MDIO で読むため、読む間隔をあけて通信の処理を妨げないようにする。
/// LED の点滅も、この間隔で切り替える。
const LINK_CHECK_MSEC: u64 = 500;

/// パニックしたことを defmt で送ってから止まる。
/// 場所を読むと、パニックのメッセージの整形に使う core::fmt が残り、命令メモリが約 20 KB 増えるため、読まない。
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panicked");
    loop {}
}

#[riscv_rt::entry]
fn main() -> ! {
    let peripherals = pac::Peripherals::take().unwrap();
    let mtime = Mtime::new(peripherals.clint, CLK_HZ);
    let phy = Dp83867::new(MDIO, DP83867_PHY_ADDR);
    let gpio = Gpio::new(peripherals.gpio);

    let mut console = Console::new(Uart::new(peripherals.uart0, CLK_HZ, CONSOLE_BAUD), phy, mtime);
    let out = console.output();
    out.puts("\n");
    out.puts_styled(Style::HEADING, "Managed switch on NEORV32.");
    out.puts(" Type \"help\" for the commands.\n");

    let mut config = Config::load(Mt25q::new(Spi::new(peripherals.spi, CLK_HZ, FLASH_SCK_HZ, FLASH_CS)));
    if config.saved().is_some() {
        defmt::info!("Loaded the settings from the SPI Flash");
    } else {
        defmt::warn!("No saved settings in the SPI Flash, using the defaults");
        console.output().puts_styled(Style::WARNING, "No saved settings. Using the defaults.\n");
    }

    while gpio.read() & GPIO_IN_PHY_READY == 0 {}
    phy.init();

    let mut storage: [SocketStorage; 1] = Default::default();
    let mut host = Host::new(MailMap::new(MAILMAP), mtime, config.current().mac, &mut storage);
    config.apply(&mut host);

    let mut traffic = Traffic::new(mtime);
    let mut links = Links::new(phy);
    let mut leds = Leds::new(gpio);
    let mut sample_timer = Ticker::new(mtime, traffic::SAMPLE_MSEC);
    let mut link_timer = Ticker::new(mtime, LINK_CHECK_MSEC);

    console.prompt();

    loop {
        host.process_frames();

        console.process_input(&mut config, &mut traffic);
        config.apply(&mut host);

        if sample_timer.due() {
            traffic.sample();
        }
        if link_timer.due() {
            links.check();
            leds.toggle_heartbeat();
            leds.set_link(links.dp83867_up());
        }
        leds.show_rx_count(host.rx_count());
    }
}
