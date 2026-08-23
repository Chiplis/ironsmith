fn main() {
    // The parser intentionally uses low-optimization Rust codegen so the
    // shipped Binaryen pass owns optimization. Its unoptimized grammar frames
    // exceed wasm-ld's 1 MiB default stack even for small cards.
    println!("cargo:rustc-link-arg=-zstack-size=67108864");
}
