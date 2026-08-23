fn main() {
    // Typed artifact materialization is deliberately compiled at opt-level 0
    // and can use large serde frames. Reserve stack explicitly instead of
    // relying on wasm-ld's 1 MiB default.
    println!("cargo:rustc-link-arg=-zstack-size=67108864");
}
