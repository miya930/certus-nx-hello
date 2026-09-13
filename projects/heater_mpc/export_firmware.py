# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "numpy",
#     "scipy",
# ]
# ///
"""制御器の行列とゲイン、検証データを、ファームウェアが読み込む Rust のソースとして書き出す。"""

import argparse
from pathlib import Path

import numpy as np

from mpc_controller import MpcController
from pid_controller import DERIVATIVE_FILTER, PidController
from thermal_control import AMBIENT, MODEL_DX, PID_DT, PLANT_DT, PLANT_DX, fit_fopdt, imc_gains
from thermal_model import DEFAULT_GEOMETRY, Board

OUTPUT = Path(__file__).parent / "build" / "firmware"

# 画面の既定値と同じ制御器の設定。
SAMPLE_TIME = 5
HORIZON = 60
CONTROL_HORIZON = 10
MOVE_WEIGHT = 100.0
POWER_MAX = 0.5
PID_LAMBDA_RATIO = 0.5

# 検証データのシナリオ。
# 目標を一定にして、ファームウェアと同じく予測ホライズンの間の参照軌道が一定になるようにする。
GOLDEN_DURATION = 400.0
GOLDEN_SETPOINT = [40.0, 40.0, 40.0, 40.0, 50.0, 40.0, 40.0, 40.0, 40.0]

# サーミスタと ADC の想定。基板の部品が決まったら差し替える。
THERMISTOR_R25_OHM = 10_000.0
THERMISTOR_BETA_K = 3950.0
THERMISTOR_PULLUP_OHM = 10_000.0
ADC_FULL_SCALE = 65535.0


def literal(value):
    # Rust では "0" は整数になるため、小数点も指数もない値には ".0" を付ける。
    text = f"{value:.9g}"
    return text if any(c in text for c in ".e") else text + ".0"


def rust_array(name, values):
    flat = np.asarray(values, dtype=np.float32).ravel()
    lines = (", ".join(literal(v) for v in flat[i : i + 8]) for i in range(0, flat.size, 8))
    body = ",\n    ".join(lines)
    return f"pub static {name}: [f32; {flat.size}] = [\n    {body},\n];\n"


def closed_loop(plant, controller, controller_dt, lookahead):
    """Python の制御器で閉ループを回し、各制御周期の測定と電力を返す。"""
    step = plant.stepper(PLANT_DT)
    rise = np.asarray(GOLDEN_SETPOINT) - AMBIENT
    target = np.tile(rise, (lookahead, 1)) if lookahead else rise
    control_steps = round(controller_dt / PLANT_DT)
    temperature = np.zeros(plant.cells)
    power = np.zeros(plant.geometry.heaters)
    measurements, powers = [], []
    for i in range(round(GOLDEN_DURATION / PLANT_DT)):
        if i % control_steps == 0:
            sensor = temperature[plant.sensors]
            power = controller.update(target, sensor)
            measurements.append(sensor.copy())
            powers.append(power.copy())
        temperature = step(temperature, power)
    return np.array(measurements), np.array(powers)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()

    geometry = DEFAULT_GEOMETRY
    plant = Board(geometry, PLANT_DX)
    model = Board(geometry, MODEL_DX)
    heaters = geometry.heaters
    a, b, c = model.discrete_model(SAMPLE_TIME)
    mpc = MpcController(a, b, c, POWER_MAX, HORIZON, CONTROL_HORIZON, MOVE_WEIGHT)

    # 二次計画のヘッセ行列は対角要素の平方根で前処理した形で持たせ、射影勾配法の刻みもここで決める。
    # どちらも制御器の設定だけで決まり、実行中には変わらない。
    moves = CONTROL_HORIZON * heaters
    difference = np.eye(moves) - np.eye(moves, k=-heaters)
    hessian = mpc.forced.T @ mpc.forced + MOVE_WEIGHT * difference.T @ difference
    scaling = np.sqrt(np.diag(hessian))
    hessian_scaled = hessian / np.outer(scaling, scaling)
    qp_step = 1.0 / np.linalg.eigvalsh(hessian_scaled).max()

    pid_gains = imc_gains(fit_fopdt(plant), PID_LAMBDA_RATIO)

    generated = [
        "// export_firmware.py が書き出した制御器の行列とゲイン。手で編集しない。\n",
        "// 行列は行優先で平坦に並べ、温度は周囲温度からの上昇 [K]、電力は [W] で扱う。\n\n",
        f"pub const NX: usize = {model.cells};\n",
        f"pub const NU: usize = {heaters};\n",
        f"pub const NY: usize = {heaters};\n",
        f"pub const NP: usize = {HORIZON};\n",
        f"pub const NC: usize = {CONTROL_HORIZON};\n",
        f"pub const SAMPLE_TIME_S: f32 = {literal(SAMPLE_TIME)};\n",
        f"pub const MOVE_WEIGHT: f32 = {literal(MOVE_WEIGHT)};\n",
        f"pub const POWER_MAX_W: f32 = {literal(POWER_MAX)};\n",
        f"pub const AMBIENT_C: f32 = {literal(AMBIENT)};\n",
        f"pub const QP_STEP: f32 = {literal(qp_step)};\n",
        f"pub const SENSOR_CELLS: [usize; NY] = [{', '.join(str(s) for s in model.sensors)}];\n",
        f"pub const PID_DT_S: f32 = {literal(PID_DT)};\n",
        f"pub const PID_KP: f32 = {literal(pid_gains['kp'])};\n",
        f"pub const PID_TI: f32 = {literal(pid_gains['ti'])};\n",
        f"pub const PID_TD: f32 = {literal(pid_gains['td'])};\n",
        f"pub const PID_DERIVATIVE_FILTER: f32 = {literal(DERIVATIVE_FILTER)};\n",
        f"pub const THERMISTOR_R25_OHM: f32 = {literal(THERMISTOR_R25_OHM)};\n",
        f"pub const THERMISTOR_BETA_K: f32 = {literal(THERMISTOR_BETA_K)};\n",
        f"pub const THERMISTOR_PULLUP_OHM: f32 = {literal(THERMISTOR_PULLUP_OHM)};\n",
        f"pub const ADC_FULL_SCALE: f32 = {literal(ADC_FULL_SCALE)};\n\n",
        "// x[k+1] = A x[k] + B u[k]、NX × NX\n",
        rust_array("A", a),
        "// NX × NU\n",
        rust_array("B", b),
        "// 出力外乱を足したモデルの定常カルマンゲイン、(NX + NY) × NY\n",
        rust_array("L", mpc.gain),
        "// 自由応答 Φ、(NP · NY) × NX\n",
        rust_array("PHI", mpc.free),
        "// 強制応答 Γ、(NP · NY) × (NC · NU)\n",
        rust_array("GAMMA", mpc.forced),
        "// H = ΓᵀΓ + λ DᵀD を S⁻¹ H S⁻¹ に前処理したもの、(NC · NU) × (NC · NU)\n",
        rust_array("HESSIAN_SCALED", hessian_scaled),
        "// 前処理の倍率 S = sqrt(H_ii)、NC · NU\n",
        rust_array("SCALING", scaling),
    ]

    mpc_measurement, mpc_power = closed_loop(plant, mpc, SAMPLE_TIME, HORIZON)
    pid = PidController(pid_gains["kp"], pid_gains["ti"], pid_gains["td"], PID_DT, POWER_MAX, heaters)
    pid_measurement, pid_power = closed_loop(plant, pid, PID_DT, 0)

    golden = [
        "// export_firmware.py が書き出した検証データ。手で編集しない。\n",
        "// Python の制御器で閉ループを回したときの、各周期の測定 (周囲温度からの上昇) と電力。\n\n",
        f"pub const SETPOINT_C: [f32; {heaters}] = [{', '.join(literal(v) for v in GOLDEN_SETPOINT)}];\n",
        f"pub const MPC_STEPS: usize = {len(mpc_power)};\n",
        "// MPC_STEPS × NY\n",
        rust_array("MPC_MEASUREMENT", mpc_measurement),
        "// MPC_STEPS × NU\n",
        rust_array("MPC_POWER", mpc_power),
        f"pub const PID_STEPS: usize = {len(pid_power)};\n",
        "// PID_STEPS × NY\n",
        rust_array("PID_MEASUREMENT", pid_measurement),
        "// PID_STEPS × NU\n",
        rust_array("PID_POWER", pid_power),
    ]

    OUTPUT.mkdir(parents=True, exist_ok=True)
    (OUTPUT / "generated.rs").write_text("".join(generated), encoding="utf-8")
    (OUTPUT / "golden_data.rs").write_text("".join(golden), encoding="utf-8")
    matrices = a.size + b.size + mpc.gain.size + mpc.free.size + mpc.forced.size + hessian_scaled.size + scaling.size
    print(f"matrices: {matrices * 4 / 1024:.0f} KB of f32")
    print(f"wrote: {OUTPUT}")


if __name__ == "__main__":
    main()
