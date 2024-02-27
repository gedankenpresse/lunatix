#![allow(unused_imports)]

use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

extern crate cc;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("No out dir"));
    let _name = env::var("CARGO_PKG_NAME").unwrap();
    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "riscv64" {
        compile_asm()
    }

    // Put the linker scripts somewhere the linker can find it
    let link_dir = PathBuf::from("src/arch/link");
    println!("cargo:rustc-link-search={}", out_dir.display());
    for entry in fs::read_dir(link_dir).unwrap() {
        let entry = entry.unwrap();
        println!("cargo:rerun-if-changed={}", entry.path().display());
        fs::copy(entry.path(), out_dir.join(entry.file_name())).unwrap();
    }

    // set "-C link-arg=-Tlink.ldS" argument when linking to use the custom linker script
    println!("cargo:rustc-link-arg-bins=-Tlink.ldS");
}

fn compile_asm() {
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", out_dir);

    let asm_dir = PathBuf::from("src/arch/riscv64imac/asm");
    println!("cargo:rerun-if-changed=src/asm/");

    for file in fs::read_dir(asm_dir).unwrap() {
        let file = file.unwrap();
        let file_name = file.file_name().into_string().unwrap();
        let name = file_name.split(".").next().unwrap();
        println!("cargo:rerun-if-changed={}", file.path().display());
        cc::Build::new()
            .file(file.path())
            .flag("-no-pie")
            .flag("-fno-pic")
            .compiler("riscv64-elf-gcc")
            .target("riscv64imac")
            .compile(name);
    }
}
