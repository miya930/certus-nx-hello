#![no_std]
#![no_main]

mod console;
mod dp83867;
mod leds;
mod memory_map;
mod mt25q;
mod ports;
mod satcat5;
mod settings;
mod ticker;
mod traffic;

use defmt_rtt as _;
use neorv32_hal::{gpio::Gpio, mtime::Mtime, pac, spi::Spi, uart::Uart};
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr};

use console::{commands::State, Console, Style};
use dp83867::Dp83867;
use leds::Leds;
use memory_map::{MAILMAP, MDIO, PORT_STATS, SWITCH_CORE};
use mt25q::Mt25q;
use ports::PORT_RMII;
use satcat5::mailmap::MailMap;
use satcat5::port_stats::RMII_STATUS_LOCK;
use settings::Settings;
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
const LINK_POLL_MSEC: u64 = 500;

/// パニックしたことを defmt で送ってから止まる。
/// 場所を読むと、パニックのメッセージの整形に使う core::fmt が残り、命令メモリが約 20 KB 増えるため、読まない。
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("panicked");
    loop {}
}

fn now(mtime: &Mtime) -> Instant {
    Instant::from_millis(mtime.millis() as i64)
}

/// 設定を smoltcp とスイッチコアに反映する。
/// ミラーリングは、指定した 1 つのポートだけをプロミスキャスにして実現する。
fn apply(settings: &Settings, iface: &mut Interface) {
    let [a, b, c, d] = settings.ip;
    defmt::info!("IP address {=u8}.{=u8}.{=u8}.{=u8}/{=u8}", a, b, c, d, settings.prefix);
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
    let mut console = Console::new(Uart::new(peripherals.uart0, CLK_HZ, CONSOLE_BAUD));
    let out = console.output();
    out.puts("\n");
    out.puts_styled(Style::HEADING, "Managed switch on NEORV32.");
    out.puts(" Type \"help\" for the commands.\n");

    let mtime = Mtime::new(peripherals.clint, CLK_HZ);
    let gpio = Gpio::new(peripherals.gpio);
    let mut flash = Mt25q::new(Spi::new(peripherals.spi, CLK_HZ, FLASH_SCK_HZ, FLASH_CS));
    let phy = Dp83867::new(MDIO, DP83867_PHY_ADDR);

    let saved = Settings::load(&mut flash);
    if saved.is_some() {
        defmt::info!("Loaded the settings from the SPI Flash");
    } else {
        defmt::warn!("No saved settings in the SPI Flash, using the defaults");
        console.output().puts_styled(Style::WARNING, "No saved settings. Using the defaults.\n");
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
    // コンソールのコマンドは設定を書き換えるだけなので、反映した設定と比べて、変わったときに反映し直す。
    let mut applied = state.settings;
    apply(&applied, &mut iface);

    // ソケットは開かない。ARP と ICMP の応答は smoltcp が IP の層で処理する。
    let mut storage: [SocketStorage; 1] = Default::default();
    let mut sockets = SocketSet::new(&mut storage[..]);

    console.prompt();

    let mut leds = Leds::new(gpio);
    let mut link_poll = Ticker::new(mtime, LINK_POLL_MSEC);
    let mut link = false;
    let mut rmii_locked = false;

    loop {
        iface.poll(now(&mtime), &mut device, &mut sockets);

        console.poll(&mut state);
        if state.settings != applied {
            applied = state.settings;
            apply(&applied, &mut iface);
        }

        state.traffic.poll();

        if link_poll.due() {
            leds.toggle_heartbeat();
            let status = phy.status();
            if status.link != link {
                link = status.link;
                if link {
                    defmt::info!(
                        "DP83867 link up, {=u32} Mbps, full duplex {=bool}",
                        status.speed_mbps,
                        status.full_duplex
                    );
                } else {
                    defmt::info!("DP83867 link down");
                }
            }
            leds.set_link(link);
            let locked = PORT_STATS.link(PORT_RMII).1 & RMII_STATUS_LOCK != 0;
            if locked != rmii_locked {
                rmii_locked = locked;
                if locked {
                    defmt::info!("RMII REF_CLK locked");
                } else {
                    defmt::warn!("RMII REF_CLK lost");
                }
            }
        }
        leds.show_rx_count(device.rx_count);
    }
}
