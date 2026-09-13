#![cfg_attr(not(test), no_std)]

pub mod linalg;
pub mod mpc;
pub mod pid;
pub mod protocol;
pub mod thermistor;

pub mod model {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../build/firmware/generated.rs"));
}
