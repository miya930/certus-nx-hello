# certus-nx-hello

Certus-NX Versa Evaluation Board で、SatCat5 のデモと制御のデモを行うためのリポジトリである。
Lattice Radiant は使わず、オープンソースのツールだけで合成からボードへの書き込みまで行う。

## 準備

ツールは OSS CAD Suite でそろえる。
展開したら、`source <展開先>/environment` を実行してツールにパスを通す。

## 構成

| ディレクトリ | 内容 |
|---|---|
| `projects/` | デモごとのプロジェクトを置く。設計と実行方法は、各プロジェクトの README.md にまとめている。 |
| `hw/` | デモで使う基板を置く。仕様は、各基板の README.md にまとめている。 |
| `third_party/` | SatCat5 などの外部 HDL を git submodule として置く。外部 HDL を Nexus で使うための層も、submodule の隣に置く。 |
| `datasheets/` | ボード、FPGA、部品のデータシートを PDF から Markdown に変換して置く。再配布にあたるため、コミットしない。 |
| `tools/` | データシートを変換するスクリプトを置く。uv で実行する。 |

開発のルールは `CLAUDE.md` にまとめている。
