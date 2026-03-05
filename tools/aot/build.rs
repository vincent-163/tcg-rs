use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Only compile helpers when llvm feature is enabled
    if cfg!(feature = "llvm") {
        compile_helpers();
    }
}

fn compile_helpers() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Create output directory for helpers
    let helpers_dir = PathBuf::from(&out_dir).join("helpers");
    std::fs::create_dir_all(&helpers_dir).unwrap();

    // Path to the C source file
    let helpers_c = PathBuf::from(&manifest_dir)
        .join("helpers")
        .join("aarch64_helpers.c");

    // Output bitcode file
    let helpers_bc = helpers_dir.join("aarch64_helpers.bc");

    println!("cargo:rerun-if-changed={}", helpers_c.display());

    // Check if clang is available
    let clang = which_clang();
    if clang.is_none() {
        eprintln!("Warning: clang not found, skipping helper bitcode compilation");
        eprintln!("AOT will use external helper symbols instead of inlined bitcode");
        // Set a dummy path so the code compiles
        println!("cargo:rustc-env=HELPERS_BC_PATH=");
        return;
    }

    let clang = clang.unwrap();

    // Compile to LLVM bitcode
    let status = Command::new(&clang)
        .args(&[
            "-c",
            "-emit-llvm",
            "-O3",
            "-target", "x86_64-unknown-linux-gnu",
            "-o", helpers_bc.to_str().unwrap(),
            helpers_c.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute clang");

    if !status.success() {
        panic!("Failed to compile helper bitcode");
    }

    // Write the bitcode path for include_bytes! macro
    let include_path = helpers_bc.to_str().unwrap().replace('\\', "/");
    println!("cargo:rustc-env=HELPERS_BC_PATH={}", include_path);
    eprintln!("Helper bitcode compiled to: {}", helpers_bc.display());
}

fn which_clang() -> Option<String> {
    // Try common clang names
    for name in &["clang", "clang-21", "clang-20", "clang-19", "clang-18"] {
        if Command::new(name)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Some(name.to_string());
        }
    }
    None
}
