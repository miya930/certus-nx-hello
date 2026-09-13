use crate::model::{NU, PID_DERIVATIVE_FILTER, PID_DT_S, PID_KP, PID_TD, PID_TI, POWER_MAX_W};

/// 各ヒーターを自分のサーミスタだけで制御する。PID_DT_S ごとに呼ぶ。
pub struct Pid {
    integral: [f32; NU],
    derivative: [f32; NU],
    previous: Option<[f32; NU]>,
}

impl Pid {
    pub const fn new() -> Self {
        Self { integral: [0.0; NU], derivative: [0.0; NU], previous: None }
    }

    pub fn update(&mut self, setpoint: &[f32; NU], measurement: &[f32; NU]) -> [f32; NU] {
        let filter_time = PID_TD / PID_DERIVATIVE_FILTER;
        let mut power = [0.0; NU];
        for i in 0..NU {
            let error = setpoint[i] - measurement[i];
            // 微分は測定値にかけ、目標値を変えたときに出力が跳ねないようにする。
            if let Some(previous) = self.previous {
                let slope = (measurement[i] - previous[i]) / PID_DT_S;
                self.derivative[i] += PID_DT_S / (filter_time + PID_DT_S) * (-PID_TD * slope - self.derivative[i]);
            }
            let unclamped = PID_KP * (error + self.integral[i] + self.derivative[i]);
            power[i] = unclamped.clamp(0.0, POWER_MAX_W);
            // 出力が飽和している向きには積分を進めず、ワインドアップを防ぐ。
            let saturated = unclamped != power[i] && (error > 0.0) == (unclamped > power[i]);
            if !saturated {
                self.integral[i] += error * PID_DT_S / PID_TI;
            }
        }
        self.previous = Some(*measurement);
        power
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturated_output_does_not_wind_up() {
        let mut pid = Pid::new();
        let setpoint = [100.0; NU];
        let measurement = [0.0; NU];
        for _ in 0..1000 {
            assert_eq!(pid.update(&setpoint, &measurement), [POWER_MAX_W; NU]);
        }
        assert_eq!(pid.integral, [0.0; NU]);
    }

    #[test]
    fn integral_accumulates_below_saturation() {
        let mut pid = Pid::new();
        let setpoint = [0.1; NU];
        let measurement = [0.0; NU];
        let first = pid.update(&setpoint, &measurement);
        let second = pid.update(&setpoint, &measurement);
        assert!(second[0] > first[0]);
    }
}
