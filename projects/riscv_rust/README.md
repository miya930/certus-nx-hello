# RISC-V ソフトコアで動く Rust ファームウェア

LFD2NX-40 に NEORV32 の RISC-V コアを実装し、Rust で書いたプログラムを動かす。
ファームウェアは `cargo run` で JTAG から書き込むため、FPGA のコンフィグとは別の経路になる。

## 構成

コアは `third_party/neorv32` の NEORV32 で、命令セットは RV32IMC にする。
命令メモリは 16 KB、データメモリは 8 KB である。
クロックは、ボードの 25 MHz の発振器 X2 から SYSTEM_25M_CLK として受ける。

GPIO の下位 8 ビットは、汎用 LED の 8 個につなぐ。
UART は FTDI の Port B につなぎ、プログラムの出力に使う。
押しボタンの SW2 を押している間は、コアをリセットする。
NEORV32 の JTAG は PMOD の J5 に出し、probe-rs からファームウェアを書き込めるようにする。

トップは `riscv_rust.vhd`、ピン割り当ては `riscv_rust.pdc` に置く。
起動 ROM は、`third_party/neorv32_probe_rs` のものを使う。
ファームウェアは `firmware/` に置く。
probe-rs に渡すメモリの配置は、`neorv32.yaml` の `variants` のうち、`riscv_rust` の項目に置く。
`neorv32.yaml` は、`third_party/neorv32_probe_rs/neorv32.yaml` へのシンボリックリンクである。

### メモリマップ

コアから見たアドレスの割り当てを次に示す。
全ての領域は FPGA の中にあり、FPGA の外とは I/O のピンでつながる。

| アドレス     | 大きさ | 中身                 | FPGA での実体               |
| ------------ | ------ | -------------------- | --------------------------- |
| `0x00000000` | 16 KB  | 命令メモリ           | EBR 8 個                    |
| `0x80000000` | 8 KB   | データメモリ         | EBR 4 個                    |
| `0xFFE00000` | 2 KB   | 起動 ROM             | EBR                         |
| `0xFFF40000` | 64 KB  | CLINT のマシンタイマ | FPGA の中のレジスタ         |
| `0xFFF50000` | 64 KB  | UART0                | TXD_UART と RXD_UART のピン |
| `0xFFFC0000` | 64 KB  | GPIO                 | 汎用 LED のピン             |
| `0xFFFE0000` | 64 KB  | SYSINFO              | FPGA の中のレジスタ         |
| `0xFFFF0000` | 64 KB  | デバッグモジュール   | J5 の JTAG のピン           |

外部バスは入れていないため、表にないアドレスを読み書きするとバスエラーの例外になる。
命令メモリとデータメモリの位置と大きさは、`riscv_rust.vhd` の generic、`firmware/memory.x`、`neorv32.yaml` で合わせる。

## 設計

### 起動

この設計は SPI を持たないため、起動 ROM は Flash を見ず、命令メモリの先頭が 0 のあいだは自身の中で待つ。
JTAG から書き込まれると先頭へ飛ぶため、ファームウェアを書き込んだあとにコアをリセットすると、ファームウェアが動き始める。
内蔵ブートローダの代わりにこの ROM を置く理由は、`third_party/neorv32_probe_rs/README.md` にある。

### JTAG

NEORV32 は TCK をコアのクロックで取り込む。
そのため TCK は、25 MHz の 1/5 である 5 MHz 以下にする。

J5 は 3.3 V に固定された Bank 2 にあるため、プローブの GPIO と直結できる。
NEORV32 の JTAG には TRST がないため、TCK、TMS、TDI、TDO の 4 本だけをつなぐ。

### ファームウェア

`firmware/` の `no_std` のプログラムは、UART に文字列を出し、LED に 2 進数のカウンタを表示する。
対象は、コアの命令セットに合わせて `riscv32imc-unknown-none-elf` にする。
起動処理は `riscv-rt` に任せる。
UART、GPIO、マシンタイマは、`crates/neorv32-hal` を通して使う。
LED の待ち時間はマシンタイマから求めるため、点滅の周期でクロックの設定を確かめられる。

### defmt のログ

ファームウェアは、defmt でログを RTT のバッファに書く。
`cargo run` で動く probe-rs が、JTAG からバッファを読んでログを表示する。
defmt はログの書式を ELF に残し、ターゲットからは書式の番号と値だけを送るため、命令メモリをあまり使わない。

NEORV32 のデバッグモジュールは、コアを止めずにメモリを読む機能を持たない。
そのため probe-rs は、バッファを読むたびにコアを止め、読み終えると再開する。
止まる時間はコアの動作に割り込むため、時間に厳しい処理を調べるときは気をつける。

パニックしたときは、場所をログに出して止まる。
パニックのメッセージは整形に `core::fmt` が要り、命令メモリを圧迫するため、出さない。
ログをどの深さまで出すかは、`firmware/.cargo/config.toml` の `DEFMT_LOG` で決める。
RTT のバッファは、満杯でも待たずに書くモードにしてあり、読まれていない古いログは上書きされる。
理由は、`firmware/Cargo.toml` の defmt-rtt のコメントにある。

### `cargo run` での書き込みに必要な変更

`cargo run` でファームウェアを書き込んで動かすために、次の変更をした。
どれか 1 つが欠けても、`cargo run` は通らない。

1. NEORV32 のデバッガを `OCD_EN` で有効にし、JTAG を J5 に出した。
2. IDCODE の製造元の欄に、Lattice の ID を `OCD_JEDEC_ID` で入れた。
3. 内蔵ブートローダを、`third_party/neorv32_probe_rs` の起動 ROM に差し替えた。
4. ブートローダがなくなったため、ファームウェアが UART の速度を自分で設定するようにした。
5. 命令メモリとデータメモリの位置を、`neorv32.yaml` で probe-rs に渡した。
   probe-rs に組み込まれた RISC-V のチップ定義には、このコアのメモリの配置がない。
6. `firmware/.cargo/config.toml` で、`probe-rs run` を cargo のランナーにした。
   probe-rs の設定は、同じファイルの `[env]` から環境変数で渡す。
7. `firmware/Cargo.toml` で、開発用のビルドでも大きさを最適化した。
   最適化しないと、ファームウェアが 16 KB の命令メモリに入りきらない。
8. probe-rs を 0.32.0 に上げた。

2、3、8 の理由は、`third_party/neorv32_probe_rs/README.md` にある。

## 検証

`make sim` では、次の 2 つを確かめる。

- リセット中は LED が消えていて、UART の送信線が 1 のままである。
- リセットを離したあと、JTAG で読んだ IDCODE が期待した値になる。

実機では、コンフィグ直後の空の命令メモリに `cargo run` で書き込み、UART に文字列が出ることを確かめた。
ファームウェアが動いている最中に続けて書き込んでも、書き込み直したファームウェアが最初から動くことを確かめた。
`cargo run` のあと、defmt のログが 250 ms ごとに欠けずに届くことを確かめた。

## 回路規模

LFD2NX-40-8BG256C で配置配線した結果を次に示す。

| 資源     | 使用量 | 総量   |
| -------- | ------ | ------ |
| LUT      | 4,726  | 32,256 |
| FF       | 1,913  | 32,256 |
| EBR      | 13     | 84     |
| 分散 RAM | 42     | 4,032  |
| I/O      | 16     | 111    |

SYSTEM_25M_CLK の最大周波数は 139.4 MHz で、25 MHz に対して十分な余裕がある。

合成のログには、ABC が出す `The network is combinational.` という警告が 1 件残る。
これは論理最適化の途中の情報で、回路の不具合を示すものではない。
nextpnr-nexus の警告はない。

## 実行方法

OSS CAD Suite と、Rust の RISC-V 向けのツールチェーン、probe-rs の 0.32.0 以降を用意する。

```sh
rustup target add riscv32imc-unknown-none-elf
```

UART を使うため、JP25 と JP26 を閉じて FTDI の Port B につなぐ。
この 2 つは既定で閉じている。
Port B は I²C と共用である。
UART になっているときは、LED の D27 が緑に点灯する。

FPGA のコンフィグは `make` で行う。

```sh
make        # 合成と配置配線、ビットストリームの生成
make sim    # テストベンチ
make load   # FPGA を SRAM にコンフィグする
make flash  # FPGA を SPI Flash にコンフィグする
```

ファームウェアは `firmware/` の中で `cargo run` を実行して書き込む。
probe-rs は書き込んだあとも接続を続け、defmt のログを表示する。
Ctrl+C で止めても、ファームウェアは動き続ける。

```sh
cd firmware
cargo run
```

プログラムの出力は、`/dev/ttyUSB1` に 19200 8N1 で出る。

### プローブの配線

プローブには、Raspberry Pi Pico か Pico 2 に rust-dap を書き込んだ CMSIS-DAP を使う。
rust-dap は、`--no-default-features --features jtag,set_clock` で JTAG 用にビルドする。

| J5 のピン | 信号 | Pico のピン |
| --------- | ---- | ----------- |
| 1         | TCK  | 4 (GPIO2)   |
| 2         | TMS  | 5 (GPIO3)   |
| 3         | TDI  | 9 (GPIO6)   |
| 4         | TDO  | 7 (GPIO5)   |
| 5         | GND  | 8 (GND)     |

ボードと Pico はそれぞれの USB から電源を取るため、3.3 V のピン同士はつながない。
J5 のピンと FPGA のボールの対応は、`datasheets/FPGA-EB-02032-1-2-Certus-NX-Versa-Evaluation-Board.md` にある。

### WSL でのプローブの設定

rust-dap の VID:PID は `6666:4444` である。
WSL では、Windows 側の usbipd でプローブを WSL に渡す。
`usbipd bind` は管理者権限で一度だけ実行し、`usbipd attach --wsl` はつなぐたびに実行する。

```sh
usbipd bind --hardware-id 6666:4444
usbipd attach --wsl --hardware-id 6666:4444
```

WSL 側では、udev のルールでプローブを `plugdev` グループから開けるようにする。
ルールはデバイスがつながったときに適用されるため、ルールを入れたあとに attach をやり直す。

```
SUBSYSTEM=="usb", ATTR{idVendor}=="6666", ATTR{idProduct}=="4444", MODE="0664", GROUP="plugdev"
```
