"""制御器のパラメータを、参照軌道に対する平均絶対誤差が最小になるように探索する。"""

import time

import numpy as np
from scipy.optimize import minimize

from thermal_control import (MODEL_DX, PID_DT, PLANT_DX, Progress, build_reference, evaluate, fit_fopdt, imc_gains,
                             make_mpc, make_pid)
from thermal_model import Board

# PID の探索は、1 mm の基板では 1 回の評価に 1 秒近くかかるため、粗い基板で行い、最後に 1 mm の基板で確かめる。
PID_SEARCH_DX = 2e-3  # m
PID_MAX_EVALUATIONS = 60
# 探索の初期値は IMC の値とし、各ゲインは初期値のこの倍率の範囲に収める。
PID_START_LAMBDA_RATIO = 0.5
PID_SCALE_RANGE = (0.1, 10.0)
PID_INITIAL_STEP = np.log(2.0)

# MPC はモデルと基板のずれで結果が変わるため、1 mm の基板で候補を順に試す。
MPC_MOVE_WEIGHTS = (10.0, 30.0, 100.0, 300.0, 1000.0, 3000.0)
MPC_CONTROL_HORIZONS = (2, 5, 10, 20)

# 進み具合の報告に使う、各段階のおおよその所要時間の比。
WEIGHT_PID_SEARCH = 10.0
WEIGHT_PID_VERIFY = 2.0
WEIGHT_MPC_TRIAL = 2.0


def optimize_pid(geometry, power_max, reference, progress):
    """Nelder–Mead 法で Kp、Ti、Td の対数を探索する。"""
    search = Board(geometry, PID_SEARCH_DX)
    plant = Board(geometry, PLANT_DX)
    heaters = geometry.heaters
    start = imc_gains(fit_fopdt(plant), PID_START_LAMBDA_RATIO)
    base = np.array([start["kp"], start["ti"], start["td"]])
    bounds = np.log(PID_SCALE_RANGE)
    history = []

    def gains_of(theta):
        scale = np.exp(np.clip(theta, *bounds))
        kp, ti, td = base * scale
        return {"kp": kp, "ti": ti, "td": td}

    def cost(theta):
        gains = gains_of(theta)
        error = evaluate(search, make_pid(gains, power_max, heaters), PID_DT, reference)
        history.append({**gains, "error": error})
        progress.update(len(history) / PID_MAX_EVALUATIONS)
        return error

    progress.begin("PID: Nelder–Mead on the 2 mm model", WEIGHT_PID_SEARCH)
    result = minimize(
        cost, np.zeros(3), method="Nelder-Mead",
        options={"maxfev": PID_MAX_EVALUATIONS, "xatol": 0.05, "fatol": 1e-3,
                 "initial_simplex": np.vstack([np.zeros(3), PID_INITIAL_STEP * np.eye(3)])},
    )
    best = gains_of(result.x)
    progress.begin("PID: verifying on the 1 mm board", WEIGHT_PID_VERIFY)
    plant_error = evaluate(plant, make_pid(best, power_max, heaters), PID_DT, reference)
    progress.update(0.5)
    start_error = evaluate(plant, make_pid(start, power_max, heaters), PID_DT, reference)
    return {
        **best,
        "evaluations": len(history),
        "searchError": float(result.fun),
        "plantError": plant_error,
        "start": {**{k: start[k] for k in ("kp", "ti", "td")}, "plantError": start_error},
        "history": history,
    }


def mpc_trial_count(mpc):
    return 1 + len(MPC_MOVE_WEIGHTS) + len([h for h in MPC_CONTROL_HORIZONS if h <= mpc["horizon"]])


def optimize_mpc(geometry, power_max, reference, mpc, progress):
    """操作量の変化の重み、次に制御ホライズンの順に、候補を試して誤差が最小のものを選ぶ。"""
    plant = Board(geometry, PLANT_DX)
    model = Board(geometry, MODEL_DX)
    trials = []
    total = mpc_trial_count(mpc)

    def trial(move_weight, control_horizon):
        progress.begin(f"MPC: λ = {move_weight:g}, Nc = {control_horizon} ({len(trials) + 1}/{total})", WEIGHT_MPC_TRIAL)
        settings = {**mpc, "moveWeight": move_weight, "controlHorizon": control_horizon}
        error = evaluate(plant, make_mpc(model, power_max, settings), settings["sampleTime"], reference)
        trials.append({"moveWeight": move_weight, "controlHorizon": control_horizon, "error": error})
        return error

    start_error = trial(mpc["moveWeight"], mpc["controlHorizon"])
    weight = min(MPC_MOVE_WEIGHTS, key=lambda w: trial(w, mpc["controlHorizon"]))
    horizons = [h for h in MPC_CONTROL_HORIZONS if h <= mpc["horizon"]]
    control_horizon = min(horizons, key=lambda h: trial(weight, h))
    best = min(trials, key=lambda t: t["error"])
    return {
        "moveWeight": best["moveWeight"],
        "controlHorizon": best["controlHorizon"],
        "plantError": best["error"],
        "start": {"moveWeight": mpc["moveWeight"], "controlHorizon": mpc["controlHorizon"], "plantError": start_error},
        "trials": trials,
    }


def optimize(geometry, power_max, duration, phases, ramp_rate, pid, mpc, target, progress=None):
    """target は pid、mpc、both のいずれか。progress は simulate と同じ形の関数で、省略できる。"""
    started = time.time()
    weight = 0.0
    if target in ("pid", "both"):
        weight += WEIGHT_PID_SEARCH + WEIGHT_PID_VERIFY
    if target in ("mpc", "both"):
        weight += WEIGHT_MPC_TRIAL * mpc_trial_count(mpc)
    tracker = Progress(progress, weight)
    _, reference = build_reference(phases, ramp_rate, duration, geometry.heaters)
    result = {}
    if target in ("pid", "both"):
        result["pid"] = optimize_pid(geometry, power_max, reference, tracker)
    if target in ("mpc", "both"):
        result["mpc"] = optimize_mpc(geometry, power_max, reference, mpc, tracker)
    tracker.finish()
    result["seconds"] = time.time() - started
    return result
