# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "matplotlib",
#     "numpy",
#     "scipy",
# ]
# ///
"""3x3 の格子に並べたヒーターが、互いの温度センサーにどれだけ影響するかを見積もる。"""

from dataclasses import replace
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

from thermal_model import DEFAULT_GEOMETRY, Board

DX = 1e-3  # m
PITCHES = (10e-3, 15e-3, 20e-3, 25e-3, 30e-3)  # m
COPPER_COVERAGES = (0.5, 1.0)

CENTER = 4
SIDE = 1
CORNER = 0
FAR_CORNER = 8

SIM_TIME = 900.0  # s
DT = 1.0  # s
STEP_RESPONSE_RATIO = 1.0 - np.exp(-1.0)
STEP_PITCH = 20e-3  # m

OUTPUT = Path(__file__).parent / "build" / "thermal_sim.png"


def step_response(board):
    """中央のヒーターに 1 W を入れたときの、各センサーの温度上昇を返す。"""
    step = board.stepper(DT)
    power = np.zeros(len(board.sensors))
    power[CENTER] = 1.0
    times = np.arange(0.0, SIM_TIME + DT, DT)
    temperature = np.zeros(board.cells)
    history = np.zeros((times.size, len(board.sensors)))
    for i in range(1, times.size):
        temperature = step(temperature, power)
        history[i] = temperature[board.sensors]
    return times, history


def rise_time(times, trace):
    reached = np.nonzero(trace >= STEP_RESPONSE_RATIO * trace[-1])[0]
    return times[reached[0]]


def label(coverage):
    return f"2 layers, {coverage:.0%} copper"


def main():
    boards = {c: {p: Board(replace(DEFAULT_GEOMETRY, pitch=p, copper=c), DX) for p in PITCHES} for c in COPPER_COVERAGES}
    gains = {c: {p: b.steady_gain() for p, b in by_pitch.items()} for c, by_pitch in boards.items()}
    steps = {c: step_response(by_pitch[STEP_PITCH]) for c, by_pitch in boards.items()}

    for coverage, by_pitch in gains.items():
        print(label(coverage))
        print("  pitch  board  self center  self corner  side/self  diagonal/self  far corner/self  all on")
        for pitch, g in by_pitch.items():
            self_gain = g[CENTER, CENTER]
            print(
                f"  {pitch * 1e3:3.0f} mm {boards[coverage][pitch].nx * DX * 1e3:3.0f} mm"
                f"  {self_gain:6.1f} K/W  {g[CORNER, CORNER]:6.1f} K/W"
                f"  {g[SIDE, CENTER] / self_gain:8.0%}  {g[CORNER, CENTER] / self_gain:12.0%}"
                f"  {g[FAR_CORNER, CORNER] / g[CORNER, CORNER]:14.0%}"
                f"  {g[CENTER].sum():5.1f} K/W"
            )
        times, history = steps[coverage]
        print(
            f"  time to {STEP_RESPONSE_RATIO:.0%} at {STEP_PITCH * 1e3:.0f} mm pitch, center heater on:"
            f" center {rise_time(times, history[:, CENTER]):.0f} s,"
            f" side {rise_time(times, history[:, SIDE]):.0f} s,"
            f" corner {rise_time(times, history[:, CORNER]):.0f} s"
        )

    fig, (ax_ratio, ax_step) = plt.subplots(1, 2, figsize=(12, 4.5))
    colors = plt.rcParams["axes.prop_cycle"].by_key()["color"]
    for (coverage, by_pitch), color in zip(gains.items(), colors):
        pitches = np.array(list(by_pitch)) * 1e3
        side = [g[SIDE, CENTER] / g[CENTER, CENTER] for g in by_pitch.values()]
        diagonal = [g[CORNER, CENTER] / g[CENTER, CENTER] for g in by_pitch.values()]
        ax_ratio.plot(pitches, side, color=color, marker="o", label=f"{label(coverage)}, side")
        ax_ratio.plot(pitches, diagonal, color=color, marker="s", linestyle="--",
                      label=f"{label(coverage)}, diagonal")
    ax_ratio.set_xlabel("heater pitch [mm]")
    ax_ratio.set_ylabel("steady rise at neighbor sensor / own sensor")
    ax_ratio.set_title("Steady-state coupling from the center heater")
    ax_ratio.set_ylim(bottom=0.0)
    ax_ratio.legend()

    for (coverage, (times, history)), color in zip(steps.items(), colors):
        for index, style, name in ((CENTER, "-", "center"), (SIDE, "--", "side"), (CORNER, ":", "corner")):
            ax_step.plot(times, history[:, index], color=color, linestyle=style,
                         label=f"{label(coverage)}, {name}")
    ax_step.set_xlabel("time [s]")
    ax_step.set_ylabel("temperature rise [K] per 1 W")
    ax_step.set_title(f"Step response to the center heater, {STEP_PITCH * 1e3:.0f} mm pitch")
    ax_step.legend()

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    fig.tight_layout()
    fig.savefig(OUTPUT, dpi=120)
    print(f"wrote: {OUTPUT}")


if __name__ == "__main__":
    main()
