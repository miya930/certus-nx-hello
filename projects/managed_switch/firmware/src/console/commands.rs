//! コンソールのコマンドを解釈して実行する。

use super::output::Output;
use super::style::Style;
use crate::config::Config;
use crate::dp83867::Dp83867;
use crate::memory_map::{PORT_STATS, SWITCH_CORE};
use crate::ports::{PORT_NAMES, PORT_RGMII, PORT_RMII};
use crate::satcat5::port_stats::RMII_STATUS_LOCK;
use crate::traffic::Traffic;
use neorv32_hal::mtime::Mtime;

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

/// 1 行のコマンドを実行する間だけ、設定、統計、出力を借りる。
pub struct Commands<'a> {
    config: &'a mut Config,
    traffic: &'a mut Traffic,
    out: &'a mut Output,
    phy: Dp83867,
    mtime: Mtime,
}

impl<'a> Commands<'a> {
    pub fn new(
        config: &'a mut Config,
        traffic: &'a mut Traffic,
        out: &'a mut Output,
        phy: Dp83867,
        mtime: Mtime,
    ) -> Self {
        Commands { config, traffic, out, phy, mtime }
    }

    /// 行を実行する。設定を変えるコマンドは動作中の設定を書き換えるだけで、スイッチへの反映は主ループが行う。
    pub fn execute(&mut self, line: &str) {
        let mut words = line.split_ascii_whitespace();
        let Some(command) = words.next() else {
            return;
        };
        match command {
            "help" => self.help(),
            "status" => self.status(),
            "stats" => self.stats(words.next()),
            "mac" => self.mac(words.next()),
            "info" => self.info(),
            "show" => self.show(),
            "set" => self.set(words.next(), words.next()),
            "save" => {
                self.config.save();
                defmt::info!("Saved the settings to the SPI Flash");
                self.out.puts_styled(Style::OK, "Saved.\n");
            }
            "defaults" => {
                self.config.restore_defaults();
                self.out.puts_styled(Style::WARNING, "Restored the defaults. Use \"save\" to keep them.\n");
            }
            _ => self.error(&["Unknown command \"", command, "\". Type \"help\" for the list.\n"]),
        }
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

    /// 受け付けなかった入力を知らせる。文は複数の部分をつないで作る。
    fn error(&mut self, parts: &[&str]) {
        self.out.set_style(Style::ERROR);
        for part in parts {
            self.out.puts(part);
        }
        self.out.set_style(Style::NORMAL);
    }

    fn help(&mut self) {
        for (name, text) in COMMANDS {
            self.out.puts_styled_padded(Style::HEADING, name, HELP_COLUMN);
            self.out.puts(text);
            self.out.puts("\n");
        }
    }

    /// 破棄やエラーの数は、0 でなければ目立たせる。
    fn put_count(&mut self, count: u64, width: usize) {
        if count > 0 {
            self.out.set_style(Style::WARNING);
        }
        self.out.put_dec_padded(count, width);
        self.out.set_style(Style::NORMAL);
    }

    fn status(&mut self) {
        let out = &mut *self.out;
        out.header(&[
            ("PORT", 8),
            ("LINK", 6),
            ("SPEED", 7),
            ("DUPLEX", 8),
            ("RX KBPS", 9),
            ("TX KBPS", 9),
            ("NOTE", 0),
        ]);
        for (port, name) in PORT_NAMES.iter().enumerate() {
            out.puts_padded(name, 8);
            let (rx_kbps, tx_kbps) = self.traffic.rate_kbps(port);
            match port {
                PORT_RGMII => {
                    // RGMII の相手は DP83867 なので、PHY のレジスタからリンクを読む。
                    let phy = self.phy.status();
                    if phy.link {
                        out.puts_styled_padded(Style::OK, "up", 6);
                    } else {
                        out.puts_styled_padded(Style::ERROR, "down", 6);
                    }
                    out.put_dec_padded(phy.speed_mbps, 7);
                    out.puts_padded(if phy.full_duplex { "full" } else { "half" }, 8);
                    out.put_dec_padded(rx_kbps, 9);
                    out.put_dec_padded(tx_kbps, 9);
                    out.puts("DP83867");
                }
                PORT_RMII => {
                    // LAN8720 の MDIO は PMOD に出していないため、リンクは読めない。
                    // RMII のポートが報告する、REF_CLK のロックと速度を出す。
                    // REF_CLK が来ていないときの速度は意味を持たない。
                    let (speed_mbps, status) = PORT_STATS.link(port);
                    let locked = status & RMII_STATUS_LOCK != 0;
                    out.puts_padded("-", 6);
                    if locked {
                        out.put_dec_padded(speed_mbps, 7);
                    } else {
                        out.puts_padded("-", 7);
                    }
                    out.puts_padded("-", 8);
                    out.put_dec_padded(rx_kbps, 9);
                    out.put_dec_padded(tx_kbps, 9);
                    if locked {
                        out.puts("LAN8720, REF_CLK locked");
                    } else {
                        out.puts_styled(Style::WARNING, "LAN8720, no REF_CLK");
                    }
                }
                _ => {
                    out.puts_styled_padded(Style::OK, "up", 6);
                    out.puts_padded("-", 7);
                    out.puts_padded("-", 8);
                    out.put_dec_padded(rx_kbps, 9);
                    out.put_dec_padded(tx_kbps, 9);
                    out.puts("NEORV32");
                }
            }
            out.puts("\n");
        }
        out.puts("Load is measured over the last second.\n");
    }

    fn stats(&mut self, argument: Option<&str>) {
        match argument {
            None => {}
            Some(CLEAR_WORD) => {
                self.traffic.clear();
                self.out.puts_styled(Style::OK, "Cleared the counters.\n");
                return;
            }
            Some(other) => return self.error(&["Unknown argument \"", other, "\".\n"]),
        }
        self.out.header(&[
            ("PORT", 8),
            ("RX FRAMES", 11),
            ("RX BCAST", 10),
            ("RX BYTES", 12),
            ("TX FRAMES", 11),
            ("TX BYTES", 12),
            ("DISCARDS", 10),
            ("ERRORS", 0),
        ]);
        let totals = self.traffic.totals;
        for (totals, name) in totals.iter().zip(PORT_NAMES) {
            self.out.puts_padded(name, 8);
            self.out.put_dec_padded(totals.rx_frames, 11);
            self.out.put_dec_padded(totals.rx_broadcast, 10);
            self.out.put_dec_padded(totals.rx_bytes, 12);
            self.out.put_dec_padded(totals.tx_frames, 11);
            self.out.put_dec_padded(totals.tx_bytes, 12);
            self.put_count(totals.discards, 10);
            self.put_count(totals.errors, 0);
            self.out.puts("\n");
        }
        self.out.puts("Counted over ");
        self.out.put_duration(self.traffic.counted_msec());
        self.out.puts(". Discards are FIFO overflows; errors are MAC, PHY and frame errors.\n");
    }

    /// MAC アドレステーブルの全ての項目を読み、使われているものだけを出す。
    fn mac(&mut self, argument: Option<&str>) {
        match argument {
            None => {}
            Some(CLEAR_WORD) => {
                SWITCH_CORE.mac_clear();
                self.out.puts_styled(Style::OK, "Cleared the MAC address table.\n");
                return;
            }
            Some(other) => return self.error(&["Unknown argument \"", other, "\".\n"]),
        }
        let out = &mut *self.out;
        let table_size = SWITCH_CORE.info().table_size;
        out.header(&[("INDEX", 7), ("MAC ADDRESS", 19), ("PORT", 0)]);
        let mut used: u32 = 0;
        for index in 0..table_size {
            if let Some((mac, port)) = SWITCH_CORE.mac_entry(index) {
                out.put_dec_padded(index, 7);
                out.put_mac(&mac);
                out.puts("  ");
                out.puts(PORT_NAMES.get(port).copied().unwrap_or("?"));
                out.puts("\n");
                used += 1;
            }
        }
        out.put_dec(used);
        out.puts(" of ");
        out.put_dec(table_size);
        out.puts(" entries in use.\n");
    }

    fn info(&mut self) {
        let out = &mut *self.out;
        let info = SWITCH_CORE.info();
        out.puts("Ports:           ");
        out.put_dec(info.ports);
        out.puts("\nData width:      ");
        out.put_dec(info.data_bits);
        out.puts(" bits\nCore clock:      ");
        out.put_dec(info.core_hz);
        out.puts(" Hz\nMAC table size:  ");
        out.put_dec(info.table_size);
        out.puts("\nFrame size:      ");
        out.put_dec(info.frame_min);
        out.puts(" - ");
        out.put_dec(info.frame_max);
        out.puts(" bytes\nUptime:          ");
        out.put_duration(self.mtime.millis());
        out.puts("\n");
    }

    fn show(&mut self) {
        let out = &mut *self.out;
        let settings = self.config.current();
        out.puts("ip       ");
        out.put_ip(&settings.ip);
        out.put(b'/');
        out.put_dec(settings.prefix);
        out.puts("\ngateway  ");
        match settings.gateway {
            Some(gateway) => out.put_ip(&gateway),
            None => out.puts(NONE_WORD),
        }
        out.puts("\nmac      ");
        out.put_mac(&settings.mac);
        out.puts("\nmirror   ");
        out.puts(settings.mirror.map_or(NONE_WORD, |port| PORT_NAMES[port as usize]));
        out.puts("\n");
        match self.config.saved() {
            None => out.puts_styled(Style::WARNING, "No saved settings. Use \"save\" to keep these.\n"),
            Some(saved) if saved != settings => {
                out.puts_styled(Style::WARNING, "The settings differ from the saved ones. Use \"save\" to keep them.\n")
            }
            Some(_) => {}
        }
    }

    fn set(&mut self, item: Option<&str>, value: Option<&str>) {
        let (Some(item), Some(value)) = (item, value) else {
            for (name, syntax) in SET_ITEMS {
                self.out.puts("set ");
                self.out.puts_padded(name, 8);
                self.out.puts(syntax);
                self.out.puts("\n");
            }
            return;
        };
        let settings = self.config.current_mut();
        let accepted = match item {
            "ip" => Self::parse_cidr(value).map(|(ip, prefix)| {
                settings.ip = ip;
                settings.prefix = prefix;
            }),
            "gateway" => Self::parse_optional(value, Self::parse_ipv4).map(|gateway| settings.gateway = gateway),
            "mac" => Self::parse_mac(value).map(|mac| settings.mac = mac),
            "mirror" => Self::parse_optional(value, Self::parse_port).map(|port| settings.mirror = port),
            _ => {
                self.error(&["Unknown setting \"", item, "\". Type \"set\" for the list.\n"]);
                return;
            }
        };
        if accepted.is_none() {
            self.error(&["Invalid value \"", value, "\".\n"]);
        }
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
        (prefix <= 32).then_some((Self::parse_ipv4(ip)?, prefix))
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
}
