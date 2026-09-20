# LED の順次点灯デモ

Certus-NX Versa Evaluation Board の 8 個の汎用 LED を、1 つずつ順番に点灯させる。
ボードと開発環境の動作を確かめるための、最小のデモである。

## 構成

- 点灯する LED は 0.1 秒ごとに 1 つずつ移り、LED_7 の次は LED_0 に戻る。
- クロックは、ボードの 25 MHz の発振器 X2 から SYSTEM_25M_CLK として受ける。
- 押しボタン SW2 を押している間はリセットし、離すと LED_0 から始める。

| ファイル | 内容 |
|---|---|
| `led_sequence.vhd` | トップ |
| `led_sequence_tb.vhd` | テストベンチ |
| `led_sequence.pdc` | ピン割り当て |

## 設計

- 点灯を切り替える間隔は、クロックの周波数と切り替えの周期から計算する。
  テストベンチでは、この 2 つを小さくして短い時間で確認する。
- LED は、出力を 0 にすると点灯する。
- 押しボタンは、押している間だけ 0 になる。
  データシートの推奨に従い、ピンの設定でプルアップする。

## 検証

`make sim` のテストベンチで、次の動作を確認している。

- リセット中は LED_0 が点灯する。
- 点灯する LED は、常に 1 つだけである。
- 点灯は、切り替えの周期ぶんのサイクルごとに 1 つ隣へ移る。
- LED_7 の次は LED_0 に戻り、2 周とも同じ順番になる。
- 途中で押しボタンを押すと、LED_0 に戻る。

## 回路規模

LFD2NX-40-8BG256C で配置配線した結果を次に示す。

| 資源 | 使用量 |
|---|---|
| LUT | 48 |
| FF | 25 |
| I/O | 10 |

SYSTEM_25M_CLK の最大周波数は 352 MHz で、25 MHz に対して十分な余裕がある。

合成のログには、ABC が出す `The network is combinational.` という警告が 1 件残る。
これは論理最適化の内部の知らせで、この回路には順序回路も含まれている。

## 実行方法

yosys、GHDL、nextpnr-nexus を含む OSS CAD Suite が必要になる。

```sh
make        # 合成と配置配線
make sim    # テストベンチ
```

- `make` は、`build/report.json` に資源ごとの使用数と最大動作周波数を出力する。
- `make sim` は、全てのテストを通過して `All tests finished.` を表示し、終了コード 0 で終わる。
- ビットストリームの生成と書き込みは、次のコマンドで行う。
  実機での確認はまだしていない。

```sh
prjoxide pack build/led_sequence.fasm build/led_sequence.bit
openFPGALoader -b certusnx_versa_evn build/led_sequence.bit
```
