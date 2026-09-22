//! コンソールのコマンドを解釈して実行する。

use crate::config::Config;
use crate::drivers::clock::Clock;
use crate::drivers::dp83867::Dp83867;
use crate::drivers::switch::RMII_STATUS_LOCK;
use crate::drivers::terminal::{Style, Terminal};
use crate::memory_map::SWITCH;
use crate::ports::{PORT_NAMES, PORT_RGMII, PORT_RMII};
use crate::traffic::Traffic;

/// help の一覧の 1 項目。引数を付けた形は、コマンドだけの形の下に並べる。
struct Command {
    name: &'static str,
    text: &'static str,
    /// 引数と、その形で何をするか。
    forms: &'static [(&'static str, &'static str)],
    /// 「コマンド help」で出す、そのまま打てる行。
    examples: &'static [&'static str],
}

const COMMANDS: [Command; 9] = [
    Command { name: "help", text: "Show this list.", forms: &[], examples: &[] },
    Command {
        name: "status",
        text: "Show the link state and the load of each port.",
        forms: &[("PORT", "Show the details of one port: rgmii, rmii or cpu.")],
        examples: &["status", "status rgmii"],
    },
    Command {
        name: "stats",
        text: "Show the traffic since boot.",
        forms: &[(CLEAR_WORD, "Restart the count.")],
        examples: &["stats", "stats clear"],
    },
    Command {
        name: "mac",
        text: "Show the MAC address table.",
        forms: &[(CLEAR_WORD, "Empty the MAC address table.")],
        examples: &["mac", "mac clear"],
    },
    Command { name: "info", text: "Show the parameters of the switch core and the uptime.", forms: &[], examples: &[] },
    Command { name: "show", text: "Show the settings.", forms: &[], examples: &[] },
    Command {
        name: "set",
        text: "List the settings and the values they take.",
        forms: &[],
        examples: &[
            "set ip 192.168.1.10/24",
            "set gateway 192.168.1.1",
            "set gateway none",
            "set mac 02:00:00:00:00:01",
            "set mirror rmii",
            "set mirror none",
        ],
    },
    Command { name: "save", text: "Save the settings to the SPI Flash.", forms: &[], examples: &[] },
    Command {
        name: "defaults",
        text: "Restore the default settings. Use \"save\" to keep them.",
        forms: &[],
        examples: &[],
    },
];

/// set の値を変える形は、設定の表と並べないと打てないため、一覧には出さず set help でだけ出す。
const SET_FORM: (&str, &str) = ("ITEM VALUE", "Change a setting now. Use \"save\" to keep it.");

/// set で変えられる設定の名前、値の書式、何を変えるか。
const SET_ITEMS: [(&str, &str, &str); 4] = [
    ("ip", "A.B.C.D/N", "IP address and prefix length of the switch"),
    ("gateway", "A.B.C.D | none", "Default gateway"),
    ("mac", "XX:XX:XX:XX:XX:XX", "MAC address of the switch, not multicast"),
    ("mirror", "rgmii | rmii | cpu | none", "Port that also gets every frame"),
];

const NONE_WORD: &str = "none";
const CLEAR_WORD: &str = "clear";
const HELP_WORD: &str = "help";
const HELP_COLUMN: usize = 16;
const DETAIL_COLUMN: usize = 21;
const SET_ITEM_COLUMN: usize = 9;
const SET_VALUE_COLUMN: usize = 27;

/// 1 行のコマンドを実行する間だけ、設定、統計、出力を借りる。
pub struct Commands<'a> {
    config: &'a mut Config,
    traffic: &'a mut Traffic,
    out: &'a mut Terminal,
    phy: Dp83867,
    clock: Clock,
}

impl<'a> Commands<'a> {
    pub fn new(
        config: &'a mut Config,
        traffic: &'a mut Traffic,
        out: &'a mut Terminal,
        phy: Dp83867,
        clock: Clock,
    ) -> Self {
        Commands { config, traffic, out, phy, clock }
    }

    /// 行を実行する。設定を変えるコマンドは動作中の設定を書き換えるだけで、スイッチへの反映はメインループが行う。
    pub fn execute(&mut self, line: &str) {
        let mut words = line.split_ascii_whitespace();
        let (Some(command), argument) = (words.next(), words.next()) else {
            return;
        };
        // どのコマンドも、引数に help を付けると、形と例を出す。
        if argument == Some(HELP_WORD) {
            if let Some(entry) = COMMANDS.iter().find(|entry| entry.name == command) {
                return self.command_help(entry);
            }
        }
        match command {
            "help" => self.help(),
            "status" => self.status(argument),
            "stats" => self.stats(argument),
            "mac" => self.mac(argument),
            "info" => self.info(),
            "show" => self.show(),
            "set" => self.set(argument, words.next()),
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
            (None, _, _) => COMMANDS.iter().for_each(|command| emit(command.name)),
            (Some(command), None, _) => {
                match command {
                    "status" => PORT_NAMES.iter().for_each(|name| emit(name)),
                    "set" => SET_ITEMS.iter().for_each(|(name, _, _)| emit(name)),
                    "stats" | "mac" => emit(CLEAR_WORD),
                    _ => {}
                }
                if COMMANDS.iter().any(|entry| entry.name == command) {
                    emit(HELP_WORD);
                }
            }
            (Some("set"), Some("mirror"), None) => {
                PORT_NAMES.iter().for_each(|name| emit(name));
                emit(NONE_WORD);
            }
            (Some("set"), Some("gateway"), None) => emit(NONE_WORD),
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
        for command in &COMMANDS {
            self.put_command(command);
        }
        self.out.puts("\nType \"COMMAND help\", such as \"set help\", for examples.\n");
    }

    /// コマンドだけの形と、引数を付けた形を 1 行ずつ出す。
    fn put_command(&mut self, command: &Command) {
        self.out.puts_styled_padded(Style::HEADING, command.name, HELP_COLUMN);
        self.out.puts(command.text);
        self.out.puts("\n");
        for &form in command.forms {
            self.put_form(command.name, form);
        }
    }

    fn put_form(&mut self, name: &str, (argument, text): (&str, &str)) {
        self.out.puts_styled(Style::HEADING, name);
        self.out.put(b' ');
        self.out.puts_padded(argument, HELP_COLUMN - name.len() - 1);
        self.out.puts(text);
        self.out.puts("\n");
    }

    /// 1 つのコマンドの形と例を出す。set は値の書式がないと打てないため、設定の表も出す。
    fn command_help(&mut self, command: &Command) {
        self.put_command(command);
        if command.name == "set" {
            self.put_form(command.name, SET_FORM);
            self.out.puts("\n");
            self.put_set_items();
        }
        if !command.examples.is_empty() {
            self.out.puts("\n");
            self.out.puts_styled(Style::HEADING, "Examples:");
            self.out.puts("\n");
            for example in command.examples {
                self.out.puts("  ");
                self.out.puts(example);
                self.out.puts("\n");
            }
        }
    }

    fn put_set_items(&mut self) {
        self.out.header(&[("ITEM", SET_ITEM_COLUMN), ("VALUE", SET_VALUE_COLUMN), ("CHANGES", 0)]);
        for (name, value, text) in SET_ITEMS {
            self.out.puts_padded(name, SET_ITEM_COLUMN);
            self.out.puts_padded(value, SET_VALUE_COLUMN);
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

    fn status(&mut self, argument: Option<&str>) {
        match argument {
            None => self.status_table(),
            Some(name) => match Self::parse_port(name) {
                Some(port) => self.port_details(port as usize),
                None => self.error(&["Unknown port \"", name, "\".\n"]),
            },
        }
    }

    fn status_table(&mut self) {
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
                    // リンクしていないときの速度と二重は意味を持たない。
                    let phy = self.phy.status();
                    if phy.link {
                        out.puts_styled_padded(Style::OK, "up", 6);
                        out.put_dec_padded(phy.speed_mbps, 7);
                        out.puts_padded(if phy.full_duplex { "full" } else { "half" }, 8);
                    } else {
                        out.puts_styled_padded(Style::ERROR, "down", 6);
                        out.puts_padded("-", 7);
                        out.puts_padded("-", 8);
                    }
                    out.put_dec_padded(rx_kbps, 9);
                    out.put_dec_padded(tx_kbps, 9);
                    out.puts("DP83867");
                }
                PORT_RMII => {
                    // LAN8720 の MDIO は PMOD に出していないため、リンクは読めない。
                    // RMII のポートが報告する、REF_CLK のロックと速度を出す。
                    // REF_CLK が来ていないときの速度は意味を持たない。
                    let link = SWITCH.port_link(port);
                    let locked = link.status & RMII_STATUS_LOCK != 0;
                    out.puts_padded("-", 6);
                    if locked {
                        out.put_dec_padded(link.speed_mbps, 7);
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

    fn label(&mut self, text: &str) {
        self.out.puts_padded(text, DETAIL_COLUMN);
    }

    /// 1 つのポートの状態と、数えている間の送受信とエラーの内訳を出す。
    fn port_details(&mut self, port: usize) {
        self.label("Port:");
        self.out.puts(PORT_NAMES[port]);
        self.out.puts("\n");
        match port {
            PORT_RGMII => self.dp83867_details(),
            PORT_RMII => self.lan8720_details(),
            _ => {
                self.label("Device:");
                self.out.puts("NEORV32\n");
                self.label("Link:");
                self.out.puts_styled(Style::OK, "up\n");
            }
        }
        let (rx_kbps, tx_kbps) = self.traffic.rate_kbps(port);
        self.label("Load:");
        self.out.puts("RX ");
        self.out.put_dec(rx_kbps);
        self.out.puts(" kbps, TX ");
        self.out.put_dec(tx_kbps);
        self.out.puts(" kbps in the last second\n");

        let totals = self.traffic.totals[port];
        self.out.puts("\nCounted over ");
        self.out.put_duration(self.traffic.counted_msec());
        self.out.puts(":\n");
        for (name, count) in [
            ("RX frames:", totals.rx_frames),
            ("RX broadcast frames:", totals.rx_broadcast),
            ("RX bytes:", totals.rx_bytes),
            ("TX frames:", totals.tx_frames),
            ("TX bytes:", totals.tx_bytes),
        ] {
            self.label(name);
            self.out.put_dec(count);
            self.out.puts("\n");
        }
        for (name, count) in [
            ("RX FIFO overflows:", totals.rx_overflows),
            ("TX FIFO overflows:", totals.tx_overflows),
            ("Frame errors:", totals.frame_errors),
            ("MAC/PHY errors:", totals.mii_errors),
        ] {
            self.label(name);
            self.put_count(count, 0);
            self.out.puts("\n");
        }
        if port == PORT_RGMII {
            self.label("PHY receive errors:");
            self.put_count(self.phy.receive_errors().into(), 0);
            self.out.puts("\n");
        }
    }

    /// 速度、二重、MDI-X、相手の能力は、リンクしているときだけ意味を持つ。
    fn dp83867_details(&mut self) {
        let status = self.phy.status();
        self.label("Device:");
        self.out.puts("DP83867\n");
        self.label("Link:");
        if !status.link {
            self.out.puts_styled(Style::ERROR, "down\n");
            return;
        }
        self.out.puts_styled(Style::OK, "up\n");
        self.label("Speed:");
        self.out.put_dec(status.speed_mbps);
        self.out.puts(" Mbps\n");
        self.label("Duplex:");
        self.out.puts(if status.full_duplex { "full\n" } else { "half\n" });
        self.label("MDI:");
        self.out.puts(if status.mdi_x { "MDI-X\n" } else { "MDI\n" });
        self.label("Partner abilities:");
        let Some(partner) = self.phy.partner() else {
            self.out.puts("no auto-negotiation\n");
            return;
        };
        self.put_list(&[
            (partner.full_1000, "1000 full"),
            (partner.half_1000, "1000 half"),
            (partner.full_100, "100 full"),
            (partner.half_100, "100 half"),
            (partner.full_10, "10 full"),
            (partner.half_10, "10 half"),
        ]);
        self.label("Partner pause:");
        self.put_list(&[(partner.pause, "symmetric"), (partner.asymmetric_pause, "asymmetric")]);
    }

    /// 当てはまる項目の名前を、コンマで区切って 1 行に並べる。
    fn put_list(&mut self, items: &[(bool, &str)]) {
        let mut separator = "";
        for (_, name) in items.iter().filter(|(applies, _)| *applies) {
            self.out.puts(separator);
            self.out.puts(name);
            separator = ", ";
        }
        if separator.is_empty() {
            self.out.puts(NONE_WORD);
        }
        self.out.puts("\n");
    }

    /// LAN8720 の MDIO は PMOD に出していないため、RMII のポートが報告する REF_CLK のロックと速度を出す。
    fn lan8720_details(&mut self) {
        let link = SWITCH.port_link(PORT_RMII);
        self.label("Device:");
        self.out.puts("LAN8720\n");
        self.label("REF_CLK:");
        // REF_CLK が来ていないときの速度は意味を持たない。
        if link.status & RMII_STATUS_LOCK == 0 {
            self.out.puts_styled(Style::WARNING, "not locked\n");
            return;
        }
        self.out.puts_styled(Style::OK, "locked\n");
        self.label("Speed:");
        self.out.put_dec(link.speed_mbps);
        self.out.puts(" Mbps\n");
    }

    fn stats(&mut self, argument: Option<&str>) {
        match argument {
            None => {}
            Some(CLEAR_WORD) => {
                self.traffic.clear();
                self.phy.clear_receive_errors();
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
            self.put_count(totals.discards(), 10);
            self.put_count(totals.errors(), 0);
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
                SWITCH.mac_clear();
                self.out.puts_styled(Style::OK, "Cleared the MAC address table.\n");
                return;
            }
            Some(other) => return self.error(&["Unknown argument \"", other, "\".\n"]),
        }
        let out = &mut *self.out;
        // 項目の番号は、QUERY_CTRL の 16 ビットの欄で渡す。
        let table_size = SWITCH.info().table_size as u16;
        out.header(&[("INDEX", 7), ("MAC ADDRESS", 19), ("PORT", 0)]);
        let mut used: u32 = 0;
        for index in 0..table_size {
            if let Some((mac, port)) = SWITCH.mac_entry(index) {
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
        let info = SWITCH.info();
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
        out.put_duration(self.clock.millis());
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
            return self.put_set_items();
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
