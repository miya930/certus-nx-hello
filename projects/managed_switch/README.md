# IP で管理できるスイッチングハブ

SatCat5 のスイッチに NEORV32 の RISC-V コアをつなぎ、スイッチ自身が IP アドレスを持つようにする。
PC から `ping` を送ると、コアで動く Rust の smoltcp が応答する。

## 構成

![managed_switch の内部構成](doc/managed_switch_block.drawio.svg)

スイッチのポートは 2 つある。
1 つはボードの DP83867 に RGMII でつながり、もう 1 つは CPU につながる `port_mailmap` である。
`port_mailmap` はフレーム全体をメモリとして見せるため、CPU はフレームを配列として読み書きできる。

HDL と制約は `hdl/`、ファームウェアは `firmware/`、ビルドで使うスクリプトは `tools/`、図は `doc/` に置く。

## 設計

### クロックとリンク速度

RGMII のポートは、PHY に合わせて 125 MHz で動く。
スイッチコアと CPU は、配置配線の結果が 125 MHz に届かないため、25 MHz で動かす。
両者の間では、SatCat5 のポートがクロックを乗り換える。

25 MHz のスイッチコアは 200 Mbps までしか扱えない。
そこで起動時に MDIO で DP83867 を設定し、1000BASE-T を広告させずに 100BASE-TX でリンクさせる。
RGMII のクロックのずれも同じ MDIO の書き込みで PHY に作らせ、FPGA 側では遅延を入れない。

### CPU とスイッチのつなぎ方

NEORV32 の外部バスは Wishbone なので、`cfgbus_host_wishbone` を通して ConfigBus を操作できる。
CPU からは、スイッチコアが `0x90000000` から、`port_mailmap` が `0x90001000` から見える。

NEORV32 は、応答をストローブの次のサイクルから受け付ける。
ブリッジは書き込みの応答をストローブと同じサイクルに返すため、応答を 1 サイクル遅らせて CPU に渡す。

ブリッジはバイトイネーブルを持たないため、ファームウェアはフレームをワード単位で読み書きする。

### ファームウェア

ファームウェアは `port_mailmap` を smoltcp の `Device` として扱う。
ARP と ICMP の echo には smoltcp が応答するため、ソケットは開かない。
MAC アドレスと IP アドレスは `firmware/src/main.rs` に書いている。

## 検証

`make sim` では、PHY のリセットの解除、MDIO の書き込み、ブートローダの起動を確かめる。

実機では PC と RJ45 を直結し、`ping` が返ることを確かめた。

## 回路規模

LFD2NX-40-8BG256C で配置配線した結果を次に示す。

| 資源 | 使用量 | 総量 |
|---|---|---|
| LUT | 9,762 | 32,256 |
| FF | 5,156 | 32,256 |
| EBR | 66 | 84 |
| 分散 RAM | 82 | 4,032 |
| I/O | 28 | 111 |

タイミングの余裕が一番少ないのは RGMII の送信側で、125 MHz に対して 140.2 MHz である。
スイッチコアの出力バッファを 8 KB にすると、ここが 112 MHz に落ちるため、2 KB にしている。

nextpnr-nexus は、設計全体に 1 つの目標周波数しか与えられない。
そのため 25 MHz の回路も 125 MHz で評価され、その警告が 1 件残る。
クロックごとの要求は `tools/check_timing.py` で確かめる。

合成のログには、ABC が出す `The network is combinational.` という警告が 1 件残る。
これは論理最適化の内部の知らせで、回路の不具合を示すものではない。

## 実行方法

OSS CAD Suite と、Rust の RISC-V 向けのツールチェーンを用意する。

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

`make upload` のシリアルポートは、`make upload PORT=/dev/ttyUSB0` のように変えられる。

PC 側には、基板の `192.168.1.10/24` と同じ範囲の固定のアドレスを割り当てる。

```sh
sudo ip addr add 192.168.1.1/24 dev <インターフェース名>
```

LED0 が点灯すれば、PHY の設定は終わっている。
その後に `ping 192.168.1.10` を送ると、応答が返る。
