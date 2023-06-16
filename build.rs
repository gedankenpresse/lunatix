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

    let arch_dir = PathBuf::from("src/arch/riscv64imac/");

    let mut link = arch_dir.clone();
    link.push("link");
    for entry in fs::read_dir(link).unwrap() {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }

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
            .compiler("riscv64-linux-gnu-gcc")
            .target("riscv64imac")
            .compile(name);
    }

    //println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-search=native={}", out_dir.display());

    println!("cargo:rerun-if-changed=build.rs");
}

