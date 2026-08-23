#!/usr/bin/env python3
"""Enforce the dependency and product boundaries used by the cold-build graph."""

from __future__ import annotations

import json
import subprocess
from collections import deque
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BINDING_PACKAGES = {
    "ironsmith-engine-wasm",
    "ironsmith-compiler-wasm",
    "ironsmith-verifier-wasm",
}


def metadata() -> dict:
    completed = subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def dependency_graph(packages: dict[str, dict]) -> dict[str, set[str]]:
    names = set(packages)
    return {
        name: {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"] in names and dependency["kind"] != "dev"
        }
        for name, package in packages.items()
    }


def reachable(graph: dict[str, set[str]], root: str) -> set[str]:
    seen: set[str] = set()
    queue = deque(graph[root])
    while queue:
        name = queue.popleft()
        if name in seen:
            continue
        seen.add(name)
        queue.extend(graph[name] - seen)
    return seen


def main() -> int:
    payload = metadata()
    packages = {package["name"]: package for package in payload["packages"]}
    graph = dependency_graph(packages)
    violations: list[str] = []

    for package in packages.values():
        for dependency in package["dependencies"]:
            if dependency.get("path") and dependency["uses_default_features"]:
                violations.append(
                    f"{package['name']} enables defaults for path dependency "
                    f"{dependency['name']}"
                )

    cdylibs = {
        package["name"]
        for package in packages.values()
        if any(
            "cdylib" in target["crate_types"]
            for target in package["targets"]
        )
    }
    if cdylibs != BINDING_PACKAGES:
        violations.append(
            f"cdylib packages are {sorted(cdylibs)}, expected {sorted(BINDING_PACKAGES)}"
        )

    forbidden_reachability = {
        "ironsmith-registry-sync": {
            "ironsmith-engine",
            "ironsmith-runtime",
            "ironsmith-compiler",
            "ironsmith-compiler-grammar",
            "ironsmith-text",
        },
        "ironsmith-engine": {
            "ironsmith-compiler",
            "ironsmith-compiler-grammar",
            "ironsmith-compiler-runtime",
            "ironsmith-text",
        },
        "ironsmith-compiler-grammar": {
            "ironsmith-engine",
            "ironsmith-runtime",
            "ironsmith-runtime-catalog",
            "ironsmith-text",
        },
    }
    for root, forbidden in forbidden_reachability.items():
        leaked = sorted(reachable(graph, root) & forbidden)
        if leaked:
            violations.append(f"{root} reaches forbidden packages: {', '.join(leaked)}")

    effect_families = {
        name for name in packages if name.startswith("ironsmith-runtime-effect-")
    }
    for family in sorted(effect_families):
        forbidden = {
            dependency
            for dependency in graph[family]
            if dependency.startswith("ironsmith-compiler")
            or dependency in {"ironsmith-engine", "ironsmith-runtime", "ironsmith-text"}
        }
        if forbidden:
            violations.append(
                f"{family} imports upper-layer packages: {', '.join(sorted(forbidden))}"
            )

    tree = subprocess.run(
        [
            "cargo",
            "tree",
            "--locked",
            "--offline",
            "-e",
            "features",
            "-p",
            "ironsmith-engine-wasm",
            "--target",
            "wasm32-unknown-unknown",
            "--no-default-features",
            "--features",
            "wasm-lean",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.lower()
    for prohibited in ('feature "runtime-parser-tests"', 'feature "engine-integration-tests"'):
        if prohibited in tree:
            violations.append(f"engine wasm feature graph contains {prohibited}")

    if violations:
        print("workspace boundary audit failed:")
        for violation in violations:
            print(f"- {violation}")
        return 1

    print(
        "workspace boundary audit passed: path defaults disabled, exactly three cdylibs, "
        "preflight/compiler/engine/effect edges clean, no test features in engine wasm"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
