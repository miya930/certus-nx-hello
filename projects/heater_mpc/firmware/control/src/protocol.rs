use core::fmt::{self, Write};
use core::ops::RangeInclusive;

use heapless::String;

use crate::model::NU;

/// 受け付ける目標温度 [°C]。ヒーターは冷やせないので下限は設けず、上限は基板を傷めない温度にする。
pub const SETPOINT_RANGE_C: RangeInclusive<f32> = 0.0..=90.0;

/// 全ての値が最も長い書式になっても 1 行が収まる大きさ。
pub const TELEMETRY_CAPACITY: usize = 512;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Off,
    Pid,
    Mpc,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Pid => "pid",
            Mode::Mpc => "mpc",
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Command {
    /// 状態の送り先として、送ってきた相手を登録するだけのコマンド。
    Watch,
    Mode(Mode),
    Setpoint([f32; NU]),
}

/// "watch"、"mode off|pid|mpc"、"setpoint <全ヒーター共通の 1 つか、ヒーターごとの NU 個の温度 [°C]>" を解釈する。
pub fn parse(line: &str) -> Option<Command> {
    let mut words = line.split_ascii_whitespace();
    let command = match words.next()? {
        "watch" => Command::Watch,
        "mode" => Command::Mode(match words.next()? {
            "off" => Mode::Off,
            "pid" => Mode::Pid,
            "mpc" => Mode::Mpc,
            _ => return None,
        }),
        "setpoint" => {
            let mut values = [0.0; NU];
            let mut count = 0;
            for word in words.by_ref() {
                let value: f32 = word.parse().ok()?;
                if count == NU || !SETPOINT_RANGE_C.contains(&value) {
                    return None;
                }
                values[count] = value;
                count += 1;
            }
            match count {
                1 => Command::Setpoint([values[0]; NU]),
                NU => Command::Setpoint(values),
                _ => return None,
            }
        }
        _ => return None,
    };
    words.next().is_none().then_some(command)
}

pub struct Telemetry {
    pub time_s: f32,
    pub mode: Mode,
    pub fault: bool,
    pub temperature: [f32; NU],
    pub setpoint: [f32; NU],
    pub power: [f32; NU],
    pub compute_ms: f32,
}

/// 受け取る側が標準の JSON の解析だけで読めるように、1 行の JSON にする。
pub fn format(t: &Telemetry, out: &mut String<TELEMETRY_CAPACITY>) -> fmt::Result {
    out.clear();
    write!(out, "{{\"time\":{:.1},\"mode\":\"{}\",\"fault\":{}", t.time_s, t.mode.name(), t.fault)?;
    write_array(out, "temperature", &t.temperature, 2)?;
    write_array(out, "setpoint", &t.setpoint, 1)?;
    write_array(out, "power", &t.power, 4)?;
    write!(out, ",\"computeMs\":{:.1}}}\n", t.compute_ms)
}

fn write_array(out: &mut String<TELEMETRY_CAPACITY>, name: &str, values: &[f32], digits: usize) -> fmt::Result {
    write!(out, ",\"{name}\":[")?;
    for (i, v) in values.iter().enumerate() {
        let separator = if i == 0 { "" } else { "," };
        write!(out, "{separator}{v:.digits$}")?;
    }
    out.write_char(']')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modes_and_watch() {
        assert_eq!(parse("watch"), Some(Command::Watch));
        assert_eq!(parse("mode mpc\n"), Some(Command::Mode(Mode::Mpc)));
        assert_eq!(parse("mode pid"), Some(Command::Mode(Mode::Pid)));
        assert_eq!(parse("mode off"), Some(Command::Mode(Mode::Off)));
        assert_eq!(parse("mode fast"), None);
        assert_eq!(parse("mode mpc pid"), None);
    }

    #[test]
    fn parses_common_and_individual_setpoints() {
        assert_eq!(parse("setpoint 40"), Some(Command::Setpoint([40.0; NU])));
        let individual = parse("setpoint 40 41 42 43 44 45 46 47 48");
        assert_eq!(individual, Some(Command::Setpoint([40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 48.0])));
    }

    #[test]
    fn rejects_invalid_setpoints() {
        assert_eq!(parse("setpoint"), None);
        assert_eq!(parse("setpoint 40 41"), None);
        assert_eq!(parse("setpoint 40 40 40 40 40 40 40 40 40 40"), None);
        assert_eq!(parse("setpoint 95"), None);
        assert_eq!(parse("setpoint -1"), None);
        assert_eq!(parse("setpoint NaN"), None);
        assert_eq!(parse("setpoint hot"), None);
    }

    #[test]
    fn longest_telemetry_fits() {
        let t = Telemetry {
            time_s: 9_999_999.0,
            mode: Mode::Mpc,
            fault: false,
            temperature: [-1234.56; NU],
            setpoint: [-90.0; NU],
            power: [-1234.5678; NU],
            compute_ms: 99_999.9,
        };
        let mut out = String::new();
        assert!(format(&t, &mut out).is_ok());
        assert!(out.starts_with("{\"time\":9999999.0,\"mode\":\"mpc\",\"fault\":false,\"temperature\":[-1234.56,"));
        assert!(out.ends_with(",\"computeMs\":99999.9}\n"));
    }
}
