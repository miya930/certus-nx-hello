# NEORV32 の起動 ROM

NEORV32 の起動 ROM に置くプログラムである。
命令メモリが空なら、SPI Flash の firmware image を命令メモリにコピーしてから、命令メモリの先頭へ飛ぶ。
動作と、`neorv32_bootrom_image.vhd` の作り方は、`third_party/neorv32_probe_rs/README.md` にある。

起動 ROM は ROM なので、初期値のある静的変数を持てない。
`link.x` は、そのような変数があるとリンクを止める。
スタックはデータメモリの先頭の 1 KB に置き、ファームウェアが動き始めると使わなくなる。
