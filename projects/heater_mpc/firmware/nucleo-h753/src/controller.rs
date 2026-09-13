use control::model::{AMBIENT_C, NU, PID_DT_S, SAMPLE_TIME_S};
use control::mpc::Mpc;
use control::pid::Pid;
use control::protocol::Mode;
use embassy_time::{Duration, Instant, Ticker};

use crate::shared;

const PID_PERIOD: Duration = Duration::from_millis((PID_DT_S * 1000.0) as u64);
const PID_PERIODS_PER_MPC: u64 = (SAMPLE_TIME_S / PID_DT_S) as u64;

/// 共有状態の温度と目標から電力を決める。
/// 優先度が最も低い executor で動かし、MPC の計算の途中でも入出力のタスクが割り込めるようにする。
#[embassy_executor::task]
pub async fn run() -> ! {
    let mut pid = Pid::new();
    let mut mpc = Mpc::new();
    let mut active = Mode::Off;
    let mut period: u64 = 0;
    let mut ticker = Ticker::every(PID_PERIOD);
    loop {
        ticker.next().await;
        let state = shared::read();
        // 制御器は周囲温度からの上昇で扱う。
        let measurement = state.temperature.map(|t| t - AMBIENT_C);
        let setpoint = state.setpoint.map(|t| t - AMBIENT_C);
        if state.mode != active {
            // 前のモードの積分や推定を持ち越さない。
            active = state.mode;
            pid = Pid::new();
            mpc.reset(&measurement);
            period = 0;
        }

        let start = Instant::now();
        let power = match active {
            Mode::Off => Some([0.0; NU]),
            Mode::Pid => Some(pid.update(&setpoint, &measurement)),
            Mode::Mpc => (period % PID_PERIODS_PER_MPC == 0).then(|| mpc.update(&setpoint, &measurement)),
        };
        // 入出力のタスクに割り込まれた時間も含めた、電力が決まるまでの時間。
        let compute_ms = start.elapsed().as_micros() as f32 / 1000.0;
        period += 1;

        if let Some(power) = power {
            shared::update(|s| {
                // 計算中に異常でモードが変わっていたら、古いモードの電力は書かない。
                if s.mode == active {
                    s.power = power;
                    s.compute_ms = compute_ms;
                }
            });
        }
    }
}
