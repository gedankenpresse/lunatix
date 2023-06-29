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

    let arch_dir = PathBuf::from("src/arch/riscv64imac/");
    let link_dir = arch_dir.join("link");

    // Put the linker scripts somewhere the linker can find it
    println!("cargo:rustc-link-search={}", out_dir.display());
    for entry in fs::read_dir(link_dir).unwrap() {
        let entry = entry.unwrap();
        println!("cargo:rerun-if-changed={}", entry.path().display());
        fs::copy(entry.path(), out_dir.join(entry.file_name())).unwrap();
    }

    // set "-C link-arg=-Tlink.ldS" argument when linking to use the custom linker script
    println!("cargo:rustc-link-arg-bins=-Tlink.ldS");

    // compile raw assembly files
    let mut asm = arch_dir.clone();
    asm.push("asm");
    for entry in fs::read_dir(asm).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name().into_string().unwrap();
        let name = file_name.split(".").next().unwrap();
        println!("{}", name);
        println!("cargo:rerun-if-changed={}", entry.path().display());
        cc::Build::new()
            .file(entry.path())
            .flag("-no-pie")
            .flag("-fno-pic")
            .compiler("riscv64-elf-gcc")
            .target("riscv64imac")
            .compile(name);
    }
}
