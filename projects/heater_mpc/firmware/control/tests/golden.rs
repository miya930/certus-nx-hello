// Python の制御器と同じ測定を与えて、同じ電力を出すことを確かめる。

use control::model::{AMBIENT_C, NU, NY};
use control::mpc::Mpc;
use control::pid::Pid;

mod golden_data {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../build/firmware/golden_data.rs"));
}
use golden_data::{MPC_MEASUREMENT, MPC_POWER, MPC_STEPS, PID_MEASUREMENT, PID_POWER, PID_STEPS, SETPOINT_C};

// 二次計画を固定回数の反復で解くため、厳密に解く Python の結果とは少しずれる。
const MPC_POWER_TOLERANCE_W: f32 = 1e-4;
// PID は同じ式なので、f32 と f64 の丸めの差だけが残る。
const PID_POWER_TOLERANCE_W: f32 = 1e-5;

fn row<const N: usize>(data: &[f32], k: usize) -> [f32; N] {
    data[k * N..(k + 1) * N].try_into().unwrap()
}

fn largest_error(steps: usize, mut update: impl FnMut(&[f32; NY]) -> [f32; NU], measurement: &[f32], power: &[f32]) -> f32 {
    (0..steps)
        .flat_map(|k| {
            let expected: [f32; NU] = row(power, k);
            update(&row(measurement, k)).into_iter().zip(expected).map(|(p, e)| (p - e).abs())
        })
        .fold(0.0, f32::max)
}

#[test]
fn mpc_matches_python() {
    let reference = SETPOINT_C.map(|v| v - AMBIENT_C);
    let mut mpc = Mpc::new();
    let error = largest_error(MPC_STEPS, |y| mpc.update(&reference, y), &MPC_MEASUREMENT, &MPC_POWER);
    println!("largest MPC power error: {error} W");
    assert!(error < MPC_POWER_TOLERANCE_W);
}

#[test]
fn pid_matches_python() {
    let setpoint = SETPOINT_C.map(|v| v - AMBIENT_C);
    let mut pid = Pid::new();
    let error = largest_error(PID_STEPS, |y| pid.update(&setpoint, y), &PID_MEASUREMENT, &PID_POWER);
    println!("largest PID power error: {error} W");
    assert!(error < PID_POWER_TOLERANCE_W);
}
