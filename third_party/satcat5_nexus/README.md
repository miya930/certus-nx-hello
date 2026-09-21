# SatCat5 を Nexus で使うための層

`third_party/satcat5` の SatCat5 を、Certus-NX で合成してシミュレーションするための層である。
SatCat5 はプラットフォームごとの部品を外から与える作りになっており、ここに Nexus 向けの実装を置く。
`projects/` の複数のプロジェクトが、この層を共有する。

## 構成

| ディレクトリ | 内容 |
|---|---|
| `primitives/` | SatCat5 のプラットフォーム部品の Nexus 向けの実装 |
| `patches/` | ビルド時に SatCat5 に当てるパッチ |
| `sim/` | Nexus のプリミティブのシミュレーションモデル |

`primitives/` と `patches/` は、合成にもシミュレーションにも使う。
`sim/` はシミュレーションでだけ使い、合成には渡さない。

## プラットフォーム部品

`common_primitives_body.vhd` は、SatCat5 の common_primitives パッケージの本体である。
FIFO をメモリ方式にし、MAC テーブルの TCAM の LUT 幅を 6 にする。
シフトレジスタ方式の FIFO は LFD2NX-40 に収まらず、LUT 幅 8 では MAC アドレスを重複して登録したためである。

`clk_input.vhd`、`ddr_input.vhd`、`ddr_output.vhd`、`dpram.vhd` は、SatCat5 が求める部品を Nexus のプリミティブで実装する。

`sb_dffr.vhd` は、iCE40 の SB_DFFR と同じ端子を持つフリップフロップを Nexus のプリミティブで用意する。
SatCat5 の Lattice 向けの同期回路 `ice40_sync.vhd` は、SB_DFFR 以外に iCE40 固有の記述を持たない。
このフリップフロップがあれば、同期回路をそのまま使える。

## パッチ

パッチはビルドのたびに `build/patched/` へ当てたコピーを作り、元のファイルの代わりに合成へ渡す。
SatCat5 の作業ツリーは書き換えない。

- `switch_core.vhd.patch` は、シミュレーションで同じエッジのデータを取り込まないよう、ポートのクロックを直接渡す。
- `port_rmii.vhd.patch` は、受信データに入力遅延を入れる設定を追加する。
- `port_rgmii.vhd.patch` は、送信クロックの分周にある剰余の演算を、比較と条件分岐で書き換える。
  剰余の演算は除算器になり、125 MHz に届かないためである。
- `packet_round_robin.vhd.patch` は、関数の引数の名前をポートの名前と重ならないように変える。
  重なった名前は GHDL が隠れた名前の警告として出すためで、動きは変わらない。
- `io_mdio_readwrite.vhd.patch` と `cfgbus_mdio.vhd.patch` は、MDIO のデータ線を入力、出力、出力の許可の 3 本に分けて外に出す。
  GHDL で合成すると、下の階層を通る双方向のポートはトップのポートとのつながりが切れるためである。
  3 状態の出力は、プロジェクトのトップで書く。
- `eth_statistics.vhd.patch` は、周波数を報告するレジスタの初期値を、非同期リセットと同じ 0 にする。
  Nexus のフリップフロップは、初期値と非同期リセットの値が違うものを表せず、合成が止まるためである。
  リセットの後はどちらも 0 になるため、動きは変わらない。
  あわせて、エラーの数の 2 つの誤りを直す。
  1 つ目は、統計のクロックで数えるエラーの数を、送信のクロックの取り込みの合図で 0 に戻していたことである。
  2 つのクロックが違うと合図を取りこぼし、エラーの数が 0 に戻らないか、取り込む前に消えていた。
  2 つ目は、パケットのエラーの数が、MAC と PHY のエラーを数えていたことである。
  直した動きは、同じフォルダの `eth_statistics_tb.vhd` で確かめる。
  このテストベンチは、`projects/managed_switch` の `make sim` で動く。

## シミュレーションモデル

`sim/` には、IDDRX1、ODDRX1、DELAYA、FD1P3DX のモデルを置く。
Nexus のプリミティブは VHDL の実体を持たないため、合成ではブラックボックスとして扱う。
シミュレーションでは、このモデルで置き換える。
モデルは技術ノートに書かれた範囲で作っており、出力の遅延は実機と異なる可能性がある。

## 使い方

プロジェクトの Makefile から、次のように参照する。

```make
NEXUS   := ../../third_party/satcat5_nexus
PATCHES := $(NEXUS)/patches
```

合成に渡すファイルには `$(NEXUS)/primitives/*.vhd` を含める。
シミュレーションでは、これに加えて `$(NEXUS)/sim/*.vhd` を含める。
