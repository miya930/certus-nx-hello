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
必要な版と準備は、各プロジェクトの README.md の実行方法にある。

`tools/` のスクリプトは、uv で実行する。

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
