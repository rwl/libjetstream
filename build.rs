extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        // .with_language(Language::Cxx)
        // .with_include_guard("_JETSTREAM_H")
        // .with_item_prefix("JET_")
        // .with_config(Config {
        //     cpp_compat: true,
        //     ..Default::default()
        // })
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("jetstream.h");
}
