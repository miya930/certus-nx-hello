# certus-nx-hello

Certus-NX Versa Evaluation Board で、SatCat5 のデモと制御のデモを行うためのリポジトリである。
Lattice Radiant は使わず、オープンソースのツールだけで合成からボードへの書き込みまで行う。
設計情報は https://miya930.github.io/certus-nx-hello/ で公開している。

## 準備

ツールは OSS CAD Suite でそろえる。
展開したら、`source <展開先>/environment` を実行してツールにパスを通す。

## 構成

| ディレクトリ | 内容 |
|---|---|
| `projects/` | デモごとのプロジェクトを置く。実行方法は各プロジェクトの README.md に書いてある。 |
| `third_party/` | SatCat5 などの外部 HDL を git submodule として置く。 |
| `datasheets/` | ボード、FPGA、部品のデータシートを PDF から Markdown に変換して置く。 |
| `tools/` | データシートを変換するスクリプトを置く。uv で実行する。 |
| `site/` | 設計情報を公開するサイトを置く。README.md と `projects/`、`docs/` のドキュメントを集めて GitHub Pages に公開する。 |

開発のルールは `CLAUDE.md` にまとめている。
