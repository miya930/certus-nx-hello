# IP アドレスを持つ Ethernet の端点

LFD2NX-40 に SatCat5 のスイッチと NEORV32 の RISC-V コアを実装し、ボードに IP アドレスを持たせる。
IP の処理は Rust の smoltcp が行い、PC から `ping` が通ることを目標にする。

## 構成

```
PHY ── port_rmii ─┬─ switch_core ── port_mailmap ── NEORV32 ── smoltcp
                  └─ (ポートを足せる)
```

- スイッチのポートは、PHY につながる RMII と、CPU につながる mailmap の 2 つである。
- `port_mailmap` は、フレーム全体をメモリに見せる仮想の内部ポートである。
  CPU から見ると、受信も送信も配列の読み書きになる。
- CPU、スイッチコア、ConfigBus を全て REF_CLK の 50 MHz で動かし、クロックの乗り換えをなくす。
- 押しボタン SW2 を押している間はリセットする。
- LED の下位 7 ビットに受け取ったフレーム数を出し、最上位を毎秒反転させる。
- UART は、ブートローダとプログラムの出力に使う。

| ファイル | 内容 |
|---|---|
| `ip_endpoint.vhd` | トップ |
| `ip_endpoint_tb.vhd` | テストベンチ |
| `ip_endpoint.pdc` | ピン割り当て |
| `firmware/` | コアで動かす Rust のプログラム |

## 設計

### CPU とスイッチのつなぎ方

NEORV32 の外部バスは Wishbone なので、SatCat5 の `cfgbus_host_wishbone` に直結できる。
変換回路は要らない。

アドレスの下位 18 ビットが、ConfigBus のデバイス番号 8 ビットとレジスタ番号 10 ビットになる。
CPU からは次の位置に見える。

| ConfigBus のデバイス | CPU から見たアドレス |
|---|---|
| 0 (`switch_core`) | `0x90000000` + レジスタ番号 × 4 |
| 1 (`port_mailmap`) | `0x90001000` + レジスタ番号 × 4 |

NEORV32 は IMEM、DMEM、IO のどれにも当たらないアドレスを外部バスへ出すため、`0x90000000` を使える。

`cfgbus_host_wishbone` はバイトイネーブルを持たない。
フレームはワード単位で読み書きし、バイトへの詰め替えは CPU 側で行う。

### VHDL の版

NEORV32 は VHDL-2008 で書かれ、自身を `neorv32` ライブラリに置くことを前提にしている。
SatCat5 も同じライブラリに入れると、両者の `work` 参照が同じ場所を指す。

SatCat5 は VHDL-93 を想定して書かれているが、`mac_log_core.vhd` の 1 箇所を除いて VHDL-2008 で解析できる。
その 1 箇所は `third_party/satcat5_nexus/patches/` のパッチで直す。

### ファームウェア

`firmware/` に `no_std` の Rust のプログラムを置く。

- `mailmap.rs` が `port_mailmap` を smoltcp の `Device` として扱う。
- ソケットは開かない。ARP と ICMP の応答は smoltcp が IP の層で処理する。
- UART の速度はブートローダが設定した値をそのまま使い、制御レジスタを書き換えない。
- 時刻はマシンタイマの 64 ビットの値から求める。

smoltcp はソケットを 1 つも有効にしないと組み上がらないため、`socket-icmp` を有効にする。

### アドレス

| 項目 | 値 |
|---|---|
| MAC アドレス | `5A:5A:00:00:00:02` |
| IP アドレス | `192.168.1.10/24` |

どちらもローカル管理のアドレスで、`firmware/src/main.rs` に書いている。

## 検証

`make sim` のテストベンチで、次の動作を確認している。

- リセット中は LED が全て消灯し、UART の送信線が 1 のまま、RMII が送信していない。
- リセットを離すと、ブートローダが UART へ送信を始める。
  CPU がスイッチコアと ConfigBus を抱えた構成でも動き出すことの確認になる。

実機での確認はまだしていない。
ボードの DP83867 が未実装の個体があるため、PHY を用意してからになる。

## 回路規模

LFD2NX-40-8BG256C で配置配線した結果を次に示す。

| 資源 | 使用量 | 総量 |
|---|---|---|
| LUT | 9,505 | 32,256 |
| FF | 5,049 | 32,256 |
| EBR | 72 | 84 |
| 分散 RAM | 82 | 4,032 |
| I/O | 19 | 111 |

EBR が一番きつい。
命令メモリ 32 KB とデータメモリ 32 KB で 72 個を使う。
これ以上メモリを増やすなら、スイッチの出力バッファを 8 KB から減らす調整が要る。

ConfigBus のクロック網の最大周波数は 102 MHz で、50 MHz に対して余裕がある。

合成のログには、ABC が出す `The network is combinational.` という警告が 1 件残る。
これは論理最適化の内部の知らせで、この回路には順序回路も含まれている。
nextpnr-nexus の警告はない。

## 実行方法

yosys、GHDL、nextpnr-nexus を含む OSS CAD Suite と、Rust のツールチェーンが必要になる。

```sh
rustup target add riscv32imc-unknown-none-elf
rustup component add llvm-tools
```

```sh
make        # 合成と配置配線、ビットストリームの生成
make sim    # テストベンチ
make load   # FPGA を SRAM にコンフィグする
make upload # ファームウェアを UART から流し込んで実行する
```

`make upload` が使うシリアルポートは `PORT` で変えられる。
