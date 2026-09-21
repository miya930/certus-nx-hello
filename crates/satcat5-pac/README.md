# SatCat5 の ConfigBus のデバイスのレジスタの型

SatCat5 の ConfigBus のデバイスのレジスタを、型付きで読み書きするためのクレートである。
型は、ビルドのときに `satcat5.svd` から svd2rust で生成し、NEORV32 の `crates/neorv32-pac` と同じ形で使える。

SatCat5 には SVD がないため、`satcat5.svd` は各デバイスの VHDL の先頭のコメントにあるレジスタの一覧から書いた。
SVD には、このリポジトリのファームウェアが使うデバイスとレジスタだけを書く。

デバイスのアドレスは、ConfigBus を CPU のどこに置くかと、デバイス番号によって、プロジェクトごとに変わる。
そのため SVD のアドレスは 0 にしておき、プロジェクトがアドレスを `RegisterBlock` のポインタにして使う。
アドレス 0 にデバイスを置いた `Peripherals` は、生成したコードから取り除く。
