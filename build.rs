fn main() {
    #[cfg(target_arch = "x86_64")]
    {
        cc::Build::new()
            .file("csrc/keccak_avx2.c")
            .flag("-mavx2")
            .flag("-march=native")
            .flag("-mtune=native")
            .flag("-funroll-loops")
            .opt_level_str("3")
            .compile("keccak_avx2");

        cc::Build::new()
            .file("csrc/keccak_avx512.c")
            .flag("-mavx512f")
            .flag("-march=native")
            .flag("-mtune=native")
            .flag("-funroll-loops")
            .opt_level_str("3")
            .compile("keccak_avx512");
    }

    println!("cargo:rerun-if-changed=csrc/");
}
