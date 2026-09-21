use std::{env, fs, path::Path};

const SVD: &str = "satcat5.svd";
const PERIPHERALS_START: &str = "# [no_mangle] static mut DEVICE_PERIPHERALS";

fn main() {
    println!("cargo:rerun-if-changed={SVD}");
    let svd = fs::read_to_string(SVD).expect("satcat5.svd is missing");
    let mut config = svd2rust::Config::default();
    config.target = svd2rust::Target::None;
    config.make_mod = true;
    let generation = svd2rust::generate(&svd, &config).expect("svd2rust failed");
    // 生成したコードは先頭に説明文の属性を持ち、include! で取り込むと inner attribute として拒まれる。
    // 説明文は `"]` で終わるため、そこまでを取り除く。
    let code = generation.lib_rs;
    let code = match code.strip_prefix("# ! [doc = \"") {
        Some(rest) => rest.split_once("\"]").expect("unterminated doc attribute").1,
        None => &code,
    };
    // 末尾の Peripherals は、SVD の仮のアドレス 0 にデバイスを置き、アドレスを与える手段を持たない。
    // その所有を記録する DEVICE_PERIPHERALS は no_mangle で、neorv32-pac の同名の記号とリンクで衝突する。
    // アドレスは各プロジェクトが与えるため、ここから後ろを取り除く。
    let code = code.split_once(PERIPHERALS_START).expect("Peripherals is missing").0;
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("pac.rs");
    fs::write(out, code).unwrap();
}
