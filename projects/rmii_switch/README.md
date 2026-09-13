# rmii_switch

## 何を確認するか

SatCat5 の RMII スイッチを LFD2NX-40 向けに合成と配置配線を行い、回路規模と動作周波数を確認する。
スイッチは VLAN と PTP を有効にし、どの RMII ポートからでも Ethernet フレームで ConfigBus を操作して設定できる。
テストベンチでは、RMII のフレーム転送、ConfigBus の読み書き、VLAN、PTP の動作を確認する。
ボードに合わせたピン割り当てをしていないため、まだ実機には書き込めない。

## 必要な機材

PC だけで確認できる。
yosys、GHDL、nextpnr-nexus を含む OSS CAD Suite が必要になる。

## 実行方法

```sh
git submodule update --init
make PORT_COUNT=4   # 合成と配置配線
make sim            # テストベンチ
```

## 期待する動作

`make PORT_COUNT=4` は、`build/ports4/report.json` に資源ごとの使用数と、クロックごとの最大動作周波数を出力する。
REF_CLK のクロック網は、合成時の名前付けにより report.json では `cfg_cmd[0]` と表示される。
このクロックの最大動作周波数が 50 MHz を上回る。

`make sim` は、全てのテストを通過して `All tests finished.` を表示し、終了コード 0 で終わる。
