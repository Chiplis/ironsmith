fn main() {
    // Typed artifact materialization is deliberately compiled at opt-level 0
    // and can use large serde frames. Reserve stack explicitly instead of
    // relying on wasm-ld's 1 MiB default.
    // Host-side `--all-targets` test harnesses use the platform linker, which
    // does not understand wasm-ld's stack option.
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("wasm") {
        println!("cargo:rustc-link-arg=-zstack-size=67108864");
    }
}
