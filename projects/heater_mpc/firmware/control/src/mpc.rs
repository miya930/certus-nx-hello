use crate::linalg::{mat_t_vec, mat_vec};
use crate::model::{
    A, B, GAMMA, HESSIAN_SCALED, L, MOVE_WEIGHT, NC, NP, NU, NX, NY, PHI, POWER_MAX_W, QP_STEP, SCALING, SENSOR_CELLS,
};

/// 推定する状態の数。格子の温度に、センサーごとの出力外乱を足したもの。
const NZ: usize = NX + NY;
const MOVES: usize = NC * NU;
const OUTPUTS: usize = NP * NY;

/// 前の周期の解をずらした値から始めるので、固定回数で打ち切っても解の変化は小さい。
/// 回数を固定して、1 周期の計算時間が状態によって変わらないようにする。
pub const QP_ITERATIONS: usize = 300;

/// 温度は周囲温度からの上昇 [K]、電力は [W] で扱う。
pub struct Mpc {
    estimate: [f32; NZ],
    power: [f32; NU],
    plan: [f32; MOVES],
}

impl Mpc {
    pub const fn new() -> Self {
        Self { estimate: [0.0; NZ], power: [0.0; NU], plan: [0.0; MOVES] }
    }

    /// 制御を始めるときに、ヒーターを止めた状態から今の測定の温度で推定を始める。
    /// 測っていない格子はセンサーの平均とし、センサーごとの差は出力外乱に持たせる。
    pub fn reset(&mut self, measurement: &[f32; NY]) {
        let mean = measurement.iter().sum::<f32>() / NY as f32;
        self.estimate[..NX].fill(mean);
        for (d, y) in self.estimate[NX..].iter_mut().zip(measurement) {
            *d = y - mean;
        }
        self.power = [0.0; NU];
        self.plan = [0.0; MOVES];
    }

    /// reference は各センサーの目標で、予測ホライズンの間は一定とする。
    pub fn update(&mut self, reference: &[f32; NY], measurement: &[f32; NY]) -> [f32; NU] {
        self.estimate_state(measurement);
        let g = self.linear_term(reference);
        self.solve(&g);
        self.power
    }

    fn estimate_state(&mut self, measurement: &[f32; NY]) {
        // 予測: 格子は前の周期の電力を入れて 1 周期進め、出力外乱はそのまま持ち越す。
        let mut predicted = [0.0; NZ];
        let mut heat = [0.0; NX];
        mat_vec(&A, NX, &self.estimate[..NX], &mut predicted[..NX]);
        mat_vec(&B, NU, &self.power, &mut heat);
        for (p, h) in predicted[..NX].iter_mut().zip(&heat) {
            *p += h;
        }
        predicted[NX..].copy_from_slice(&self.estimate[NX..]);

        // 補正: 測定と、予測から期待される測定の差にカルマンゲインをかけて、全ての状態に配る。
        let mut innovation = [0.0; NY];
        for (i, e) in innovation.iter_mut().enumerate() {
            *e = measurement[i] - predicted[SENSOR_CELLS[i]] - predicted[NX + i];
        }
        mat_vec(&L, NY, &innovation, &mut self.estimate);
        for (x, p) in self.estimate.iter_mut().zip(&predicted) {
            *x += p;
        }
    }

    /// 二次計画 ½ UᵀHU + gᵀU の g を返す。
    fn linear_term(&self, reference: &[f32; NY]) -> [f32; MOVES] {
        // 目標から、出力外乱と、電力を入れなくても今の状態がたどる自由応答を引いた分を、電力で追いかける。
        let mut target = [0.0; OUTPUTS];
        mat_vec(&PHI, NX, &self.estimate[..NX], &mut target);
        for (k, t) in target.iter_mut().enumerate() {
            let sensor = k % NY;
            *t = reference[sensor] - self.estimate[NX + sensor] - *t;
        }
        let mut g = [0.0; MOVES];
        mat_t_vec(&GAMMA, MOVES, &target, &mut g);
        // 変化の重みの 1 手目は、前の周期の電力からの差にかかる。
        for (gi, u) in g.iter_mut().zip(&self.power) {
            *gi += MOVE_WEIGHT * u;
        }
        for gi in g.iter_mut() {
            *gi = -*gi;
        }
        g
    }

    /// 0 ≤ U ≤ POWER_MAX_W の二次計画を、加速付きの射影勾配法で解く。
    /// 変数を v = S U に置き換えて H の対角をそろえ、全ての変数が同じ刻みで収束するようにする。
    fn solve(&mut self, g: &[f32; MOVES]) {
        let mut g_scaled = [0.0; MOVES];
        let mut upper = [0.0; MOVES];
        let mut v = [0.0; MOVES];
        for i in 0..MOVES {
            g_scaled[i] = g[i] / SCALING[i];
            upper[i] = POWER_MAX_W * SCALING[i];
            // 前の周期の計画を 1 手ずらし、最後の手はそのまま保持する。
            let shifted = if i + NU < MOVES { self.plan[i + NU] } else { self.plan[i] };
            v[i] = (shifted * SCALING[i]).clamp(0.0, upper[i]);
        }

        let mut y = v;
        let mut hy = [0.0; MOVES];
        let mut t = 1.0f32;
        for _ in 0..QP_ITERATIONS {
            mat_vec(&HESSIAN_SCALED, MOVES, &y, &mut hy);
            let t_next = (1.0 + libm::sqrtf(1.0 + 4.0 * t * t)) / 2.0;
            let momentum = (t - 1.0) / t_next;
            for i in 0..MOVES {
                let next = (y[i] - QP_STEP * (hy[i] + g_scaled[i])).clamp(0.0, upper[i]);
                y[i] = next + momentum * (next - v[i]);
                v[i] = next;
            }
            t = t_next;
        }

        for i in 0..MOVES {
            self.plan[i] = v[i] / SCALING[i];
        }
        self.power.copy_from_slice(&self.plan[..NU]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_stays_within_bounds() {
        let mut mpc = Mpc::new();
        let far_above = [100.0; NY];
        let far_below = [-100.0; NY];
        for measurement in [[0.0; NY], [50.0; NY]] {
            for reference in [far_above, far_below] {
                for power in mpc.update(&reference, &measurement) {
                    assert!((0.0..=POWER_MAX_W).contains(&power), "{power}");
                }
            }
        }
    }

    #[test]
    fn reset_reproduces_measurement() {
        let mut mpc = Mpc::new();
        let measurement = [30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0];
        mpc.reset(&measurement);
        for (i, y) in measurement.iter().enumerate() {
            let expected = mpc.estimate[SENSOR_CELLS[i]] + mpc.estimate[NX + i];
            assert!((expected - y).abs() < 1e-4);
        }
    }

    #[test]
    fn no_power_when_already_at_reference() {
        let mut mpc = Mpc::new();
        let measurement = [0.0; NY];
        assert_eq!(mpc.update(&measurement, &measurement), [0.0; NU]);
    }
}
