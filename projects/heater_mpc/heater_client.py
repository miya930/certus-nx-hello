# /// script
# requires-python = ">=3.10"
# ///
"""NUCLEO-H753ZI のヒーター制御のファームウェアにコマンドを送り、送られてくる状態を表示する。

例:
  uv run projects/heater_mpc/heater_client.py setpoint 40
  uv run projects/heater_mpc/heater_client.py mode mpc
  uv run projects/heater_mpc/heater_client.py            # 状態を表示するだけ
"""

import argparse
import json
import socket

DEFAULT_ADDRESS = "192.168.10.50"
PORT = 5005
TIMEOUT_S = 3.0
DATAGRAM_CAPACITY = 1024
GRID_COLUMNS = 3


def grid(values, digits):
    rows = [values[i : i + GRID_COLUMNS] for i in range(0, len(values), GRID_COLUMNS)]
    return " | ".join(" ".join(f"{v:7.{digits}f}" for v in row) for row in rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--address", default=DEFAULT_ADDRESS)
    parser.add_argument("command", nargs="*", help="コマンドを 1 つ。省略すると watch を送る")
    args = parser.parse_args()

    board = (args.address, PORT)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(TIMEOUT_S)
    # 状態はコマンドを最後に送った相手に届くため、表示だけのときも watch を送って登録する。
    sock.sendto((" ".join(args.command) or "watch").encode(), board)
    reply = sock.recv(DATAGRAM_CAPACITY).decode().strip()
    print(reply)
    if reply != "ok":
        raise SystemExit(1)

    print("temperature [degC] and power [W] are listed row by row, left to right. Ctrl+C to stop.")
    while True:
        try:
            state = json.loads(sock.recv(DATAGRAM_CAPACITY))
        except TimeoutError:
            raise SystemExit("no telemetry received") from None
        fault = "  FAULT" if state["fault"] else ""
        print(f"t={state['time']:8.1f} s  mode={state['mode']}  compute={state['computeMs']:.1f} ms{fault}")
        print(f"  temperature {grid(state['temperature'], 2)}")
        print(f"  setpoint    {grid(state['setpoint'], 1)}")
        print(f"  power       {grid(state['power'], 4)}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
