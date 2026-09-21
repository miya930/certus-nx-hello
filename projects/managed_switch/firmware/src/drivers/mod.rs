//! ボードと FPGA の中のデバイスを、デバイスごとに 1 つの型で扱うドライバ。
//! レジスタは、NEORV32 の周辺なら neorv32-hal を、SatCat5 なら satcat5-pac を通して読み書きする。

pub mod clock;
pub mod dp83867;
pub mod leds;
pub mod switch;
pub mod terminal;
