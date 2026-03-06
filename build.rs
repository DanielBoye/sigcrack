fn main() {
    println!("cargo:rerun-if-changed=csrc");
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("x86_64") {
        return;
    }
    cc::Build::new()
        .file("csrc/keccak_avx512.c")
        .flag("-mavx512f")
        .opt_level(3)
        .compile("keccak_avx512");
}
