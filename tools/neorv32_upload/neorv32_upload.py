#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "pyserial>=3.5",
# ]
# ///
"""Upload an executable to the NEORV32 bootloader over UART.

Usage:
    uv run tools/neorv32_upload/neorv32_upload.py /dev/ttyUSB1 build/neorv32_exe.bin
    uv run tools/neorv32_upload/neorv32_upload.py --watch 10 /dev/ttyUSB1 build/neorv32_exe.bin

ブートローダは auto-boot の待ち時間が過ぎるとコンソールに落ちるため、
リセット直後でもプロンプトが出た後でも、そのまま実行できる。
--watch を付けると、実行を始めた後の出力をその秒数だけ表示する。
"""

from __future__ import annotations

import argparse
import sys
import time

import serial

BAUD = 19200
PROMPT = b"CMD:>"


def read_until(
    port: serial.Serial, pattern: bytes, timeout: float
) -> tuple[bool, bytes]:
    """pattern が現れるまで読む。現れたかどうかと、読んだ全体を返す。"""
    deadline = time.time() + timeout
    buffer = b""
    while time.time() < deadline:
        buffer += port.read(256)
        if pattern in buffer:
            return True, buffer
    return False, buffer


def show(label: str, data: bytes) -> None:
    text = data.decode("utf-8", "replace").strip()
    if text:
        print(f"{label}: {text}", file=sys.stderr)


def upload(port: serial.Serial, image: bytes) -> None:
    # 空白は auto-boot を止めるためのもので、コンソールに落ちた後なら読み捨てられる。
    port.write(b" ")
    found, data = read_until(port, PROMPT, 12.0)
    if not found:
        show("応答", data)
        raise SystemExit("ブートローダのプロンプトが出ない。ボードをリセットして試す。")

    port.write(b"u")
    found, data = read_until(port, b"Awaiting", 5.0)
    if not found:
        show("応答", data)
        raise SystemExit("転送の待ち受けに入らない。")

    port.write(image)
    port.flush()
    # 1 バイトに 10 ビットかかるため、転送にかかる時間から待ち時間を決める。
    found, data = read_until(port, b"OK", len(image) / (BAUD / 10) + 10.0)
    show("転送", data)
    if not found:
        raise SystemExit("ブートローダが実行ファイルを受け付けなかった。")

    port.write(b"e")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port", help="ブートローダがつながるシリアルポート")
    parser.add_argument("image", help="image_gen が作った実行ファイル")
    parser.add_argument(
        "--watch",
        type=float,
        default=0.0,
        help="実行を始めた後、この秒数だけ出力を表示する",
    )
    args = parser.parse_args()

    image = open(args.image, "rb").read()
    with serial.Serial(args.port, BAUD, timeout=0.2) as port:
        port.reset_input_buffer()
        upload(port, image)
        print(f"{len(image)} バイトを転送して実行した。", file=sys.stderr)

        if args.watch > 0:
            deadline = time.time() + args.watch
            while time.time() < deadline:
                chunk = port.read(256)
                if chunk:
                    sys.stdout.write(chunk.decode("utf-8", "replace"))
                    sys.stdout.flush()


if __name__ == "__main__":
    main()
