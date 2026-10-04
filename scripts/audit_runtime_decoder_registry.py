#!/usr/bin/env python3
"""Audit source-level effect decoder registration parity, not gameplay semantics.

The monolithic decoder is a disabled reference implementation. Differences from
it are candidates until a freshly compiled canonical artifact fails to load.
Also compare the active routing table against each routed shard's decoder.
"""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import re

FAMILIES = ["ZoneLibrary", "Player", "Resources", "Permanent", "Combat",
            "StackEvent", "CompositionAL", "CompositionMZ"]


def source(path):
    content = path.read_bytes()
    return content.decode(), {"path": str(path), "sha256": hashlib.sha256(content).hexdigest()}


FAMILY_MODULES = ("zone_library", "player", "resources", "permanent", "combat", "stack_event",
                  "composition_a_l", "composition_m_z")


def audit(root):
    materializer, materializer_hash = source(root / "crates/ironsmith-engine/src/artifact_materializer.rs")
    facade, facade_hash = source(root / "crates/ironsmith-artifact-effect-decoder/src/lib.rs")
    reference = set(re.findall(r'"(\w+Effect)"\s*=>', materializer))
    routes = dict(re.findall(r'"(\w+Effect)"\s*=>\s*Some\(EffectFamily::(\w+)\)', facade))
    hashes = [materializer_hash, facade_hash]
    shards = {}
    for index, family in enumerate(FAMILIES):
        body, fingerprint = source(root / f"crates/ironsmith-artifact-effect-decoder/src/{FAMILY_MODULES[index]}.rs")
        shards[family] = set(re.findall(r'"(\w+Effect)"\s*=>', body))
        hashes.append(fingerprint)
    missing_shard = [{"effect": effect, "routed_family": family}
                     for effect, family in sorted(routes.items()) if effect not in shards.get(family, set())]
    unrouted = [{"effect": effect, "implemented_family": family}
                for family, effects in shards.items() for effect in sorted(effects) if effect not in routes]
    return {
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "scope": "source registration parity; missing reference entries are candidates, not independently confirmed card failures",
        "reference_effect_count": len(reference), "active_route_count": len(routes),
        "missing_from_active_routes": sorted(reference - routes.keys()),
        "active_routes_missing_shard_implementation": missing_shard,
        "shard_implementations_missing_routes": unrouted,
        "active_routes_not_in_reference": sorted(routes.keys() - reference),
        "provenance": hashes,
        "limitations": ["Regex recognizes explicit named Effect match arms only; it is not a Rust parser.",
                        "Reference decoder is cfg-disabled and may lag new functionality.",
                        "Registration parity does not validate payload types, execution, static abilities, triggers, or costs."],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out", type=Path, default=Path("reports/runtime-audit/decoder-registry.json"))
    args = parser.parse_args()
    result = audit(args.root)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k not in {"provenance", "limitations"}}, indent=2))


if __name__ == "__main__":
    main()
