use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

fn main() {
    println!("cargo:warning=CARGO_CFG_FEATURE={}", env::var("CARGO_CFG_FEATURE").unwrap());
    println!("cargo:warning=CARGO_FEATURE_RP235XA={}", env::var("CARGO_FEATURE_RP235XA").unwrap());

    #[cfg(all(feature = "rp2040", feature = "rp235xa"))]
    compile_error!("feature \"rp2040\" and feature \"rp235xa\" must not both be specified");
    #[cfg(not(any(feature = "rp2040", feature = "rp235xa")))]
    compile_error!("one or the other of feature \"rp2040\" and \"rp235xa\" must be specified");


    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out.display());

    let memory_x = if env::var("CARGO_FEATURE_RP235XA").is_ok() {
        include_str!("../mcaux-app/memory-rp235xa.x")
    } else {
        include_str!("../mcaux-app/memory-rp2040.x")
    };

    // Adjust section names for bootloader
    let memory_x = memory_x.replace("LENGTH(FLASH)", "LENGTH(ACTIVE)");
    let memory_x = memory_x.replace("ORIGIN(FLASH)", "ORIGIN(ACTIVE)");
    let memory_x = memory_x.replacen("FLASH", "ACTIVE", 1);
    let memory_x = memory_x.replace(" LOADER ", " FLASH ");

    let fname = out.join("memory.x");
    println!("cargo:warning=filename: {:?}", fname);
    let mut f = File::create(fname).unwrap();
    f.write_all(memory_x.as_bytes()).unwrap();

    if env::var("CARGO_FEATURE_RP235XA").is_ok() {
        println!("cargo:rerun-if-changed=../mcaux-app/memory-rp235xa.x");
    } else {
        println!("cargo:rerun-if-changed=../mcaux-app/memory-rp2040.x");
    }

    println!("cargo:rerun-if-changed=./build.rs");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    if env::var("CARGO_FEATURE_DEFMT").is_ok() {
        println!("cargo:warning=Linking with defmt");
        println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    }
    if env::var("CARGO_FEATURE_RP2040").is_ok() {
        println!("cargo:warning=Linking with link-rp.x");
        println!("cargo:rustc-link-arg-bins=-Tlink-rp.x");
    }
}
