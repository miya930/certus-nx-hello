# certus-nx-hello

Certus-NX Versa Evaluation Board で、SatCat5 のデモと制御のデモを行うためのリポジトリである。
Lattice Radiant は使わず、オープンソースのツールだけで合成からボードへの書き込みまで行う。

## プロジェクト

| プロジェクト                                                   | 内容                                        |
| -------------------------------------------------------------- | ------------------------------------------- |
| [`projects/led`](projects/led/README.md)                       | LED による 2 進カウンタ                     |
| [`projects/riscv_rust`](projects/riscv_rust/README.md)         | RISC-V ソフトコアで動く Rust ファームウェア |
| [`projects/managed_switch`](projects/managed_switch/README.md) | SatCat5 と NEORV32 によるマネージドスイッチ |

設計と実行方法は、各プロジェクトの README.md にまとめている。

## 準備

SatCat5 と NEORV32 は git submodule なので、clone したあとに取ってくる。

```sh
git submodule update --init
```

FPGA のツールは OSS CAD Suite でそろえる。
展開したら、`source <展開先>/environment` を実行してツールにパスを通す。

ファームウェアを持つプロジェクトは、Rust と probe-rs も使う。
ファームウェアの準備は、各プロジェクトの README.md の実行方法にある。

`tools/` のスクリプトは、uv で実行する。

## 動作を確かめた環境

次の環境で、全てのプロジェクトの合成から FPGA のコンフィグまでと、ファームウェアの書き込みと実行を確かめた。

| ツール        | 版       |
| ------------- | -------- |
| OSS CAD Suite | 20260923 |
| Rust          | 1.96.0   |
| probe-rs      | 0.32.0   |
| uv            | 0.8.17   |
| WSL           | 2.4.12.0 |
| usbipd-win    | 5.2.0    |

OSS CAD Suite の 20260328 版の nextpnr-nexus は、I/O の遅延素子を通した入力を I/O レジスタに入れられない。
そのため、RMII の受信で遅延素子を使う `projects/managed_switch` は、配置配線で失敗する。

yosys の版が変わると、合成で付くクロックの名前が変わることがある。
名前が変わると、`projects/managed_switch/tools/check_timing.py` がクロックを見分けられずに止まるため、表を直す。

probe-rs の 0.30.0 と 0.31.0 では、NEORV32 に接続できない。
その理由は、`third_party/neorv32_probe_rs/README.md` にある。

WSL は NAT のネットワークで使った。
このとき、ボードにつないだ Ethernet のインターフェースは Windows が持つため、PC のアドレスは Windows 側で設定する。
USB のプローブと、ボードの FTDI は、Windows の usbipd で WSL に渡す。
プローブを渡す手順は、`projects/riscv_rust/README.md` にある。
ボードの FTDI も、同じ手順で `0403:6010` を指定して渡す。

## 構成

| ディレクトリ        | 内容                                                                                                                                                                   |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `projects/`         | デモごとのプロジェクトを置く。                                                                                                                                         |
| `crates/`           | 複数のプロジェクトのファームウェアで共有する Rust のクレートを置く。                                                                                                   |
| `flash_algorithms/` | probe-rs が SPI Flash に書くときに、NEORV32 で動かす書き込みプログラムを置く。                                                                                         |
| `third_party/`      | SatCat5 と NEORV32 を git submodule として置く。これらを Nexus と probe-rs で使うための部品は、隣の `satcat5_nexus/` と `neorv32_probe_rs/` に置く。                   |
| `tools/`            | ビルドと書き込みで使うスクリプトと、データシートを変換するスクリプトを置く。                                                                                           |
| `datasheets/`       | ボード、FPGA、部品のデータシートを PDF から Markdown に変換して置く。再配布にあたるため、コミットしない。変換の手順は `.claude/skills/add-datasheet/SKILL.md` にある。 |

開発のルールは `CLAUDE.md` にまとめている。
