//! コンソールのコマンドを解釈して実行する。

use crate::mdio;
use crate::settings::{self, Settings};
use crate::sgr::{puts_styled, puts_styled_padded, BOLD, GREEN, RED, RESET, YELLOW};
use crate::switch::{self, PORT_NAMES, PORT_RGMII, PORT_RMII};
use crate::uart::{put, put_dec, put_dec_padded, put_hex, puts, puts_padded};

pub struct State {
    /// 動作中の設定。
    pub settings: Settings,
    /// Flash に保存してある設定。保存したことがなければ None になる。
    pub saved: Option<Settings>,
}

const COMMANDS: [(&str, &str); 8] = [
    ("help", "Show this list."),
    ("status", "Show the link state of each port."),
    ("stats", "Show the traffic since the last \"stats\"."),
    ("info", "Show the parameters of the switch core."),
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
const HELP_COLUMN: usize = 10;

/// 行を実行し、設定を変えたときは真を返す。呼び出し側が、その設定を動作に反映する。
pub fn execute(line: &str, state: &mut State) -> bool {
    let mut words = line.split_ascii_whitespace();
    let Some(command) = words.next() else {
        return false;
    };
    match command {
        "help" => help(),
        "status" => status(),
        "stats" => stats(),
        "info" => info(),
        "show" => show(state),
        "set" => return set(words.next(), words.next(), &mut state.settings),
        "save" => {
            state.settings.save();
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

fn status() {
    puts_styled(BOLD, "PORT    LINK  SPEED  DUPLEX  NOTE\n");
    for (port, name) in PORT_NAMES.iter().enumerate() {
        puts_padded(name, 8);
        match port {
            PORT_RGMII => {
                // RGMII の相手は DP83867 なので、PHY のレジスタからリンクを読む。
                let phy = mdio::phy_status();
                if phy.link {
                    puts_styled_padded(GREEN, "up", 6);
                } else {
                    puts_styled_padded(RED, "down", 6);
                }
                put_dec_padded(phy.speed_mbps, 7);
                puts_padded(if phy.full_duplex { "full" } else { "half" }, 8);
                puts("DP83867");
            }
            PORT_RMII => {
                // LAN8720 の MDIO は PMOD に出していないため、リンクは読めない。
                // RMII のポートが報告する、REF_CLK のロックと速度を出す。
                // REF_CLK が来ていないときの速度は意味を持たない。
                let locked = switch::link_status(port) & switch::RMII_STATUS_LOCK != 0;
                puts_padded("-", 6);
                if locked {
                    put_dec_padded(switch::link_speed_mbps(port), 7);
                } else {
                    puts_padded("-", 7);
                }
                puts_padded("-", 8);
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
                puts("NEORV32");
            }
        }
        puts("\n");
    }
}

fn stats() {
    switch::refresh_stats();
    puts_styled(BOLD, "PORT    RX FRAMES   RX BYTES    TX FRAMES   TX BYTES    ERRORS\n");
    for (port, name) in PORT_NAMES.iter().enumerate() {
        puts_padded(name, 8);
        for register in [switch::STAT_RX_FRAMES, switch::STAT_RX_BYTES, switch::STAT_TX_FRAMES, switch::STAT_TX_BYTES] {
            put_dec_padded(switch::stat(port, register), 12);
        }
        // エラーの数は、上位から MAC と PHY、送信 FIFO のあふれ、受信 FIFO のあふれ、フレームの誤りの順に 8 ビットずつ並ぶ。
        let errors: u32 = switch::stat(port, switch::STAT_ERRORS).to_be_bytes().iter().map(|&count| count as u32).sum();
        if errors > 0 {
            puts(YELLOW);
        }
        put_dec(errors);
        puts(RESET);
        puts("\n");
    }
}

fn info() {
    let info = switch::info();
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
    puts(" bytes\n");
}

fn show(state: &State) {
    let settings = &state.settings;
    puts("ip       ");
    put_ip(&settings.ip);
    put(b'/');
    put_dec(settings.prefix as u32);
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
        put_dec(octet as u32);
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
