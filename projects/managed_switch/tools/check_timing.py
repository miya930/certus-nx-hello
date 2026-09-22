#!/usr/bin/env python3
"""nextpnr-nexus の報告を読み、クロックごとに目標を満たしたかを確かめる。

Usage:
    python3 check_timing.py build/report.json

nextpnr-nexus の --freq は設計全体に 1 つの目標しか与えられない。
PDC の create_clock も --sdc も内部のクロック網には届かず、制約は既定値のままになる。
この設計は RGMII の 125 MHz、RMII の 50 MHz、スイッチコアと CPU の 25 MHz を持つため、
合成前の目標を一番高い 125 MHz に合わせ、実際の要求はここで確かめる。

クロックの名前は、トップの信号名とポートの番号から合成時に作られる。
信号名やポートの番号を変えたときは、この表も直す。
"""

from __future__ import annotations

import json
import sys

# クロックの名前と、そのクロックに必要な周波数 (MHz)。
REQUIRED_MHZ = {
    "rx_data[0]$glb_clk": 125.0,  # RGMII の受信
    "tx_ctrl[0]$glb_clk": 125.0,  # RGMII の送信
    "rx_data[125]$glb_clk": 50.0,  # RMII の送受信
    "cfg_cmd[0]$glb_clk": 25.0,  # スイッチコア、ConfigBus、CPU
}


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)

    fmax = json.load(open(sys.argv[1], encoding="utf-8"))["fmax"]
    failed, unknown = [], []

    for clock, result in sorted(fmax.items()):
        achieved = result["achieved"]
        need = REQUIRED_MHZ.get(clock)
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
