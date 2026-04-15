#[path = "../ironsmith-registry/build_support.rs"]
mod registry_build_support;

fn main() {
    registry_build_support::run_runtime_registry_build();
}
