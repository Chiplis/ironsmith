#!/usr/bin/env python3
"""Hash the local Rust dependency closure that produces card artifacts."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROOT_PACKAGE = "ironsmith-artifact-baker"


def cargo_metadata() -> dict:
    completed = subprocess.run(
        [
            "cargo",
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def local_dependency_roots(metadata: dict) -> list[Path]:
    packages = {package["id"]: package for package in metadata["packages"]}
    root_ids = [
        package_id
        for package_id, package in packages.items()
        if package["name"] == ROOT_PACKAGE and package.get("source") is None
    ]
    if len(root_ids) != 1:
        raise RuntimeError(
            f"expected one local {ROOT_PACKAGE} package, found {len(root_ids)}"
        )

    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    pending = root_ids
    visited: set[str] = set()
    while pending:
        package_id = pending.pop()
        if package_id in visited:
            continue
        visited.add(package_id)
        pending.extend(dependency["pkg"] for dependency in nodes[package_id]["deps"])

    roots = []
    for package_id in visited:
        package = packages[package_id]
        if package.get("source") is not None:
            continue
        manifest = Path(package["manifest_path"]).resolve()
        if manifest.is_relative_to(ROOT):
            roots.append(manifest.parent)
    return sorted(set(roots))


def fingerprint_inputs(package_roots: list[Path]) -> list[Path]:
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for package_root in package_roots:
        paths.append(package_root / "Cargo.toml")
        build_script = package_root / "build.rs"
        if build_script.is_file():
            paths.append(build_script)
        source_root = package_root / "src"
        if source_root.is_dir():
            paths.extend(path for path in source_root.rglob("*") if path.is_file())
    return sorted(set(paths))


def main() -> None:
    digest = hashlib.sha256()
    for path in fingerprint_inputs(local_dependency_roots(cargo_metadata())):
        relative = path.relative_to(ROOT).as_posix().encode("utf-8")
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        payload = path.read_bytes()
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    print(digest.hexdigest())


if __name__ == "__main__":
    main()
