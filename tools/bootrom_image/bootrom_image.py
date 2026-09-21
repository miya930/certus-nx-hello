#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "pyelftools>=0.31",
# ]
# ///
"""Turn the boot ROM ELF into NEORV32's boot ROM image package in VHDL.

Usage:
    uv run tools/bootrom_image/bootrom_image.py \\
        crates/neorv32-bootrom/build/riscv32imc-unknown-none-elf/release/neorv32-bootrom \\
        third_party/neorv32_probe_rs/neorv32_bootrom_image.vhd

NEORV32 の neorv32_bootrom は、パッケージ neorv32_bootrom_image の image_size_c と image_data_c を読む。
ROM のアドレスの幅は image_size_c から決まるため、配列はその幅で指せる語の数だけ取る。
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from elftools.elf.elffile import ELFFile

BOOTROM_BASE = 0xFFE0_0000
WORD_BYTES = 4


def load_image(elf_path: Path) -> bytes:
    """起動 ROM の位置に置く中身を、ELF の読み込み用のセグメントから取り出す。"""
    with elf_path.open("rb") as file:
        elf = ELFFile(file)
        segments = [s for s in elf.iter_segments() if s["p_type"] == "PT_LOAD" and s["p_filesz"] > 0]
        rom = [s for s in segments if s["p_paddr"] >= BOOTROM_BASE]
        if len(rom) != 1 or rom[0]["p_paddr"] != BOOTROM_BASE:
            sys.exit(f"{elf_path}: expected one loadable segment at {BOOTROM_BASE:#010x}")
        data = rom[0].data()
    padding = -len(data) % WORD_BYTES
    return data + bytes(padding)


def render(image: bytes) -> str:
    words = [int.from_bytes(image[i : i + WORD_BYTES], "little") for i in range(0, len(image), WORD_BYTES)]
    # neorv32_bootrom は、image_size_c を 2 のべき乗に切り上げた幅でアドレスを引く。
    rom_words = (1 << (len(image) - 1).bit_length()) // WORD_BYTES
    lines = [
        "library ieee;",
        "use ieee.std_logic_1164.all;",
        "",
        "-- crates/neorv32-bootrom から tools/bootrom_image で生成した。手で編集しない。",
        "-- ブートローダの代わりにこの ROM を置く理由は、同じフォルダの README.md に書いた。",
        "package neorv32_bootrom_image is",
        "",
        f"type rom_t is array (0 to {rom_words - 1}) of std_ulogic_vector(31 downto 0);",
        f"constant image_size_c : natural := {len(image)};",
        "constant image_data_c : rom_t := (",
    ]
    lines += [f'x"{word:08x}",' for word in words]
    lines += ["others => (others => '0')", ");", "", "end package;", ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("elf", type=Path)
    parser.add_argument("vhdl", type=Path)
    args = parser.parse_args()
    image = load_image(args.elf)
    args.vhdl.write_text(render(image))
    print(f"{args.vhdl}: {len(image)} bytes")


if __name__ == "__main__":
    main()
