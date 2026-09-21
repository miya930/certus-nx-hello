# NEORV32 に probe-rs で書き込むための層

`third_party/neorv32` の NEORV32 に、probe-rs で JTAG からファームウェアを書き込むための部品を置く。
`projects/` の複数のプロジェクトが、この層を共有する。

## 起動 ROM

`neorv32_bootrom_image.vhd` は、NEORV32 の内蔵ブートローダの代わりに起動 ROM へ置くプログラムである。
NEORV32 の `rtl/file_list_core.f` にある同じ名前のファイルの代わりに、同じ位置で解析する。

命令メモリは RAM で、初期値を持たないため、FPGA のコンフィグ直後は 0 で埋まっている。
0 は不正命令なので、そこから起動するとコアは例外を繰り返す。
NEORV32 は、例外を起こした命令を 1 命令だけ進めても、デバッグモードに戻らない。
probe-rs は接続のたびにコアを 1 命令進めるため、この状態では接続が終わらない。

起動 ROM は、命令メモリの先頭が 0 なら、SPI Flash の `0xF00000` にファームウェアの像があるかを見る。
像があれば命令メモリに写し、命令メモリの先頭へ飛ぶ。
像がないか、設計が SPI を持たなければ、命令メモリの先頭が 0 でなくなるまで自身の中で待つ。
JTAG から書き込んだあとのリセットでは、命令メモリの先頭が 0 でないため、写さずにそのまま飛ぶ。

像は、先頭の 4 バイトが `IMEM` の文字、続く 4 バイトが中身のバイト数で、その後に命令メモリの中身が続く。
像は `tools/firmware_flash` が作り、probe-rs で Flash に書く。

`neorv32_bootrom_image.vhd` は手で編集せず、同じフォルダの `bootrom` から次のように作る。

```sh
cd third_party/neorv32_probe_rs/bootrom && cargo build --release && cd ../../..
uv run tools/bootrom_image/bootrom_image.py \
    third_party/neorv32_probe_rs/bootrom/build/riscv32imc-unknown-none-elf/release/neorv32-bootrom \
    third_party/neorv32_probe_rs/neorv32_bootrom_image.vhd
```

## プロジェクトの側で必要なこと

起動 ROM を使うため、`BOOT_MODE_SELECT` は 0 にする。

probe-rs は、IDCODE の製造元の欄が 0 だと無効な IDCODE として接続しない。
NEORV32 は自身の JEDEC ID を持たないため、`OCD_JEDEC_ID` にコアが載る FPGA の製造元である Lattice の ID を入れる。
この値は、FPGA 自身が返す IDCODE 0x310F1043 の製造元の欄と同じである。

probe-rs は 0.32.0 以降を使う。
0.30.0 は、CMSIS-DAP のプローブで JTAG の TAP を選ばないまま RISC-V のデバッグモジュールにアクセスし、接続に失敗する。
