//! スイッチ自身の IP アドレスでの通信。
//! CPU のポートを smoltcp の Device として使う。
//! ソケットは開かず、ARP と ICMP の echo には smoltcp が IP の層で応答する。

use crate::satcat5::mailmap::MailMap;
use crate::settings::Settings;
use neorv32_hal::mtime::Mtime;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr};

pub struct Host<'a> {
    mtime: Mtime,
    device: MailMap,
    iface: Interface,
    sockets: SocketSet<'a>,
}

impl<'a> Host<'a> {
    /// IP アドレスは configure で与える。
    pub fn new(mut device: MailMap, mtime: Mtime, mac: [u8; 6], storage: &'a mut [SocketStorage<'a>]) -> Self {
        let config = Config::new(EthernetAddress(mac).into());
        let iface = Interface::new(config, &mut device, Self::now(&mtime));
        Host { mtime, device, iface, sockets: SocketSet::new(storage) }
    }

    fn now(mtime: &Mtime) -> Instant {
        Instant::from_millis(mtime.millis() as i64)
    }

    /// CPU のポートに届いたフレームを処理し、応答を送る。
    pub fn process_frames(&mut self) {
        self.iface.poll(Self::now(&self.mtime), &mut self.device, &mut self.sockets);
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
