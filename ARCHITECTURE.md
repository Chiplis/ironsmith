# Workspace Architecture

This repository now uses an explicit workspace split instead of a root product crate.

## Crates

- `crates/ironsmith-runtime`
  - Primary engine crate during the extraction.
  - Owns gameplay/runtime behavior, the existing engine surface, and the current wasm/tooling-capable implementation.
- `crates/ironsmith-core`
  - Shared-domain extraction target.
  - Starts small and grows as shared data types move out of runtime.
- `crates/ironsmith-compiler`
  - Parser/compiler extraction target.
  - Downstream packages should migrate compiler-facing APIs here as they are carved out of runtime.
- `crates/ironsmith-registry`
  - Registry/loading extraction target.
  - Owns registry-facing APIs as they are carved out of runtime.
- `crates/ironsmith-wasm`
  - Wasm adapter extraction target.
- `crates/ironsmith-cli`
  - CLI adapter.
- `crates/ironsmith-tools`
  - Tooling adapter binaries and architecture guardrails.

## Boundary Rules

- `ironsmith-core` must not depend on any internal workspace crate.
- `ironsmith-runtime` must not depend on `ironsmith-compiler`, `ironsmith-registry`, or `ironsmith-wasm`.
- `ironsmith-compiler` must not depend on `ironsmith-runtime`, `ironsmith-registry`, or `ironsmith-wasm`.
- `ironsmith-registry` may depend on `ironsmith-core` and `ironsmith-compiler`, but not on `ironsmith-runtime`.
- Adapter crates may depend on the packages they need.

## Enforcement

- `cargo test --workspace` runs a workspace-boundary test in `crates/ironsmith-tools/tests/workspace_boundaries.rs`.
- The same test also enforces a soft structural rule for production code:
  - no non-generated Rust source file under `crates/*/src` may exceed 3,000 lines.

## Current State

- The workspace split is in place and the root is now a virtual manifest.
- `ironsmith-runtime` still contains the legacy monolithic implementation.
- `ironsmith-core` now owns shared low-level compiled-card support data such as ids,
  mana/types/colors/zones, counter metadata, static-ability identity metadata,
  common effect metadata, the shared filter-reference/player-filter model layer,
  and the aggregate compiled-card container types:
  `TotalCost`, `OptionalCost`, `OptionalCostsPaid`, `AlternativeCastingMethod`,
  `ResolutionProgram`, `Ability`, `CardDefinition`, and `AuraAttachmentFilter`.
- `ironsmith-runtime` now treats those compiled-card containers as core-owned models
  specialized with runtime payload types, and keeps only runtime behavior such as
  cost payment, target matching, and mana-symbol inference in runtime-local
  extension traits/helpers.
- `ironsmith-core`, `ironsmith-compiler`, `ironsmith-registry`, and `ironsmith-wasm` are active extraction targets rather than fully migrated end states.
