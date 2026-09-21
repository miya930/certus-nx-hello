//! コンソールのコマンドを解釈して実行する。

use super::output::{put, put_dec, put_dec_padded, put_hex, puts, puts_padded};
use super::style::{puts_styled, puts_styled_padded, BOLD, GREEN, RED, RESET, YELLOW};
use crate::dp83867::Dp83867;
use crate::memory_map::{PORT_STATS, SWITCH_CORE};
use crate::mt25q::Mt25q;
use crate::ports::{PORT_NAMES, PORT_RGMII, PORT_RMII};
use crate::satcat5::port_stats::RMII_STATUS_LOCK;
use crate::settings::{self, Settings};
use crate::traffic::Traffic;
use neorv32_hal::{mtime::Mtime, spi::Spi};

pub struct State {
    /// 動作中の設定。
    pub settings: Settings,
    /// Flash に保存してある設定。保存したことがなければ None になる。
    pub saved: Option<Settings>,
    pub traffic: Traffic,
    pub flash: Mt25q<Spi>,
    pub phy: Dp83867,
    pub mtime: Mtime,
}

const COMMANDS: [(&str, &str); 9] = [
    ("help", "Show this list."),
    ("status", "Show the link state and the load of each port."),
    ("stats", "Show the traffic since boot. \"stats clear\" restarts the count."),
    ("mac", "Show the MAC address table. \"mac clear\" empties it."),
    ("info", "Show the parameters of the switch core and the uptime."),
    ("show", "Show the settings."),
    ("set", "Change a setting. Type \"set\" for the list."),
    ("save", "Save the settings to the SPI Flash."),
    ("defaults", "Restore the default settings. Use \"save\" to keep them."),
];

const SET_ITEMS: [(&str, &str); 4] = [
    ("ip", "A.B.C.D/N"),
    ("gateway", "A.B.C.D | none"),
    ("mac", "XX:XX:XX:XX:XX:XX"),
    ("mirror", "rgmii | rmii | cpu | none"),
];

const NONE_WORD: &str = "none";
const CLEAR_WORD: &str = "clear";
const HELP_COLUMN: usize = 10;
const MSEC_PER_SEC: u64 = 1000;
const SEC_PER_MIN: u64 = 60;
const MIN_PER_HOUR: u64 = 60;
const HOUR_PER_DAY: u64 = 24;

/// 行を実行し、設定を変えたときは真を返す。呼び出し側が、その設定を動作に反映する。
pub fn execute(line: &str, state: &mut State) -> bool {
    let mut words = line.split_ascii_whitespace();
    let Some(command) = words.next() else {
        return false;
    };
    match command {
        "help" => help(),
        "status" => status(&state.traffic, &state.phy),
        "stats" => stats(words.next(), &mut state.traffic),
        "mac" => mac(words.next()),
        "info" => info(&state.mtime),
        "show" => show(state),
        "set" => return set(words.next(), words.next(), &mut state.settings),
        "save" => {
            let Ok(()) = state.settings.save(&mut state.flash);
            state.saved = Some(state.settings);
            puts_styled(GREEN, "Saved.\n");
        }
        "defaults" => {
            state.settings = settings::DEFAULT;
            puts_styled(YELLOW, "Restored the defaults. Use \"save\" to keep them.\n");
            return true;
        }
        _ => {
            puts(RED);
            puts("Unknown command \"");
            puts(command);
            puts("\". Type \"help\" for the list.\n");
            puts(RESET);
        }
    }
    false
}

/// 入力中の単語の前にある部分から、次に入りうる単語を渡す。
pub fn complete(context: &str, emit: &mut dyn FnMut(&'static str)) {
    let mut words = context.split_ascii_whitespace();
    match (words.next(), words.next(), words.next()) {
        (None, _, _) => COMMANDS.iter().for_each(|(name, _)| emit(name)),
        (Some("set"), None, _) => SET_ITEMS.iter().for_each(|(name, _)| emit(name)),
        (Some("set"), Some("mirror"), None) => {
            PORT_NAMES.iter().for_each(|name| emit(name));
            emit(NONE_WORD);
        }
        (Some("set"), Some("gateway"), None) => emit(NONE_WORD),
        (Some("stats" | "mac"), None, _) => emit(CLEAR_WORD),
        _ => {}
    }
}

fn help() {
    for (name, text) in COMMANDS {
        puts_styled_padded(BOLD, name, HELP_COLUMN);
        puts(text);
        puts("\n");
    }
}

/// 表の見出しを、列の幅に合わせて並べる。最後の列は幅を持たない。
fn header(columns: &[(&str, usize)]) {
    puts(BOLD);
    for &(title, width) in columns {
        puts_padded(title, width);
    }
    puts(RESET);
    puts("\n");
}

fn put_count(count: u64, width: usize) {
    if count > 0 {
        puts(YELLOW);
    }
    put_dec_padded(count, width);
    puts(RESET);
}

fn status(traffic: &Traffic, phy: &Dp83867) {
    header(&[("PORT", 8), ("LINK", 6), ("SPEED", 7), ("DUPLEX", 8), ("RX KBPS", 9), ("TX KBPS", 9), ("NOTE", 0)]);
    for (port, name) in PORT_NAMES.iter().enumerate() {
        puts_padded(name, 8);
        let (rx_kbps, tx_kbps) = traffic.rate_kbps(port);
        match port {
            PORT_RGMII => {
                // RGMII の相手は DP83867 なので、PHY のレジスタからリンクを読む。
                let phy = phy.status();
                if phy.link {
                    puts_styled_padded(GREEN, "up", 6);
                } else {
                    puts_styled_padded(RED, "down", 6);
                }
                put_dec_padded(phy.speed_mbps, 7);
                puts_padded(if phy.full_duplex { "full" } else { "half" }, 8);
                put_dec_padded(rx_kbps, 9);
                put_dec_padded(tx_kbps, 9);
                puts("DP83867");
            }
            PORT_RMII => {
                // LAN8720 の MDIO は PMOD に出していないため、リンクは読めない。
                // RMII のポートが報告する、REF_CLK のロックと速度を出す。
                // REF_CLK が来ていないときの速度は意味を持たない。
                let (speed_mbps, status) = PORT_STATS.link(port);
                let locked = status & RMII_STATUS_LOCK != 0;
                puts_padded("-", 6);
                if locked {
                    put_dec_padded(speed_mbps, 7);
                } else {
                    puts_padded("-", 7);
                }
                puts_padded("-", 8);
                put_dec_padded(rx_kbps, 9);
                put_dec_padded(tx_kbps, 9);
                if locked {
                    puts("LAN8720, REF_CLK locked");
                } else {
                    puts_styled(YELLOW, "LAN8720, no REF_CLK");
                }
            }
            _ => {
                puts_styled_padded(GREEN, "up", 6);
                puts_padded("-", 7);
                puts_padded("-", 8);
                put_dec_padded(rx_kbps, 9);
                put_dec_padded(tx_kbps, 9);
                puts("NEORV32");
            }
        }
        puts("\n");
    }
    puts("Load is measured over the last second.\n");
}

fn stats(argument: Option<&str>, traffic: &mut Traffic) {
    match argument {
        None => {}
        Some(CLEAR_WORD) => {
            traffic.clear();
            puts_styled(GREEN, "Cleared the counters.\n");
            return;
        }
        Some(other) => return unknown_argument(other),
    }
    header(&[
        ("PORT", 8),
        ("RX FRAMES", 11),
        ("RX BCAST", 10),
        ("RX BYTES", 12),
        ("TX FRAMES", 11),
        ("TX BYTES", 12),
        ("DISCARDS", 10),
        ("ERRORS", 0),
    ]);
    for (totals, name) in traffic.totals.iter().zip(PORT_NAMES) {
        puts_padded(name, 8);
        put_dec_padded(totals.rx_frames, 11);
        put_dec_padded(totals.rx_broadcast, 10);
        put_dec_padded(totals.rx_bytes, 12);
        put_dec_padded(totals.tx_frames, 11);
        put_dec_padded(totals.tx_bytes, 12);
        put_count(totals.discards, 10);
        put_count(totals.errors, 0);
        puts("\n");
    }
    puts("Counted over ");
    put_duration(traffic.counted_msec());
    puts(". Discards are FIFO overflows; errors are MAC, PHY and frame errors.\n");
}

/// MAC アドレステーブルの全ての項目を読み、使われているものだけを出す。
fn mac(argument: Option<&str>) {
    match argument {
        None => {}
        Some(CLEAR_WORD) => {
            SWITCH_CORE.mac_clear();
            puts_styled(GREEN, "Cleared the MAC address table.\n");
            return;
        }
        Some(other) => return unknown_argument(other),
    }
    let table_size = SWITCH_CORE.info().table_size;
    header(&[("INDEX", 7), ("MAC ADDRESS", 19), ("PORT", 0)]);
    let mut used: u32 = 0;
    for index in 0..table_size {
        if let Some((mac, port)) = SWITCH_CORE.mac_entry(index) {
            put_dec_padded(index, 7);
            put_mac(&mac);
            puts("  ");
            puts(PORT_NAMES.get(port).copied().unwrap_or("?"));
            puts("\n");
            used += 1;
        }
    }
    put_dec(used);
    puts(" of ");
    put_dec(table_size);
    puts(" entries in use.\n");
}

fn unknown_argument(argument: &str) {
    puts(RED);
    puts("Unknown argument \"");
    puts(argument);
    puts("\".\n");
    puts(RESET);
}

/// 経過時間を、日、時、分、秒で出す。1 日に満たなければ日を省く。
fn put_duration(msec: u64) {
    let seconds = msec / MSEC_PER_SEC;
    let minutes = seconds / SEC_PER_MIN;
    let hours = minutes / MIN_PER_HOUR;
    let days = hours / HOUR_PER_DAY;
    if days > 0 {
        put_dec(days);
        puts("d ");
    }
    for (value, unit) in [(hours % HOUR_PER_DAY, "h "), (minutes % MIN_PER_HOUR, "m "), (seconds % SEC_PER_MIN, "s")] {
        if value < 10 {
            put(b'0');
        }
        put_dec(value);
        puts(unit);
    }
}

fn info(mtime: &Mtime) {
    let info = SWITCH_CORE.info();
    puts("Ports:           ");
    put_dec(info.ports);
    puts("\nData width:      ");
    put_dec(info.data_bits);
    puts(" bits\nCore clock:      ");
    put_dec(info.core_hz);
    puts(" Hz\nMAC table size:  ");
    put_dec(info.table_size);
    puts("\nFrame size:      ");
    put_dec(info.frame_min);
    puts(" - ");
    put_dec(info.frame_max);
    puts(" bytes\nUptime:          ");
    put_duration(mtime.millis());
    puts("\n");
}

fn show(state: &State) {
    let settings = &state.settings;
    puts("ip       ");
    put_ip(&settings.ip);
    put(b'/');
    put_dec(settings.prefix);
    puts("\ngateway  ");
    match settings.gateway {
        Some(gateway) => put_ip(&gateway),
        None => puts(NONE_WORD),
    }
    puts("\nmac      ");
    put_mac(&settings.mac);
    puts("\nmirror   ");
    puts(settings.mirror.map_or(NONE_WORD, |port| PORT_NAMES[port as usize]));
    puts("\n");
    match state.saved {
        None => puts_styled(YELLOW, "No saved settings. Use \"save\" to keep these.\n"),
        Some(saved) if saved != state.settings => {
            puts_styled(YELLOW, "The settings differ from the saved ones. Use \"save\" to keep them.\n")
        }
        Some(_) => {}
    }
}

fn set(item: Option<&str>, value: Option<&str>, settings: &mut Settings) -> bool {
    let (Some(item), Some(value)) = (item, value) else {
        for (name, syntax) in SET_ITEMS {
            puts("set ");
            puts_padded(name, 8);
            puts(syntax);
            puts("\n");
        }
        return false;
    };
    let accepted = match item {
        "ip" => parse_cidr(value).map(|(ip, prefix)| {
            settings.ip = ip;
            settings.prefix = prefix;
        }),
        "gateway" => parse_optional(value, parse_ipv4).map(|gateway| settings.gateway = gateway),
        "mac" => parse_mac(value).map(|mac| settings.mac = mac),
        "mirror" => parse_optional(value, parse_port).map(|port| settings.mirror = port),
        _ => {
            puts(RED);
            puts("Unknown setting \"");
            puts(item);
            puts("\". Type \"set\" for the list.\n");
            puts(RESET);
            return false;
        }
    };
    if accepted.is_none() {
        puts(RED);
        puts("Invalid value \"");
        puts(value);
        puts("\".\n");
        puts(RESET);
        return false;
    }
    true
}

/// "none" は値がないことを表す。
fn parse_optional<T>(text: &str, parse: fn(&str) -> Option<T>) -> Option<Option<T>> {
    if text == NONE_WORD {
        Some(None)
    } else {
        parse(text).map(Some)
    }
}

fn parse_port(text: &str) -> Option<u8> {
    PORT_NAMES.iter().position(|&name| name == text).map(|port| port as u8)
}

fn parse_ipv4(text: &str) -> Option<[u8; 4]> {
    let mut ip = [0; 4];
    let mut parts = text.split('.');
    for octet in ip.iter_mut() {
        *octet = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(ip)
}

fn parse_cidr(text: &str) -> Option<([u8; 4], u8)> {
    let (ip, prefix) = text.split_once('/')?;
    let prefix: u8 = prefix.parse().ok()?;
    (prefix <= 32).then_some((parse_ipv4(ip)?, prefix))
}

/// マルチキャストの MAC は送信元に使えないため、最初のバイトの最下位ビットが 1 のものは受け付けない。
fn parse_mac(text: &str) -> Option<[u8; 6]> {
    let mut mac = [0; 6];
    let mut parts = text.split(':');
    for byte in mac.iter_mut() {
        *byte = u8::from_str_radix(parts.next()?, 16).ok()?;
    }
    (parts.next().is_none() && mac[0] & 1 == 0).then_some(mac)
}

fn put_ip(ip: &[u8; 4]) {
    for (index, &octet) in ip.iter().enumerate() {
        if index > 0 {
            put(b'.');
        }
        put_dec(octet);
    }
}

fn put_mac(mac: &[u8; 6]) {
    for (index, &byte) in mac.iter().enumerate() {
        if index > 0 {
            put(b':');
        }
        put_hex(byte as u32, 2);
    }
}
