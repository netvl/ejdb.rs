extern crate bindgen;
extern crate cmake;
extern crate pkg_config;

use cmake::Config;
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    pkg_config::Config::new().probe("zlib").unwrap();

    let dst = Config::new("ejdb-upstream")
        .cflag("-w")
        .profile("Release")
        .define("BUILD_SAMPLES", "OFF")
        .define("BUILD_SHARED_LIBS", "OFF")
        .build();

    Command::new("make").status().expect("failed to make!");

    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib64").display()
    );
    println!("cargo:rustc-link-lib=static=ejdb-1");

    let bindings = bindgen::Builder::default()
        .header(dst.join("include/ejdb/ejdb.h").as_path().to_str().unwrap())
        // Hide duplicated types
        .blacklist_item("FP_NAN")
        .blacklist_item("FP_INFINITE")
        .blacklist_item("FP_ZERO")
        .blacklist_item("FP_SUBNORMAL")
        .blacklist_item("FP_NORMAL")
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
