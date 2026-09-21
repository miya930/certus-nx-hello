# probe-rs の MT25Q の書き込みプログラム

probe-rs が、NEORV32 の SPI を通して、ボードの SPI Flash の MT25QU128 に書き込むためのプログラムである。
probe-rs は、これをデータメモリの `0x80000020` に置いて呼ぶ。
命令メモリに置かないのは、起動 ROM が命令メモリの先頭で Flash から写すかを決めるためである。

probe-rs は Flash を番地で扱うため、SPI Flash の 0 番地を `0x20000000` に見せる。
probe-rs に見せるのは、ファームウェアの像を置く `0x20F00000` からの 128 KB だけにする。
ビットストリームと保存した設定を、probe-rs が消さないようにするためである。

プログラムを変えたときは、次のように作り直し、出力の `flash_algorithms` の項目を各プロジェクトの `neorv32.yaml` に写す。
`target-gen elf --update` は、既存の YAML のコメントとデータメモリの範囲を落とすため使わない。
target-gen は、probe-rs と同じ版のものを使う。

```sh
cargo build --release
target-gen elf --fixed-load-address --name mt25q \
    build/riscv32imc-unknown-none-elf/release/mt25q-flash-algorithm build/mt25q.yaml
```
