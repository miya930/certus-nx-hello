# RISC-V ソフトコアで Rust を動かす

LFD2NX-40 に NEORV32 の RISC-V コアを実装し、Rust で書いたプログラムを動かす。
ファームウェアは UART のブートローダから送るため、FPGA のコンフィグとは別の経路になる。

## 構成

コアは `third_party/neorv32` の NEORV32 で、命令セットは RV32IMC にする。
命令メモリは 16 KB、データメモリは 8 KB である。
クロックは、ボードの 25 MHz の発振器 X2 から SYSTEM_25M_CLK として受ける。

GPIO の下位 8 ビットは、汎用 LED の 8 個につなぐ。
UART は FTDI の Port B につなぎ、ブートローダとプログラムの出力に使う。
押しボタンの SW2 を押している間は、コアをリセットする。

トップは `riscv_rust.vhd`、ピン割り当ては `riscv_rust.pdc`、ファームウェアは `firmware/` に置く。

## 設計

### 起動

起動方法には、NEORV32 の内蔵ブートローダを選ぶ。
電源を入れるとブートローダが UART で待ち受け、受け取ったプログラムを命令メモリに置いて実行する。
ブートローダは ELF を読めないため、Makefile で平坦なバイナリにしてからヘッダを付ける。

SPI のペリフェラルは入れていないため、ブートローダは SPI Flash から起動できない。
ファームウェアは、電源を入れるたびに UART から送る。

### ファームウェア

`firmware/` の `no_std` のプログラムは、UART に文字列を出し、LED に 2 進数のカウンタを表示する。
対象は、コアの命令セットに合わせて `riscv32imc-unknown-none-elf` にする。
起動処理は `riscv-rt` に任せる。
LED の待ち時間はマシンタイマから求めるため、点滅の周期でクロックの設定を確かめられる。

## 検証

`make sim` では、リセット中に LED が消えていることと、リセットを離すとブートローダが UART へ送信を始めることを確かめる。

実機では、ビットストリームを書き込むと、ブートローダのバナーが `/dev/ttyUSB1` に 19200 8N1 で出ることを確かめた。

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
これは論理最適化の内部の知らせで、回路の不具合を示すものではない。
nextpnr-nexus の警告はない。

## 実行方法

OSS CAD Suite と、Rust の RISC-V 向けのツールチェーンを用意する。

```sh
rustup target add riscv32imc-unknown-none-elf
rustup component add llvm-tools
```

UART を使うため、JP25 と JP26 を閉じて FTDI の Port B につなぐ。
この 2 つは既定で閉じている。
Port B は I²C と共用である。
UART になっているときは、LED の D27 が緑に点灯する。

```sh
make        # 合成と配置配線、ビットストリームの生成
make sim    # テストベンチ
make load   # FPGA を SRAM にコンフィグする
make flash  # FPGA を SPI Flash にコンフィグする
make upload # ファームウェアを UART から流し込んで実行する
```

`make upload` のシリアルポートは、`make upload PORT=/dev/ttyUSB0` のように変えられる。
