#[path = "../ironsmith-registry/build_support.rs"]
mod registry_build_support;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(ironsmith_runtime_parser_tests)");

    registry_build_support::run_runtime_registry_build();
}
