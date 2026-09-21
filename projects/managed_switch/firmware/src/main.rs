#![no_std]
#![no_main]

mod cfgbus;
mod clint;
mod commands;
mod console;
mod flash;
mod mailmap;
mod mdio;
mod settings;
mod sgr;
mod switch;
mod traffic;
mod uart;

use core::ptr::{read_volatile, write_volatile};
use panic_halt as _;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr};

use commands::State;
use console::Console;
use settings::Settings;
use traffic::Traffic;

const GPIO_PORT_IN: *const u32 = 0xFFFC_0000 as *const u32;
const GPIO_PORT_OUT: *mut u32 = 0xFFFC_0004 as *mut u32;

/// GPIO の入力の最下位は、PHY のリセットの解除から MDIO を使えるまでの待ちが終わったことを示す。
const GPIO_IN_PHY_READY: u32 = 1 << 0;

// LED は、最下位に DP83867 のリンクを、最上位に動作を示す点滅を出し、残りに受け取ったフレームの数を出す。
const LED_LINK: u32 = 1 << 0;
const LED_HEARTBEAT: u32 = 1 << 7;
const LED_RX_SHIFT: u32 = 1;
const LED_RX_MASK: u32 = 0x3F;

const CONSOLE_BAUD: u32 = 115_200;
/// リンクは MDIO で読むため、読む間隔をあけて通信の処理を妨げないようにする。
const LINK_POLL_MSEC: u64 = 500;

fn now() -> Instant {
    Instant::from_millis(clint::millis() as i64)
}

/// 設定を smoltcp とスイッチコアに反映する。
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
    switch::set_mirror(settings.mirror);
}

#[riscv_rt::entry]
fn main() -> ! {
    uart::init(clint::CLK_HZ, CONSOLE_BAUD);
    uart::puts("\n");
    sgr::puts_styled(sgr::BOLD, "Managed switch on NEORV32.");
    uart::puts(" Type \"help\" for the commands.\n");

    flash::init();
    let saved = Settings::load();
    if saved.is_none() {
        sgr::puts_styled(sgr::YELLOW, "No saved settings. Using the defaults.\n");
    }
    let mut state = State {
        settings: saved.unwrap_or(settings::DEFAULT),
        saved,
        traffic: Traffic::new(),
    };

    while unsafe { read_volatile(GPIO_PORT_IN) } & GPIO_IN_PHY_READY == 0 {}
    mdio::init_phy();

    let mut device = mailmap::MailMap::new();
    let config = Config::new(EthernetAddress(state.settings.mac).into());
    let mut iface = Interface::new(config, &mut device, now());
    apply(&state.settings, &mut iface);

    // ソケットは開かない。ARP と ICMP の応答は smoltcp が IP の層で処理する。
    let mut storage: [SocketStorage; 1] = Default::default();
    let mut sockets = SocketSet::new(&mut storage[..]);

    let mut console = Console::new();
    console.prompt();

    let mut leds = 0;
    let mut next_poll = 0;

    loop {
        iface.poll(now(), &mut device, &mut sockets);

        while let Some(byte) = uart::get() {
            if let Some(line) = console.feed(byte, commands::complete) {
                if commands::execute(line, &mut state) {
                    apply(&state.settings, &mut iface);
                }
                console.prompt();
            }
        }

        if state.traffic.due() {
            state.traffic.sample();
        }

        let millis = clint::millis();
        if millis >= next_poll {
            next_poll = millis + LINK_POLL_MSEC;
            leds ^= LED_HEARTBEAT;
            leds &= !LED_LINK;
            if mdio::phy_status().link {
                leds |= LED_LINK;
            }
        }
        let rx = (device.rx_count & LED_RX_MASK) << LED_RX_SHIFT;
        unsafe { write_volatile(GPIO_PORT_OUT, leds | rx) };
    }
}
