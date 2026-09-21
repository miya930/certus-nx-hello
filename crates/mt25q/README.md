# SPI Flash の MT25Q のドライバ

ボードの SPI Flash の Micron MT25QU128 を、embedded-hal の `SpiDevice` の上で読み、消し、書く。
管理できるスイッチのファームウェアが設定の保存に使い、probe-rs の Flash の書き込みプログラムと起動 ROM がファームウェアの読み書きに使う。
