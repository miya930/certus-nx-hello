"""基板を PID 制御と MPC で温度制御するシナリオを実行し、画面に渡す結果をまとめる。"""

import numpy as np

from mpc_controller import DISTURBANCE_NOISE, MEASUREMENT_NOISE, STATE_NOISE, MpcController
from pid_controller import DERIVATIVE_FILTER, PidController
from thermal_model import H_SIDE, Board

AMBIENT = 25.0  # degC

PLANT_DX = 1e-3  # m
PLANT_DT = 1.0  # s

# MPC は、実際の基板より粗い格子のモデルで予測する。
MODEL_DX = 5e-3  # m

PID_DT = 1.0  # s
STEP_TEST_TIME = 900.0  # s
# PID の λ を選ぶときに試す、開ループの時定数に対する閉ループの時定数の比。
PID_LAMBDA_RATIOS = (0.25, 0.5, 1.0, 2.0, 4.0)

FRAME_INTERVAL = 10.0  # s
HEATMAP_STRIDE = 2
# ヒートマップは 0.1 K 単位の整数で送り、JSON を小さくする。
FRAME_SCALE = 10

# 進み具合の報告に使う、各段階のおおよその所要時間の比。
WEIGHT_FIT = 1.0
WEIGHT_PID_RUN = 2.0
WEIGHT_MPC_RUN = 4.0
PROGRESS_EVERY = 50  # steps

# モデルの値の表示で、丸める桁数。
MODEL_DIGITS = 6


class Progress:
    """段階ごとの重みから全体の進み具合を 0 から 1 で求め、コールバックに渡す。"""

    def __init__(self, callback, total_weight):
        self.callback = callback
        self.total = total_weight
        self.done = 0.0
        self.weight = 0.0
        self.name = ""

    def begin(self, name, weight):
        self.done += self.weight
        self.weight = weight
        self.name = name
        self.update(0.0)

    def update(self, fraction):
        if self.callback:
            self.callback(self.name, min(1.0, (self.done + self.weight * fraction) / self.total))

    def finish(self):
        self.done += self.weight
        self.weight = 0.0
        self.update(0.0)


def setpoint_at(phases, t):
    return next(p["setpoint"] for p in reversed(phases) if t >= p["start"])


def reference_trajectory(phases, ramp_rate, times, heaters):
    """時刻ごとの目標温度 [degC] を返す。

    ramp_rate [K/min] が正のときは、周囲温度から始めて、目標が変わるたびにその速さで新しい目標へ移る。
    """
    reference = np.zeros((times.size, heaters))
    current = np.full(heaters, AMBIENT)
    max_step = np.inf if ramp_rate <= 0 else ramp_rate / 60.0 * PLANT_DT
    for i, t in enumerate(times):
        target = setpoint_at(phases, t)
        current = current + np.clip(target - current, -max_step, max_step)
        reference[i] = current
    return reference


def center_heater(geometry):
    return (geometry.rows // 2) * geometry.cols + geometry.cols // 2


def fit_fopdt(board):
    """中央のヒーターの自分のセンサーへのステップ応答を、一次遅れとむだ時間で近似する。"""
    step = board.stepper(PLANT_DT)
    center = center_heater(board.geometry)
    power = np.zeros(board.geometry.heaters)
    power[center] = 1.0
    temperature = np.zeros(board.cells)
    times = np.arange(PLANT_DT, STEP_TEST_TIME + PLANT_DT, PLANT_DT)
    trace = np.zeros(times.size)
    for i in range(times.size):
        temperature = step(temperature, power)
        trace[i] = temperature[board.sensors[center]]

    gain = trace[-1]
    # 最終値の 28 % と 63 % に達する時刻から、時定数とむだ時間を求める。
    t28 = times[np.argmax(trace >= 0.28 * gain)]
    t63 = times[np.argmax(trace >= 0.63 * gain)]
    tau = 1.5 * (t63 - t28)
    delay = max(t63 - tau, PID_DT)
    return {"gain": gain, "tau": tau, "delay": delay}


def imc_gains(fit, lambda_ratio):
    """閉ループの時定数を λ = lambda_ratio * τ とした IMC の式で PID のゲインを決める。"""
    tau, delay, gain = fit["tau"], fit["delay"], fit["gain"]
    closed_loop = lambda_ratio * tau
    kp = (2 * tau + delay) / (gain * (2 * closed_loop + delay))
    ti = tau + delay / 2
    td = tau * delay / (2 * tau + delay)
    return {"kp": kp, "ti": ti, "td": td, "lambdaRatio": lambda_ratio, "closedLoopTime": closed_loop}


def make_pid(gains, power_max, heaters):
    return PidController(gains["kp"], gains["ti"], gains["td"], PID_DT, power_max, heaters)


def simulate_loop(board, controller, controller_dt, reference, record_frames, progress=None):
    """制御器を基板に閉ループでつなぎ、センサーの温度、電力、温度分布の履歴を返す。"""
    step = board.stepper(PLANT_DT)
    heaters = board.geometry.heaters
    control_steps = round(controller_dt / PLANT_DT)
    lookahead = getattr(controller, "lookahead", 0)
    temperature = np.zeros(board.cells)
    power = np.zeros(heaters)
    samples = reference.shape[0]
    sensor = np.zeros((samples, heaters))
    heater = np.zeros((samples, heaters))
    frames = []
    for i in range(samples):
        sensor[i] = temperature[board.sensors]
        if i % control_steps == 0:
            if lookahead:
                ahead = np.minimum(i + control_steps * np.arange(1, lookahead + 1), samples - 1)
                target = reference[ahead]
            else:
                target = reference[i]
            power = controller.update(target - AMBIENT, sensor[i])
        heater[i] = power
        if record_frames and i % round(FRAME_INTERVAL / PLANT_DT) == 0:
            frame = temperature.reshape(board.ny, board.nx)[::HEATMAP_STRIDE, ::HEATMAP_STRIDE] + AMBIENT
            frames.append(np.round(frame * FRAME_SCALE).astype(int).ravel().tolist())
        if progress and i % PROGRESS_EVERY == 0:
            progress.update(i / samples)
        temperature = step(temperature, power)
    return sensor + AMBIENT, heater, frames


def mean_absolute_error(sensor, reference):
    return float(np.mean(np.abs(sensor - reference)))


def run(board, controller, controller_dt, reference, progress=None):
    sensor, heater, frames = simulate_loop(board, controller, controller_dt, reference, True, progress)
    return {
        "pv": np.round(sensor, 3).tolist(),
        "mv": np.round(heater, 4).tolist(),
        "frames": frames,
        "meanAbsoluteError": mean_absolute_error(sensor, reference),
    }


def evaluate(board, controller, controller_dt, reference):
    """結果を残さずに閉ループを回し、参照軌道に対する平均絶対誤差 [K] だけを返す。"""
    sensor, _, _ = simulate_loop(board, controller, controller_dt, reference, False)
    return mean_absolute_error(sensor, reference)


def pid_weight(pid):
    runs = len(PID_LAMBDA_RATIOS) if pid["mode"] == "imc" and pid["optimize"] else 1
    return WEIGHT_FIT + WEIGHT_PID_RUN * runs


def run_pid(board, power_max, reference, pid, progress):
    """pid の mode が imc なら λ から、explicit なら与えられたゲインで PID を実行する。"""
    progress.begin("PID: step response fit", WEIGHT_FIT)
    fit = fit_fopdt(board)
    heaters = board.geometry.heaters
    common = {"mode": pid["mode"], "sampleTime": PID_DT, "derivativeFilter": DERIVATIVE_FILTER, "fopdt": fit}
    if pid["mode"] == "explicit":
        gains = {"kp": pid["kp"], "ti": pid["ti"], "td": pid["td"]}
        progress.begin("PID: explicit gains", WEIGHT_PID_RUN)
        return {"info": {**common, **gains}, **run(board, make_pid(gains, power_max, heaters), PID_DT, reference, progress)}

    # imc では、optimize のときに候補の λ を全て試して平均誤差が最小のものを選ぶ。
    ratios = PID_LAMBDA_RATIOS if pid["optimize"] else (pid["lambdaRatio"],)
    candidates = []
    for n, ratio in enumerate(ratios, start=1):
        gains = imc_gains(fit, ratio)
        progress.begin(f"PID: λ/τ = {ratio:g} ({n}/{len(ratios)})", WEIGHT_PID_RUN)
        result = run(board, make_pid(gains, power_max, heaters), PID_DT, reference, progress)
        candidates.append((gains, result))
    gains, result = min(candidates, key=lambda c: c[1]["meanAbsoluteError"])
    info = {
        **common,
        **gains,
        "optimized": pid["optimize"],
        "sweep": [{"lambdaRatio": g["lambdaRatio"], "meanAbsoluteError": r["meanAbsoluteError"]} for g, r in candidates],
    }
    return {"info": info, **result}


def make_mpc(model, power_max, mpc):
    return MpcController(*model.discrete_model(mpc["sampleTime"]), power_max,
                         mpc["horizon"], mpc["controlHorizon"], mpc["moveWeight"])


def describe_model(model, a, b, sample_time):
    """MPC が予測に使う状態空間モデルの値を、画面で表示できる形にまとめる。"""
    eigenvalues = np.linalg.eigvals(a).real
    # 離散時間の固有値 λ を時定数 τ = -Ts / ln λ に直す。
    time_constants = -sample_time / np.log(np.clip(eigenvalues, 1e-12, 1 - 1e-12))
    return {
        "dxMm": model.dx * 1e3,
        "sampleTime": sample_time,
        "rows": model.ny,
        "cols": model.nx,
        "states": model.cells,
        "cellCapacity": model.capacity,
        "conductance": model.conductance,
        "cellLoss": 2.0 * H_SIDE * model.dx * model.dx,
        "heaterCells": [row * model.nx + col for row, col in model.heater_cells],
        "sensorCells": model.sensors,
        "a": np.round(a, MODEL_DIGITS).tolist(),
        "b": np.round(b, MODEL_DIGITS).tolist(),
        "timeConstants": np.sort(time_constants)[::-1].tolist(),
    }


def run_mpc(model, plant, power_max, reference, mpc, progress):
    progress.begin("MPC: prediction matrices", WEIGHT_FIT)
    a, b, c = model.discrete_model(mpc["sampleTime"])
    controller = MpcController(a, b, c, power_max, mpc["horizon"], mpc["controlHorizon"], mpc["moveWeight"])
    info = {
        **mpc,
        "modelDxMm": MODEL_DX * 1e3,
        "states": model.cells,
        "stateNoise": STATE_NOISE,
        "disturbanceNoise": DISTURBANCE_NOISE,
        "measurementNoise": MEASUREMENT_NOISE,
    }
    progress.begin("MPC: closed loop", WEIGHT_MPC_RUN)
    result = run(plant, controller, mpc["sampleTime"], reference, progress)
    return {"info": info, "model": describe_model(model, a, b, mpc["sampleTime"]), **result}


def build_reference(phases, ramp_rate, duration, heaters):
    phases = sorted(({"start": p["start"], "setpoint": np.asarray(p["setpoint"], float)} for p in phases),
                    key=lambda p: p["start"])
    times = np.arange(0.0, duration + PLANT_DT, PLANT_DT)
    return phases, reference_trajectory(phases, ramp_rate, times, heaters)


def simulate(geometry, power_max, duration, phases, ramp_rate, pid, mpc, progress=None):
    """phases は start [s] と setpoint [degC] の辞書の列、pid と mpc は画面で決めた設定の辞書。

    progress は (段階の名前, 0 から 1 の進み具合) を受け取る関数で、省略できる。
    """
    tracker = Progress(progress, pid_weight(pid) + WEIGHT_FIT + WEIGHT_MPC_RUN)
    plant = Board(geometry, PLANT_DX)
    model = Board(geometry, MODEL_DX)
    phases, reference = build_reference(phases, ramp_rate, duration, geometry.heaters)
    gain = plant.steady_gain()

    controllers = {
        "PID": run_pid(plant, power_max, reference, pid, tracker),
        "MPC": run_mpc(model, plant, power_max, reference, mpc, tracker),
    }
    tracker.finish()

    mm = plant.dx * 1e3
    return {
        "ambient": AMBIENT,
        "timeStep": PLANT_DT,
        "powerMax": power_max,
        "rows": geometry.rows,
        "cols": geometry.cols,
        "board": {
            "widthMm": plant.nx * mm,
            "heightMm": plant.ny * mm,
            "heaterSizeMm": {"x": geometry.heater_x * 1e3, "y": geometry.heater_y * 1e3},
            "heaters": [{"x": col * mm, "y": row * mm} for row, col in plant.heater_cells],
            "sensors": [{"x": col * mm, "y": row * mm} for row, col in plant.sensor_cells],
        },
        "frame": {
            "interval": FRAME_INTERVAL,
            "scale": FRAME_SCALE,
            "rows": len(range(0, plant.ny, HEATMAP_STRIDE)),
            "cols": len(range(0, plant.nx, HEATMAP_STRIDE)),
            "cellMm": HEATMAP_STRIDE * mm,
        },
        "sv": np.round(reference, 3).tolist(),
        "rampRate": ramp_rate,
        "phases": [
            {"start": p["start"], "setpoint": p["setpoint"].tolist(),
             "requiredPower": np.linalg.solve(gain, p["setpoint"] - AMBIENT).tolist()}
            for p in phases
        ],
        "plant": {
            "dxMm": mm,
            "cells": plant.cells,
            "conductance": plant.conductance,
            "arealCapacity": plant.areal_capacity,
            "cellCapacity": plant.capacity,
            "cellLoss": 2.0 * H_SIDE * plant.dx * plant.dx,
            "convection": H_SIDE,
            "characteristicLengthMm": plant.length * 1e3,
            "steadyGain": gain.tolist(),
        },
        "controllers": controllers,
    }
