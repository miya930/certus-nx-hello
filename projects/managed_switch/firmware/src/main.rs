#![no_std]
#![no_main]

mod console;
mod dp83867;
mod memory_map;
mod mt25q;
mod ports;
mod satcat5;
mod settings;
mod traffic;

use neorv32_hal::{gpio::Gpio, mtime::Mtime, pac, spi::Spi, uart::Uart};
use panic_halt as _;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr};

use console::{commands::State, sgr, term, Console};
use dp83867::Dp83867;
use memory_map::{MAILMAP, MDIO, SWITCH_CORE};
use mt25q::Mt25q;
use satcat5::mailmap::MailMap;
use settings::Settings;
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

// LED は、最下位に DP83867 のリンクを、最上位に動作を示す点滅を出し、残りに受け取ったフレームの数を出す。
const LED_LINK: u32 = 1 << 0;
const LED_HEARTBEAT: u32 = 1 << 7;
const LED_RX_SHIFT: u32 = 1;
const LED_RX_MASK: u32 = 0x3F;

/// リンクは MDIO で読むため、読む間隔をあけて通信の処理を妨げないようにする。
const LINK_POLL_MSEC: u64 = 500;

fn now(mtime: &Mtime) -> Instant {
    Instant::from_millis(mtime.millis() as i64)
}

/// 設定を smoltcp とスイッチコアに反映する。
/// ミラーリングは、指定した 1 つのポートだけをプロミスキャスにして実現する。
fn apply(settings: &Settings, iface: &mut Interface) {
    iface.set_hardware_addr(EthernetAddress(settings.mac).into());
    iface.update_ip_addrs(|addrs| {
        addrs.clear();
        let cidr = Ipv4Cidr::new(Ipv4Address::from(settings.ip), settings.prefix);
        addrs.push(IpCidr::Ipv4(cidr)).expect("address list is full");
    });
    iface.routes_mut().remove_default_ipv4_route();
    if let Some(gateway) = settings.gateway {
        iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::from(gateway))
            .expect("route table is full");
    }
    SWITCH_CORE.set_promiscuous(settings.mirror.map_or(0, |port| 1 << port));
}

#[riscv_rt::entry]
fn main() -> ! {
    let peripherals = pac::Peripherals::take().unwrap();
    term::init(Uart::new(peripherals.uart0, CLK_HZ, CONSOLE_BAUD));
    term::puts("\n");
    sgr::puts_styled(sgr::BOLD, "Managed switch on NEORV32.");
    term::puts(" Type \"help\" for the commands.\n");

    let mtime = Mtime::new(peripherals.clint, CLK_HZ);
    let mut gpio = Gpio::new(peripherals.gpio);
    let mut flash = Mt25q::new(Spi::new(peripherals.spi, CLK_HZ, FLASH_SCK_HZ, FLASH_CS));
    let phy = Dp83867::new(MDIO, DP83867_PHY_ADDR);

    let saved = Settings::load(&mut flash);
    if saved.is_none() {
        sgr::puts_styled(sgr::YELLOW, "No saved settings. Using the defaults.\n");
    }
    let mut state = State {
        settings: saved.unwrap_or(settings::DEFAULT),
        saved,
        traffic: Traffic::new(mtime),
        flash,
        phy,
        mtime,
    };

    while gpio.read() & GPIO_IN_PHY_READY == 0 {}
    phy.init();

    let mut device = MailMap::new(MAILMAP);
    let config = Config::new(EthernetAddress(state.settings.mac).into());
    let mut iface = Interface::new(config, &mut device, now(&mtime));
    apply(&state.settings, &mut iface);

    // ソケットは開かない。ARP と ICMP の応答は smoltcp が IP の層で処理する。
    let mut storage: [SocketStorage; 1] = Default::default();
    let mut sockets = SocketSet::new(&mut storage[..]);

    let mut console = Console::new();

    let mut leds = 0;
    let mut next_poll = 0;

    loop {
        iface.poll(now(&mtime), &mut device, &mut sockets);

        if console.poll(&mut state) {
            apply(&state.settings, &mut iface);
        }

        if state.traffic.due() {
            state.traffic.sample();
        }

        let millis = mtime.millis();
        if millis >= next_poll {
            next_poll = millis + LINK_POLL_MSEC;
            leds ^= LED_HEARTBEAT;
            leds &= !LED_LINK;
            if phy.status().link {
                leds |= LED_LINK;
            }
        }
        let rx = (device.rx_count & LED_RX_MASK) << LED_RX_SHIFT;
        gpio.write(leds | rx);
    }
}
