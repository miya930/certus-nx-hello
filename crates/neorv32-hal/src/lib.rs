//! NEORV32 に内蔵の周辺を、embedded-hal と embedded-io の trait で使うための層。
//! 周辺のレジスタは、NEORV32 の SVD から生成した neorv32-pac を通して読み書きする。

#![no_std]

pub use neorv32_pac as pac;

pub mod gpio;
pub mod mtime;
pub mod spi;
pub mod uart;
