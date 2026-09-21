use std::{env, fs, path::Path};

const SVD: &str = "../../third_party/neorv32/sw/svd/neorv32.svd";

fn main() {
    println!("cargo:rerun-if-changed={SVD}");
    let svd = fs::read_to_string(SVD).expect("NEORV32 SVD is missing; run git submodule update --init");
    // SVD の cpu の項目は svd-parser が求めるタグを持たず、読み込みが止まる。
    // PAC の生成には CPU の情報を使わないため、項目ごと取り除いてから渡す。
    let (head, rest) = svd.split_once("<cpu>").expect("SVD has no cpu element");
    let (_, tail) = rest.split_once("</cpu>").expect("SVD cpu element is not closed");
    let input = [head, tail].concat();
    let mut config = svd2rust::Config::default();
    config.target = svd2rust::Target::None;
    config.make_mod = true;
    let generation = svd2rust::generate(&input, &config).expect("svd2rust failed");
    // 生成したコードは先頭に説明文の属性を持ち、include! で取り込むと inner attribute として拒まれる。
    // 説明文は `"]` で終わるため、そこまでを取り除く。
    let code = generation.lib_rs;
    let code = match code.strip_prefix("# ! [doc = \"") {
        Some(rest) => rest.split_once("\"]").expect("unterminated doc attribute").1,
        None => &code,
    };
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("pac.rs");
    fs::write(out, code).unwrap();
}
