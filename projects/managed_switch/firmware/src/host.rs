//! スイッチ自身の IP アドレスでの通信。
//! CPU のポートを smoltcp の Device として使う。
//! ソケットは開かず、ARP と ICMP の echo には smoltcp が IP の層で応答する。

use crate::drivers::clock::Clock;
use crate::drivers::switch::CpuPort;
use managed_switch_logic::settings::Settings;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr};

pub struct Host<'a> {
    clock: Clock,
    device: CpuPort,
    iface: Interface,
    sockets: SocketSet<'a>,
}

impl<'a> Host<'a> {
    /// IP アドレスは configure で与える。
    pub fn new(mut device: CpuPort, clock: Clock, mac: [u8; 6], storage: &'a mut [SocketStorage<'a>]) -> Self {
        let config = Config::new(EthernetAddress(mac).into());
        let iface = Interface::new(config, &mut device, Self::now(&clock));
        Host { clock, device, iface, sockets: SocketSet::new(storage) }
    }

    fn now(clock: &Clock) -> Instant {
        Instant::from_millis(clock.millis() as i64)
    }

    /// CPU のポートに届いたフレームを 1 つだけ処理し、応答を送る。
    /// Interface::poll は、デバイスにフレームがある限り受信を続ける。
    /// フレームが届き続けるとメインループのほかの処理が止まるため、受信を 1 フレームずつに分ける。
    pub fn process_frame(&mut self) {
        let now = Self::now(&self.clock);
        self.iface.poll_maintenance(now);
        self.iface.poll_ingress_single(now, &mut self.device, &mut self.sockets);
        self.iface.poll_egress(now, &mut self.device, &mut self.sockets);
    }

    /// MAC アドレス、IP アドレス、ゲートウェイを設定する。
    pub fn configure(&mut self, settings: &Settings) {
        let [a, b, c, d] = settings.ip;
        defmt::info!("IP address {=u8}.{=u8}.{=u8}.{=u8}/{=u8}", a, b, c, d, settings.prefix);
        self.iface.set_hardware_addr(EthernetAddress(settings.mac).into());
        self.iface.update_ip_addrs(|addrs| {
            addrs.clear();
            let cidr = Ipv4Cidr::new(Ipv4Address::from(settings.ip), settings.prefix);
            addrs.push(IpCidr::Ipv4(cidr)).expect("address list is full");
        });
        let routes = self.iface.routes_mut();
        routes.remove_default_ipv4_route();
        if let Some(gateway) = settings.gateway {
            routes.add_default_ipv4_route(Ipv4Address::from(gateway)).expect("route table is full");
        }
    }

    /// CPU のポートで受け取ったフレームの数。
    pub fn rx_count(&self) -> u32 {
        self.device.rx_count
    }
}
