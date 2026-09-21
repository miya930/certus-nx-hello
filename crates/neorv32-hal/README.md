# NEORV32 の周辺の HAL

NEORV32 に内蔵の周辺を、embedded-hal と embedded-io の trait で使うためのクレートである。
レジスタは `crates/neorv32-pac` の型を通して読み書きする。

次の周辺を扱う。

| モジュール | 周辺 | 実装する trait |
|---|---|---|
| `uart` | UART0 | embedded-io の `Read`、`ReadReady`、`Write` |
| `spi` | SPI | embedded-hal の `SpiDevice` |
| `mtime` | CLINT のマシンタイマ | embedded-hal の `DelayNs` |
| `gpio` | GPIO | なし |

UART、SPI、マシンタイマの `new` は、コアのクロック周波数を受け取り、分周比や時間をそこから計算する。
使う周辺は、各プロジェクトの FPGA の設計で有効にしておく。
