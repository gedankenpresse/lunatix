use std::fs;
use std::path::PathBuf;

extern crate cc;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let arch_dir = PathBuf::from("src/arch/riscv64imac/");

    // compile raw assembly files
    let asm_dir = arch_dir.join("asm");
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
