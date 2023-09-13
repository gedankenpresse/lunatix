use std::path::PathBuf;
use std::{env, fs};

extern crate cc;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed=src/asm/");
    let asm_dir = PathBuf::from("src/asm/");

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

        println!("cargo:rustc-link-search=native={}", out_dir);
    }
}
