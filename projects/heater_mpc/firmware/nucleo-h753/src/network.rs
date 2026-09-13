use control::protocol::{self, Command, TELEMETRY_CAPACITY, Telemetry};
use defmt::{unwrap, warn};
use embassy_futures::join::join;
use embassy_futures::select::{Either, select};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
use embassy_stm32::eth::{Ethernet, GenericPhy, Sma};
use embassy_stm32::peripherals::{ETH, ETH_SMA};
use embassy_time::{Duration, Instant, Ticker};
use heapless::String;
use static_cell::StaticCell;

use crate::shared;

pub type Device = Ethernet<'static, ETH, GenericPhy<Sma<'static, ETH_SMA>>>;

// PC と直接つなぐことを想定し、DHCP を使わずに固定のアドレスにする。
const ADDRESS: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address::new(192, 168, 10, 50), 24);
pub const PORT: u16 = 5005;
const TELEMETRY_PERIOD: Duration = Duration::from_secs(1);
const DATAGRAM_CAPACITY: usize = 256;
const SOCKETS: usize = 1;
const PACKETS: usize = 4;

/// UDP でコマンドを受け、最後にコマンドを送ってきた相手に状態を送る。
#[embassy_executor::task]
pub async fn run(device: Device, seed: u64) -> ! {
    static RESOURCES: StaticCell<StackResources<SOCKETS>> = StaticCell::new();
    let config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: ADDRESS,
        gateway: None,
        dns_servers: heapless::Vec::new(),
    });
    let (stack, mut runner) = embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);
    // スタックの処理と UDP の処理を同じタスクで回し、スタックを executor の間で受け渡さずに済ませる。
    join(runner.run(), serve(stack)).await.0
}

async fn serve(stack: Stack<'_>) -> ! {
    let mut rx_meta = [PacketMetadata::EMPTY; PACKETS];
    let mut tx_meta = [PacketMetadata::EMPTY; PACKETS];
    let mut rx_buffer = [0; DATAGRAM_CAPACITY * PACKETS];
    let mut tx_buffer = [0; TELEMETRY_CAPACITY * PACKETS];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
    unwrap!(socket.bind(PORT));

    let mut peer: Option<IpEndpoint> = None;
    let mut datagram = [0; DATAGRAM_CAPACITY];
    let mut line = String::new();
    let mut ticker = Ticker::every(TELEMETRY_PERIOD);
    loop {
        match select(socket.recv_from(&mut datagram), ticker.next()).await {
            Either::First(Ok((length, meta))) => {
                let command = core::str::from_utf8(&datagram[..length]).ok().and_then(protocol::parse);
                let reply: &[u8] = match command {
                    Some(command) => {
                        apply(command);
                        peer = Some(meta.endpoint);
                        b"ok\n"
                    }
                    None => b"error\n",
                };
                if socket.send_to(reply, meta.endpoint).await.is_err() {
                    warn!("reply was not sent");
                }
            }
            // 受信バッファより長いデータグラムは捨てる。
            Either::First(Err(_)) => {}
            Either::Second(()) => {
                let Some(peer) = peer else { continue };
                let state = shared::read();
                let telemetry = Telemetry {
                    time_s: Instant::now().as_millis() as f32 / 1000.0,
                    mode: state.mode,
                    fault: state.fault,
                    temperature: state.temperature,
                    setpoint: state.setpoint,
                    power: state.applied_power(),
                    compute_ms: state.compute_ms,
                };
                if protocol::format(&telemetry, &mut line).is_err() || socket.send_to(line.as_bytes(), peer).await.is_err() {
                    warn!("telemetry was not sent");
                }
            }
        }
    }
}

fn apply(command: Command) {
    shared::update(|s| match command {
        Command::Watch => {}
        Command::Mode(mode) => s.mode = mode,
        Command::Setpoint(setpoint) => s.setpoint = setpoint,
    });
}
