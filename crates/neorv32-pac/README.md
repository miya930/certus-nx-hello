# NEORV32 の周辺のレジスタの型

NEORV32 の周辺のレジスタを、型付きで読み書きするためのクレートである。
型は、ビルドのときに `third_party/neorv32/sw/svd/neorv32.svd` から svd2rust で生成する。
そのため、ビルドの前に `git submodule update --init` で NEORV32 を取ってくる。

SVD は NEORV32 の版に合わせて submodule の中にあるため、生成したコードはコミットしない。
ファームウェアからは、このクレートを直接使わず、`crates/neorv32-hal` を通して使う。
