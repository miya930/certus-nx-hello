#!/usr/bin/env python3
"""nextpnr-nexus の報告を読み、クロックごとに目標を満たしたかを確かめる。

Usage:
    python3 check_timing.py build/report.json

nextpnr-nexus の --freq は設計全体に 1 つの目標しか与えられない。
PDC の create_clock も --sdc も内部のクロック網には届かず、制約は既定値のままになる。
この設計は RGMII の 125 MHz と、スイッチコアと CPU の 25 MHz の 2 つを持つため、
合成前の目標を高いほうに合わせ、実際の要求はここで確かめる。

クロックの名前は、トップの信号名から合成時に作られる。
信号名を変えたときは、この表も直す。
"""

from __future__ import annotations

import json
import sys

# 接頭辞と、そのクロックに必要な周波数 (MHz)。
REQUIRED_MHZ = {
    "rx_data": 125.0,   # RGMII の受信
    "tx_ctrl": 125.0,   # RGMII の送信
    "cfg_cmd": 25.0,    # スイッチコア、ConfigBus、CPU
}


def required(clock: str) -> float | None:
    for prefix, mhz in REQUIRED_MHZ.items():
        if clock.startswith(prefix):
            return mhz
    return None


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)

    fmax = json.load(open(sys.argv[1], encoding="utf-8"))["fmax"]
    failed, unknown = [], []

    for clock, result in sorted(fmax.items()):
        achieved = result["achieved"]
        need = required(clock)
        if need is None:
            unknown.append(clock)
        elif achieved < need:
            failed.append(f"{clock}: {achieved:.1f} MHz (要 {need:.0f} MHz)")
        else:
            print(f"{clock}: {achieved:.1f} MHz (要 {need:.0f} MHz)")

    if unknown:
        raise SystemExit("目標が決まっていないクロックがある: " + ", ".join(unknown))
    if failed:
        raise SystemExit("タイミングを満たさないクロックがある:\n  " + "\n  ".join(failed))


if __name__ == "__main__":
    main()
