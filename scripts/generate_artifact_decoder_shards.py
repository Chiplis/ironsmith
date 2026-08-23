#!/usr/bin/env python3
"""Generate parallel typed runtime-effect materializer crates.

The registry is captured once from the engine materializer and then becomes
the stable input for regeneration. Payloads are assigned by runtime ownership
family, so the eight crates are genuine sibling effect families rather than
arbitrary hash buckets. The physical ``shard-NN`` directories are retained to
avoid a noisy source-tree move; Cargo package names and routing use the domain
names below.
"""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "crates/ironsmith-artifact-effect-decoder/effect-registry.tsv"
MATERIALIZER = ROOT / "crates/ironsmith-engine/src/artifact_materializer.rs"
SHARD_COUNT = 8
FAMILY_NAMES = (
    "zone-library",
    "player",
    "resources",
    "permanent",
    "combat",
    "stack-event",
    "composition-a-l",
    "composition-m-z",
)


def runtime_family_for(kind: str) -> str:
    """Locate the engine module that implements one serialized effect kind."""

    effects_root = ROOT / "crates/ironsmith-engine/src/effects"
    patterns = (
        f"impl EffectExecutor for {kind}",
        f"impl CostExecutableEffect for {kind}",
        f"pub struct {kind}",
        f"pub enum {kind}",
        f"pub type {kind}",
        f"pub use ironsmith_core::{kind};",
    )
    sources = sorted(effects_root.rglob("*.rs"))
    for path in sources:
        source = path.read_text(encoding="utf-8")
        if any(pattern in source for pattern in patterns):
            return path.relative_to(effects_root).parts[0]
    # A number of shared-schema executors use multiline ``pub use`` lists.
    for path in sources:
        source = path.read_text(encoding="utf-8")
        if (
            kind in source
            and "pub use ironsmith_core" in source
            and ("impl EffectExecutor" in source or path.name == "mod.rs")
        ):
            return path.relative_to(effects_root).parts[0]
    raise SystemExit(f"could not locate runtime effect family for {kind}")


def shard_for(kind: str) -> int:
    family = runtime_family_for(kind)
    if family in {"zones", "cards"}:
        return 0
    if family == "player":
        return 1
    if family in {"mana", "life", "counters"}:
        return 2
    if family in {"permanents", "tokens", "control", "continuous"}:
        return 3
    if family in {"combat", "damage"}:
        return 4
    if family in {"stack", "delayed", "replacement", "restrictions.rs"}:
        return 5
    if family == "composition":
        return 6 if kind[0].lower() < "m" else 7
    raise SystemExit(f"unassigned runtime effect family {family!r} for {kind}")


def load_registry() -> list[tuple[str, str]]:
    if REGISTRY.exists():
        return [
            tuple(line.split("\t", 1))
            for line in REGISTRY.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.startswith("#")
        ]

    pattern = re.compile(
        r'^\s*"([^"]+)"\s*=>\s*decode_as::<T,\s*(.+)>\(effect\),\s*$'
    )
    entries = []
    for line in MATERIALIZER.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            entries.append((match.group(1), match.group(2)))
    if len(entries) < 200:
        raise SystemExit(f"expected at least 200 effect decoders, found {len(entries)}")
    REGISTRY.parent.mkdir(parents=True, exist_ok=True)
    REGISTRY.write_text(
        "# effect kind\tserde payload type\n"
        + "".join(f"{kind}\t{payload}\n" for kind, payload in entries),
        encoding="utf-8",
    )
    return entries


def manifest(name: str, dependencies: str) -> str:
    return f"""[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
{dependencies}

[lib]
path = "src/lib.rs"

[features]
default = []
"""


def write_shard(index: int, entries: list[tuple[str, str]]) -> None:
    crate_dir = ROOT / f"crates/ironsmith-artifact-effect-decoder-shard-{index:02d}"
    source_dir = crate_dir / "src"
    source_dir.mkdir(parents=True, exist_ok=True)
    (crate_dir / "Cargo.toml").write_text(
        manifest(
            f"ironsmith-runtime-effect-{FAMILY_NAMES[index]}",
            'ironsmith-core = { path = "../ironsmith-core", default-features = false, features = ["serde"] }\n'
            'ironsmith-compiled-artifact = { path = "../ironsmith-compiled-artifact", default-features = false }\n'
            'serde = "1.0.228"\n'
            'serde_json = "1.0.149"',
        ),
        encoding="utf-8",
    )
    arms = "\n".join(
        f'        "{kind}" => decode_as::<{payload}>(payload).map(Some),'
        for kind, payload in entries
    )
    (source_dir / "lib.rs").write_text(
        f"""//! Generated typed materializers for the {FAMILY_NAMES[index]} runtime effect family.

use std::any::Any;

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub type ErasedPayload = Box<dyn Any + Send + Sync>;

fn decode_as<D>(payload: Value) -> Result<ErasedPayload, String>
where
    D: DeserializeOwned + Send + Sync + 'static,
{{
    serde_json::from_value::<D>(payload)
        .map(|value| Box::new(value) as ErasedPayload)
        .map_err(|error| error.to_string())
}}

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {{
    match kind {{
{arms}
        _ => Ok(None),
    }}
}}
""",
        encoding="utf-8",
    )


def write_facade(entries: list[tuple[str, str]]) -> None:
    crate_dir = ROOT / "crates/ironsmith-artifact-effect-decoder"
    source_dir = crate_dir / "src"
    source_dir.mkdir(parents=True, exist_ok=True)
    dependencies = "\n".join(
        f'ironsmith-runtime-effect-{FAMILY_NAMES[index]} = '
        f'{{ path = "../ironsmith-artifact-effect-decoder-shard-{index:02d}", default-features = false }}'
        for index in range(SHARD_COUNT)
    )
    dependencies += '\nserde_json = "1.0.149"'
    (crate_dir / "Cargo.toml").write_text(
        manifest("ironsmith-artifact-effect-decoder", dependencies),
        encoding="utf-8",
    )
    family_variants = (
        "ZoneLibrary",
        "Player",
        "Resources",
        "Permanent",
        "Combat",
        "StackEvent",
        "CompositionAL",
        "CompositionMZ",
    )
    family_arms = "\n".join(
        f'        "{kind}" => Some(EffectFamily::{family_variants[shard_for(kind)]}),' 
        for kind, _ in entries
    )
    (source_dir / "lib.rs").write_text(
        f"""//! Parallel composition facade for typed compiled-effect decoding.

use std::any::Any;

use serde_json::Value;

pub type ErasedPayload = Box<dyn Any + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectFamily {{
    ZoneLibrary,
    Player,
    Resources,
    Permanent,
    Combat,
    StackEvent,
    CompositionAL,
    CompositionMZ,
}}

pub fn family_for_kind(kind: &str) -> Option<EffectFamily> {{
    match kind {{
{family_arms}
        _ => None,
    }}
}}

pub fn decode(kind: &str, payload: Value) -> Result<ErasedPayload, String> {{
    let decoded = match family_for_kind(kind) {{
        Some(family) => match family {{
            EffectFamily::ZoneLibrary => ironsmith_runtime_effect_zone_library::decode(kind, payload),
            EffectFamily::Player => ironsmith_runtime_effect_player::decode(kind, payload),
            EffectFamily::Resources => ironsmith_runtime_effect_resources::decode(kind, payload),
            EffectFamily::Permanent => ironsmith_runtime_effect_permanent::decode(kind, payload),
            EffectFamily::Combat => ironsmith_runtime_effect_combat::decode(kind, payload),
            EffectFamily::StackEvent => ironsmith_runtime_effect_stack_event::decode(kind, payload),
            EffectFamily::CompositionAL => ironsmith_runtime_effect_composition_a_l::decode(kind, payload),
            EffectFamily::CompositionMZ => ironsmith_runtime_effect_composition_m_z::decode(kind, payload),
        }},
        None => return Err(format!("unknown compiled effect payload kind: {{kind}}")),
    }}?;
    decoded.ok_or_else(|| format!("unknown compiled effect payload kind: {{kind}}"))
}}

#[cfg(test)]
mod tests {{
    use super::{{EffectFamily, family_for_kind}};

    #[test]
    fn routes_representative_effects_to_domain_families() {{
        assert_eq!(family_for_kind("MoveToZoneEffect"), Some(EffectFamily::ZoneLibrary));
        assert_eq!(family_for_kind("ChoosePlayerEffect"), Some(EffectFamily::Player));
        assert_eq!(family_for_kind("AddManaEffect"), Some(EffectFamily::Resources));
        assert_eq!(family_for_kind("CreateTokenEffect"), Some(EffectFamily::Permanent));
        assert_eq!(family_for_kind("DealDamageEffect"), Some(EffectFamily::Combat));
        assert_eq!(family_for_kind("CopySpellEffect"), Some(EffectFamily::StackEvent));
        assert_eq!(family_for_kind("ChooseModeEffect"), Some(EffectFamily::CompositionAL));
        assert_eq!(family_for_kind("WithIdEffect"), Some(EffectFamily::CompositionMZ));
        assert_eq!(family_for_kind("NotAnEffect"), None);
    }}
}}
""",
        encoding="utf-8",
    )


def main() -> None:
    entries = load_registry()
    shards = [[] for _ in range(SHARD_COUNT)]
    for entry in entries:
        shards[shard_for(entry[0])].append(entry)
    for index, entries_for_shard in enumerate(shards):
        write_shard(index, entries_for_shard)
    write_facade(entries)
    print(
        f"generated {len(entries)} decoders across {SHARD_COUNT} shards: "
        + ", ".join(str(len(entries_for_shard)) for entries_for_shard in shards)
    )


if __name__ == "__main__":
    main()
