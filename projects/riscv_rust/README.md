# RISC-V ソフトコアで Rust を動かす

LFD2NX-40 に NEORV32 の RISC-V コアを実装し、Rust で書いたプログラムを動かす。
ファームウェアは UART のブートローダから送るため、FPGA のコンフィグとは別の経路になる。

## 構成

- コアは NEORV32 で、`third_party/neorv32` に置いている。
- 命令セットは RV32I に、圧縮命令の C と乗除算の M を加える。
- 命令メモリは 16 KB、データメモリは 8 KB である。
- クロックは、ボードの 25 MHz の発振器 X2 から SYSTEM_25M_CLK として受ける。
- 押しボタン SW2 を押している間はリセットする。
- GPIO の下位 8 ビットを、汎用 LED 8 個につなぐ。
- UART は FTDI の Port B につながり、ブートローダとプログラムの出力に使う。

| ファイル | 内容 |
|---|---|
| `riscv_rust.vhd` | トップ |
| `riscv_rust_tb.vhd` | テストベンチ |
| `riscv_rust.pdc` | ピン割り当て |
| `firmware/` | コアで動かす Rust のプログラム |

## 設計

- NEORV32 は VHDL-2008 で書かれ、自身を `neorv32` ライブラリに置くことを前提にしている。
  トップも同じライブラリに入れて、ライブラリの参照を 1 つで済ませる。
- 解析順は NEORV32 の `rtl/file_list_core.f` にあるため、GHDL に解析順を求めない。
- 起動方法には内蔵ブートローダを選ぶ。
  電源を入れると UART で待ち受け、受け取ったプログラムを命令メモリに置いて実行する。
- 押しボタンは押している間だけ 0 になり、コアのリセットも負論理なので、そのままつなぐ。
- LED は 0 で点灯するため、GPIO の出力を反転して出す。
- UART の信号名は FTDI から見た向きである。
  TXD_UART は FTDI が送る線なので FPGA の入力、RXD_UART は FTDI が受ける線なので FPGA の出力になる。

## ファームウェア

`firmware/` に `no_std` の Rust のプログラムを置く。
UART に文字列を出し、LED に 2 進数のカウンタを表示する。

- 対象は `riscv32imc-unknown-none-elf` で、コアに実装した命令セットと合わせる。
- 起動処理は `riscv-rt` に任せる。
- メモリの位置と大きさは `firmware/memory.x` に書き、トップの generic と合わせる。
- UART の速度はブートローダが設定した値をそのまま使い、制御レジスタを書き換えない。
- 待ち時間はマシンタイマの値から求める。
  タイマはシステムクロックで進むため、点滅の周期がクロックの設定と合っているかの確認になる。

ブートローダは ELF を読めない。
`llvm-objcopy` で平坦なバイナリにし、NEORV32 の `image_gen` で署名とチェックサムを付けた実行ファイルにする。

## ジャンパの設定

UART は、JP25 と JP26 を閉じて FTDI の Port B につなぐ。
この 2 つは既定で閉じている。

Port B は I²C と共用で、FTDI の設定でどちらか一方になる。
UART になっているときは、LED の D27 が緑に点灯する。

## 検証

`make sim` のテストベンチで、次の動作を確認している。

- リセット中は LED が全て消灯し、UART の送信線が 1 のままである。
- リセットを離すと、ブートローダが UART へ送信を始める。

実機では、ビットストリームを書き込むとブートローダのバナーが `/dev/ttyUSB1` に 19200 8N1 で出ることを確認した。
SPI Flash からの起動は `ERROR_DEVICE` で失敗し、そのまま `CMD:>` のコンソールに落ちる。
いまの構成では SPI のペリフェラルを実装していないためで、ファームウェアはメモリ上にだけ置ける。

## 回路規模

LFD2NX-40-8BG256C で配置配線した結果を次に示す。

| 資源 | 使用量 | 総量 |
|---|---|---|
| LUT | 4,064 | 32,256 |
| FF | 1,503 | 32,256 |
| EBR | 14 | 84 |
| 分散 RAM | 42 | 4,032 |
| I/O | 12 | 111 |

SYSTEM_25M_CLK の最大周波数は 152 MHz で、25 MHz に対して十分な余裕がある。

合成のログには、ABC が出す `The network is combinational.` という警告が 1 件残る。
これは論理最適化の内部の知らせで、この回路には順序回路も含まれている。
nextpnr-nexus の警告はない。

## 実行方法

yosys、GHDL、nextpnr-nexus を含む OSS CAD Suite が必要になる。

```sh
make        # 合成と配置配線、ビットストリームの生成
make sim    # テストベンチ
make load   # FPGA を SRAM にコンフィグする
make flash  # FPGA を SPI Flash にコンフィグする
make upload # ファームウェアを UART から流し込んで実行する
```

`make upload` が使うシリアルポートは `PORT` で変えられる。

```sh
make upload PORT=/dev/ttyUSB0
```

- `make` は、`build/report.json` に資源ごとの使用数と最大動作周波数を出力する。
- `make sim` は、全てのテストを通過して `All tests finished.` を表示し、終了コード 0 で終わる。
