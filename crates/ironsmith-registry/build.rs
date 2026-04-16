#[path = "build_support.rs"]
mod registry_build_support;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(ironsmith_runtime_parser_tests)");
    println!("cargo:rustc-check-cfg=cfg(ironsmith_runtime_inline_compiler_runtime)");
    registry_build_support::run_runtime_registry_build();
}
