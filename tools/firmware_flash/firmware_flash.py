#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "pyelftools>=0.31",
# ]
# ///
"""Write a NEORV32 firmware ELF to the SPI flash with probe-rs, then run it from the instruction memory.

Usage (as the cargo runner of a firmware):
    firmware_flash.py <elf> [probe-rs run arguments...]

probe-rs の設定は、cargo の [env] が渡す PROBE_RS_CHIP などの環境変数で与える。
Flash に書いたあと、同じ ELF を probe-rs run で命令メモリに書いて実行するため、
Flash への書き込みを待たずにリセットでそのまま動き、defmt のログも出る。
電源を入れた直後は命令メモリが空なので、起動 ROM が Flash の firmware image を命令メモリにコピーする。
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from elftools.elf.elffile import ELFFile

# firmware image を置く SPI Flash の位置。probe-rs には、SPI Flash のアドレスをそのまま渡す。
# 位置と firmware image の書式は third_party/neorv32_probe_rs/bootrom と、third_party/neorv32_probe_rs/neorv32.yaml の FLASH に合わせる。
IMAGE_ADDRESS = 0x00F0_0000
IMAGE_MAGIC = b"IMEM"
# 命令メモリはアドレス 0 から始まり、データメモリは 0x80000000 から始まる。
IMEM_END = 0x8000_0000
WORD_BYTES = 4


def load_image(elf_path: Path) -> bytes:
    """命令メモリに置く中身を、ELF の読み込み用のセグメントから並べる。"""
    image = bytearray()
    with elf_path.open("rb") as file:
        for segment in ELFFile(file).iter_segments():
            address = segment["p_paddr"]
            if segment["p_type"] != "PT_LOAD" or segment["p_filesz"] == 0 or address >= IMEM_END:
                continue
            data = segment.data()
            if len(image) < address:
                image += bytes(address - len(image))
            image[address : address + len(data)] = data
    image += bytes(-len(image) % WORD_BYTES)
    return bytes(image)


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    elf_path = Path(sys.argv[1])
    image = load_image(elf_path)
    bin_path = elf_path.with_suffix(".flash.bin")
    bin_path.write_bytes(IMAGE_MAGIC + len(image).to_bytes(4, "little") + image)
    print(f"Writing {len(image)} bytes of {elf_path.name} to the SPI flash", file=sys.stderr)

    download = ["probe-rs", "download", "--binary-format", "bin", "--base-address", hex(IMAGE_ADDRESS)]
    subprocess.run([*download, str(bin_path)], check=True)
    os.execvp("probe-rs", ["probe-rs", "run", str(elf_path), *sys.argv[2:]])


if __name__ == "__main__":
    main()
