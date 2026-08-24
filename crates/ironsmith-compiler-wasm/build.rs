fn main() {
    // The parser intentionally uses low-optimization Rust codegen so the
    // shipped Binaryen pass owns optimization. Its unoptimized grammar frames
    // exceed wasm-ld's 1 MiB default stack even for small cards.
    // Host-side `--all-targets` test harnesses use the platform linker, which
    // does not understand wasm-ld's stack option.
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("wasm") {
        println!("cargo:rustc-link-arg=-zstack-size=67108864");
    }
}
