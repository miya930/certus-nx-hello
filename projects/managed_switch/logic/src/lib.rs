//! ファームウェアのうち、ハードウェアに触れない処理。
//! PC 上で単体テストするため、ファームウェアとは別のクレートにする。

#![cfg_attr(not(test), no_std)]

pub mod ports;
pub mod settings;
