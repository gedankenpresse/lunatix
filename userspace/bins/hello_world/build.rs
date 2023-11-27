use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

extern crate cc;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("No out dir"));
    let _name = env::var("CARGO_PKG_NAME").unwrap();
    println!("cargo:rerun-if-changed=build.rs");

    let link_dir = PathBuf::from("src/arch/link");

    // Put the linker scripts somewhere the linker can find it
    println!("cargo:rustc-link-search={}", out_dir.display());
    for entry in fs::read_dir(link_dir).unwrap() {
        let entry = entry.unwrap();
        println!("cargo:rerun-if-changed={}", entry.path().display());
        fs::copy(entry.path(), out_dir.join(entry.file_name())).unwrap();
    }

    println!("cargo:rerun-if-changed=src/lunatix.manifest");
    assert!(Command::new("riscv64-elf-objcopy")
        .args([
            "--strip-all",
            "-Ibinary",
            "-Oelf64-littleriscv",
            "--rename-section",
            ".data=.lunatix_manifest,contents",
            "src/lunatix.manifest",
            out_dir.join("lunatix_manifest.o").to_str().unwrap(),
        ])
        .output()
        .expect("Could not compile lunatix.manifest into object file")
        .status
        .success());

    // set "-C link-arg=-Tlink.ldS" argument when linking to use the custom linker script
    println!("cargo:rustc-link-arg-bins=-Tlink.ldS");
    println!("cargo:rustc-link-lib=static:+verbatim=lunatix_manifest.o")
}
