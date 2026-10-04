#!/usr/bin/env python3
"""Freeze authoritative, full-corpus compile evidence and detect regressions.

This script deliberately does not implement a second card compiler. It runs the
existing sync_card_status_db command against a fresh SQLite database and exports
its latest_card_compilation view. A successful run is evidence that the audit
completed, not a claim that every card is supported or behaves correctly.
"""
from __future__ import annotations

import argparse
import gzip
import lzma
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import sys

SCHEMA_VERSION = 1
FORMATS = ("commander", "standard", "modern", "legacy", "vintage")
STATUSES = {"strict_compiled", "compiled_with_allow_unsupported", "parse_failed"}
FIELDS = (
    "card_name", "parse_status", "parse_error", "oracle_text", "raw_oracle_text",
    "normalized_oracle_text", "compiled_text", "oracle_coverage", "compiled_coverage",
    "similarity_score", "line_delta", "semantic_mismatch", "has_unimplemented",
    "parse_lossy", "parse_loss_reasons", "parse_loss_count", "content_hash",
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def json_digest(value) -> str:
    raw = json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(raw.encode()).hexdigest()


def write_json(path: Path, value) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n")


def normalize_name(value: str) -> str:
    value = value.strip()
    return value.replace(" / ", " // ", 1) if " // " not in value else value


def load_corpus(path: Path) -> dict:
    # Mirrors tooling.rs load_registry_cards_from_values and
    # build_registry_card_record_with_explicit_includes for membership only.
    # Fail closed if the Rust tool and this inventory ever disagree.
    with path.open(encoding="utf-8") as stream:
        cards = json.load(stream)
    if not isinstance(cards, list) or any(not isinstance(card, dict) for card in cards):
        raise ValueError("cards must be the Scryfall JSON array accepted by the Rust loader")
    selected = {}
    exclusions = Counter()
    source_faces = 0
    oracle_ids = Counter(card.get("oracle_id") for card in cards if card.get("oracle_id"))
    for card in cards:
        faces = card.get("card_faces") or []
        source_faces += len(faces) if faces else 1
        if card.get("digital") is True:
            exclusions["digital"] += 1
            continue
        legalities = card.get("legalities")
        if isinstance(legalities, dict) and legalities and not any(
            legalities.get(fmt) == "legal" for fmt in FORMATS
        ):
            exclusions["outside_supported_formats"] += 1
            continue
        face = faces[0] if faces else {}
        raw_name = card.get("name")
        if raw_name is None:
            raw_name = face.get("name")
        name = normalize_name(raw_name) if isinstance(raw_name, str) else ""
        if not name:
            exclusions["missing_name"] += 1
            continue
        if name in selected:
            exclusions["duplicate_name"] += 1
            continue
        selected[name] = {
            "card_name": name,
            "oracle_id": card.get("oracle_id"),
            "source_card_sha256": json_digest(card),
            "layout": card.get("layout"),
            "face_names": [face.get("name") for face in faces],
        }
    if not selected:
        raise ValueError("corpus contains no canonical cards")
    metadata = path.with_name(path.name + ".scryfall-bulk-data.json")
    dataset_hash = sha256_file(path)
    source_metadata = json.loads(metadata.read_text()) if metadata.is_file() else None
    if source_metadata and source_metadata.get("filtered_cards_sha256") != dataset_hash:
        raise ValueError("Scryfall sidecar does not match corpus SHA-256")
    return {
        "path": str(path.resolve()), "sha256": dataset_hash,
        "source_entry_count": len(cards), "source_face_count": source_faces,
        "unique_nonnull_oracle_id_count": len(oracle_ids),
        "entries_without_oracle_id_count": sum(not card.get("oracle_id") for card in cards),
        "duplicate_nonnull_oracle_id_count": sum(count - 1 for count in oracle_ids.values()),
        "canonical_card_count": len(selected), "exclusions": dict(exclusions),
        "canonical_names_sha256": json_digest(sorted(selected)),
        "source_metadata": source_metadata,
        "cards": [selected[name] for name in sorted(selected)],
    }


def is_supported(row: dict) -> bool:
    # Semantic mismatch is recorded separately: its similarity heuristic is not
    # the authoritative compiler's acceptance gate.
    return (row["parse_status"] == "strict_compiled"
            and not row["has_unimplemented"] and not row["parse_lossy"]
            and not row["parse_error"])


def classify(row: dict) -> str:
    error = row.get("parse_error") or ""
    if error.startswith("panic:"):
        return "compiler_panic"
    if row.get("has_unimplemented") or error.startswith((
        "Card compiled but contains unsupported mechanics:",
        "generated definition still contains unimplemented content",
    )):
        return "unsupported_mechanic"
    if error.startswith(("compiled text contains ", "compiled text dropped ")):
        return "semantic_output_failure"
    if row["parse_status"] == "compiled_with_allow_unsupported":
        return "allow_unsupported_fallback"
    if row["parse_status"] == "parse_failed":
        # 'UnsupportedLine' and 'unsupported ... clause' are parser diagnostics,
        # not proof that the underlying game mechanic is missing from runtime.
        return "parser_failure"
    if row.get("parse_lossy"):
        return "lossy_compilation"
    if error:
        return "inconsistent_success"
    return "strict_compiled"


def normalize_diagnostic(error: str) -> str:
    # Unwrap Rust Debug's single-string error wrapper before replacing quoted
    # input examples. Otherwise ParseError("reason...") collapses every reason.
    wrappers = []
    while True:
        match = re.fullmatch(r"([A-Za-z_][\w:]*)\((.*)\)", error, re.DOTALL)
        if not match:
            break
        inner = match.group(2)
        if inner.startswith('"') and inner.endswith('"'):
            try:
                inner = json.loads(inner)
            except ValueError:
                break
        wrappers.append(match.group(1))
        error = inner
    error = re.sub(r"'[^'\n]*'|\"[^\"\n]*\"", "<text>", error)
    error = re.sub(r"\b\d+\b", "<n>", error)
    return ": ".join(wrappers + [" ".join(error.split())])


def failure_signature(row: dict) -> str | None:
    if is_supported(row):
        return None
    error = row.get("parse_error") or row.get("parse_loss_reasons") or ""
    # Diagnostic groups are NOT parser routes or unique mechanics. Preserve
    # both primary and fallback failure reasons while stripping input examples.
    marker = "; oracle-only fallback also failed: "
    signature = marker.join(normalize_diagnostic(part) for part in error.split(marker))
    return classify(row) + ": " + signature


def route_diagnostics(row: dict) -> list[dict]:
    """Extract distinct payload input diagnostics, never treat them as cards.

    A combined error contains both metadata-bearing input and Oracle-only input
    failures. Loss records preserve the primary failure when the fallback
    succeeds. Repeated strict/allow attempts are not individually observable.
    """
    error = row.get("parse_error") or ""
    separator = "; oracle-only fallback also failed: "
    if separator in error:
        primary, fallback = error.split(separator, 1)
        return [{"route": "parse_input", "outcome": "failed", "diagnostic": primary},
                {"route": "oracle_only", "outcome": "failed", "diagnostic": fallback}]
    prefix = "oracle_only_fallback: parse input failed before oracle text fallback: "
    losses = row.get("parse_loss_reasons") or ""
    if prefix in losses:
        primary = losses.split(prefix, 1)[1].split("\n", 1)[0]
        outcome = "compiled" if row["parse_status"] != "parse_failed" else "unknown"
        return [{"route": "parse_input", "outcome": "failed", "diagnostic": primary},
                {"route": "oracle_only", "outcome": outcome, "diagnostic": None}]
    if row["parse_status"] == "parse_failed" and classify(row) == "parser_failure":
        return [{"route": "parse_input", "outcome": "failed", "diagnostic": error}]
    return []


def summarize(rows: list[dict]) -> dict:
    groups = defaultdict(list)
    for row in rows:
        signature = failure_signature(row)
        if signature is not None:
            groups[signature].append(row["card_name"])
    routes = [route for row in rows for route in route_diagnostics(row)]
    return {
        "canonical_card_count": len(rows),
        "parse_lossy_card_count": sum(bool(row["parse_lossy"]) for row in rows),
        "diagnostic_route_record_count": len(routes),
        "diagnostic_route_counts": dict(sorted(Counter(
            route["route"] + ":" + route["outcome"] for route in routes
        ).items())),
        "supported_card_count": sum(is_supported(row) for row in rows),
        "failing_card_count": sum(not is_supported(row) for row in rows),
        "parse_status_counts": dict(sorted(Counter(row["parse_status"] for row in rows).items())),
        "failure_category_counts": dict(sorted(Counter(classify(row) for row in rows).items())),
        "semantic_mismatch_card_count": sum(bool(row["semantic_mismatch"]) for row in rows),
        "diagnostic_group_count": len(groups),
        "parser_route_count": None,
        "parser_route_count_note": "The status DB does not record parser-route identities; diagnostic groups are not routes.",
        "diagnostic_groups": [
            {"signature": signature, "card_count": len(names), "cards": sorted(names)}
            for signature, names in sorted(groups.items(), key=lambda item: (-len(item[1]), item[0]))
        ],
    }


def read_database(db_path: Path, corpus: dict) -> list[dict]:
    if not db_path.is_file():
        raise ValueError(f"missing status database: {db_path}")
    conn = sqlite3.connect(db_path.resolve().as_uri() + "?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    with conn:
        if conn.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
            raise ValueError("status database failed integrity check")
        rows = []
        for db_row in conn.execute(
            "SELECT " + ", ".join(FIELDS) + ", compiled_card_definition FROM latest_card_compilation ORDER BY card_name"
        ):
            row = dict(db_row)
            definition = row.pop("compiled_card_definition")
            row["compiled_definition_sha256"] = (
                hashlib.sha256(definition.encode()).hexdigest() if definition is not None else None
            )
            if row["parse_status"] not in STATUSES:
                raise ValueError(f"unknown parse status for {row['card_name']}: {row['parse_status']}")
            row["route_diagnostics"] = route_diagnostics(row)
            row["category"] = classify(row)
            row["supported"] = is_supported(row)
            rows.append(row)
    conn.close()
    expected = {card["card_name"] for card in corpus["cards"]}
    actual = {row["card_name"] for row in rows}
    if expected != actual or len(actual) != len(rows):
        raise ValueError(f"incomplete/changed corpus: missing={sorted(expected-actual)}, extra={sorted(actual-expected)}, duplicates={len(rows)-len(actual)}")
    return rows


def git_output(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def source_state(repo: Path) -> dict:
    return {
        "commit": git_output(repo, "rev-parse", "HEAD"),
        "tree": git_output(repo, "rev-parse", "HEAD^{tree}"),
        "status": git_output(repo, "status", "--porcelain", "--untracked-files=normal"),
    }


def partition_corpus(corpus: dict, processes: int) -> list[dict]:
    if processes < 1:
        raise ValueError("--processes must be at least 1")
    count = min(processes, len(corpus["cards"]))
    if not count:
        raise ValueError("cannot partition an empty corpus")
    return [{**corpus, "cards": corpus["cards"][index::count]} for index in range(count)]


def run_logged_command(command, repo, env, output):
    with (output / "stdout.log").open("w") as stdout, (output / "stderr.log").open("w") as stderr:
        return subprocess.run(command, cwd=repo, env=env, stdout=stdout, stderr=stderr).returncode


def run_audit(args) -> int:
    repo, cards, out = args.repo.resolve(), args.cards.resolve(), args.out_dir.resolve()
    checkout_state = source_state(repo)
    if args.processes > 1 and not args.sync_bin:
        raise ValueError("parallel audit requires --sync-bin and --build-manifest; build once before sharding")
    build_manifest = None
    binary_hash = None
    if bool(args.sync_bin) != bool(args.build_manifest):
        raise ValueError("--sync-bin and --build-manifest must be supplied together")
    if args.sync_bin:
        build_manifest = json.loads(args.build_manifest.read_text())
        state = build_manifest["source"]
        binary_hash = sha256_file(args.sync_bin)
        if binary_hash != build_manifest["binary_sha256"]:
            raise ValueError("frozen binary does not match build-manifest SHA-256")
        if not build_manifest.get("build_command"):
            raise ValueError("build manifest must record the successful build command")
        tree = git_output(repo, "rev-parse", state["commit"] + "^{tree}")
        if tree != state["tree"]:
            raise ValueError("build manifest source tree does not match its commit")
    else:
        state = checkout_state
    if state["status"]:
        raise ValueError("audit compiler must come from a clean committed checkout")
    if args.expected_commit and state["commit"] != args.expected_commit:
        raise ValueError(f"expected commit {args.expected_commit}, found {state['commit']}")
    corpus = load_corpus(cards)
    partitions = partition_corpus(corpus, args.processes)
    out.mkdir(parents=True, exist_ok=False)
    write_json(out / "corpus.json", corpus)
    env = dict(os.environ)
    # Avoid accidental permissive parsing and semantic scoring changes. Serialize
    # the audit for conservative reproducibility. Tooling toggles a global
    # fallback flag, but the active payload path explicitly passes false; this
    # is not evidence of a currently active strict/allow race.
    removed = sorted(key for key in env if key.startswith("IRONSMITH_"))
    for key in removed:
        del env[key]
    env["RAYON_NUM_THREADS"] = "1"
    if args.sync_bin:
        command = [str(args.sync_bin.resolve())]
    else:
        command = ["cargo", "run", "--locked", "-p", "ironsmith-tools", "--bin", "sync_card_status_db"]
        if args.release:
            command.append("--release")
        command.append("--")
    command += ["--cards", str(cards)]
    shards = []
    for index, partition in enumerate(partitions):
        shard_dir = out if len(partitions) == 1 else out / f"shard-{index:03d}"
        shard_dir.mkdir(exist_ok=True)
        shard_command = command + ["--db-path", str(shard_dir / "status.sqlite3")]
        if len(partitions) > 1:
            names = shard_dir / "cards.names"
            names.write_text("\n".join(card["card_name"] for card in partition["cards"]) + "\n")
            shard_command += ["--names-file", str(names)]
        shards.append({"index": index, "directory": str(shard_dir),
                       "card_count": len(partition["cards"]),
                       "canonical_names_sha256": json_digest(sorted(card["card_name"] for card in partition["cards"])),
                       "command": shard_command})
    manifest = {
        "schema_version": SCHEMA_VERSION, "completed": False,
        "started_at": datetime.now(timezone.utc).isoformat(), "repo": str(repo),
        "source": state, "dataset_sha256": corpus["sha256"],
        "checkout_state_at_start": checkout_state, "build_manifest": build_manifest,
        "command": shards[0]["command"] if len(shards) == 1 else None,
        "processes": len(shards), "shards": shards, "audit_mode": "authoritative_full_corpus",
        "environment": {key: env.get(key) for key in (
            "RAYON_NUM_THREADS", "RUSTFLAGS", "CARGO_TARGET_DIR", "CARGO_BUILD_JOBS",
            "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_RELEASE_DEBUG",
        )},
        "cleared_ironsmith_environment_keys": removed,
        "rustc_version": subprocess.check_output(["rustc", "--version"], cwd=repo, text=True).strip(),
        "cargo_version": subprocess.check_output(["cargo", "--version"], cwd=repo, text=True).strip(),
    }
    write_json(out / "run.json", manifest)
    with ThreadPoolExecutor(max_workers=len(shards)) as executor:
        futures = [executor.submit(run_logged_command, shard["command"], repo, env, Path(shard["directory"]))
                   for shard in shards]
        for shard, future in zip(shards, futures):
            shard["exit_code"] = future.result()
            write_json(out / "run.json", manifest)
    manifest["exit_code"] = next((shard["exit_code"] for shard in shards if shard["exit_code"]), 0)
    manifest["finished_at"] = datetime.now(timezone.utc).isoformat()
    write_json(out / "run.json", manifest)
    if manifest["exit_code"]:
        raise ValueError(f"compiler audit failed ({manifest['exit_code']}); see shard stderr logs in {out}")
    if sha256_file(cards) != corpus["sha256"]:
        raise ValueError("corpus changed during audit; evidence is invalid")
    if args.sync_bin:
        if sha256_file(args.sync_bin) != binary_hash:
            raise ValueError("frozen compiler binary changed during audit")
    elif source_state(repo) != checkout_state:
        raise ValueError("source checkout changed during audit; evidence is invalid")
    rows = []
    for shard, partition in zip(shards, partitions):
        rows.extend(read_database(Path(shard["directory"]) / "status.sqlite3", partition))
    rows.sort(key=lambda row: row["card_name"])
    expected_names = sorted(card["card_name"] for card in corpus["cards"])
    if [row["card_name"] for row in rows] != expected_names:
        raise ValueError("aggregate shard coverage differs from the full frozen corpus")
    snapshot = {
        "schema_version": SCHEMA_VERSION, "audit_mode": manifest["audit_mode"],
        "source": state, "dataset_sha256": corpus["sha256"],
        "canonical_names_sha256": corpus["canonical_names_sha256"],
        "full_coverage": True, "summary": summarize(rows), "cards": rows,
    }
    write_json(out / "snapshot.json", snapshot)
    manifest["completed"] = True
    manifest["snapshot_sha256"] = sha256_file(out / "snapshot.json")
    write_json(out / "run.json", manifest)
    print(json.dumps({key: value for key, value in snapshot["summary"].items() if key != "diagnostic_groups"}, indent=2))
    return 0


def validate_snapshot(snapshot: dict) -> dict[str, dict]:
    if snapshot.get("schema_version") != SCHEMA_VERSION or not snapshot.get("full_coverage"):
        raise ValueError("unsupported or incomplete snapshot")
    if snapshot.get("audit_mode") != "authoritative_full_corpus":
        raise ValueError("snapshot is not an authoritative full-corpus audit")
    rows = snapshot["cards"]
    if not rows or any(row["parse_status"] not in STATUSES for row in rows):
        raise ValueError("empty snapshot or unknown status")
    indexed = {row["card_name"]: row for row in rows}
    if len(indexed) != len(rows) or json_digest(sorted(indexed)) != snapshot["canonical_names_sha256"]:
        raise ValueError("duplicate, missing, or changed snapshot cards")
    if len(rows) != snapshot["summary"]["canonical_card_count"]:
        raise ValueError("snapshot card count mismatch")
    return indexed


def compare_snapshots(baseline: dict, current: dict, tolerance: float = 1e-6) -> dict:
    before, after = validate_snapshot(baseline), validate_snapshot(current)
    if baseline["dataset_sha256"] != current["dataset_sha256"] or before.keys() != after.keys():
        raise ValueError("baseline and current corpus differ; do not rebaseline to hide missing cards")
    resolved, unresolved, regressions, changed = [], [], [], []
    for name in sorted(before):
        old, new = before[name], after[name]
        was_supported, now_supported = is_supported(old), is_supported(new)
        if not was_supported:
            (resolved if now_supported else unresolved).append(name)
        reasons = []
        if was_supported and not now_supported:
            reasons.append("previously_supported_card_failed")
        if was_supported and now_supported:
            if new["similarity_score"] < old["similarity_score"] - tolerance:
                reasons.append("similarity_score_decreased")
            if not old["semantic_mismatch"] and new["semantic_mismatch"]:
                reasons.append("new_semantic_mismatch")
        if reasons:
            regressions.append({"card_name": name, "reasons": reasons,
                                "before_status": old["parse_status"], "after_status": new["parse_status"],
                                "before_score": old["similarity_score"], "after_score": new["similarity_score"],
                                "after_error": new.get("parse_error")})
        if old["compiled_definition_sha256"] != new["compiled_definition_sha256"]:
            changed.append(name)
    return {
        "schema_version": SCHEMA_VERSION,
        "baseline_commit": baseline["source"]["commit"], "current_commit": current["source"]["commit"],
        "dataset_sha256": baseline["dataset_sha256"], "total_card_count": len(before),
        "baseline_failing_card_count": len(resolved) + len(unresolved),
        "resolved_baseline_card_count": len(resolved), "remaining_baseline_card_count": len(unresolved),
        "regression_card_count": len(regressions), "resolved_baseline_cards": resolved,
        "remaining_baseline_cards": unresolved, "regressions": regressions,
        "changed_definition_cards": changed,
        "compile_campaign_complete": not unresolved and not regressions,
        "note": "Compile acceptance and semantic-score checks do not prove runtime behavior; focused engine tests remain required.",
    }


def restore_corpus(archive: Path, output: Path, expected_sha256: str) -> None:
    """Recreate exact frozen source bytes; never overwrite an existing corpus."""
    if output.exists():
        raise ValueError(f"refusing to overwrite existing corpus: {output}")
    temporary = output.with_name(output.name + ".partial")
    if temporary.exists():
        raise ValueError(f"refusing to overwrite existing partial file: {temporary}")
    try:
        opener = lzma.open if archive.suffix == ".xz" else gzip.open
        with opener(archive, "rb") as source, temporary.open("xb") as destination:
            shutil.copyfileobj(source, destination)
        if sha256_file(temporary) != expected_sha256:
            raise ValueError("restored corpus SHA-256 differs from frozen manifest")
        temporary.rename(output)
    except BaseException:
        if temporary.exists():
            temporary.unlink()
        raise


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    restore = commands.add_parser("restore", help="restore exact dataset bytes from a pinned gzip or xz archive")
    restore.add_argument("--archive", type=Path, required=True)
    restore.add_argument("--out", type=Path, required=True)
    restore.add_argument("--sha256", required=True)
    inventory = commands.add_parser("inventory", help="identify corpus membership without compiling")
    inventory.add_argument("--cards", type=Path, required=True)
    inventory.add_argument("--out", type=Path, required=True)
    run = commands.add_parser("run", help="compile every card into a fresh, immutable audit directory")
    run.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    run.add_argument("--cards", type=Path, required=True)
    run.add_argument("--out-dir", type=Path, required=True)
    run.add_argument("--expected-commit")
    run.add_argument("--release", action="store_true")
    run.add_argument("--processes", type=int, default=1, help="isolated frozen-binary workers with disjoint name shards")
    run.add_argument("--sync-bin", type=Path, help="frozen compiler binary; requires build provenance")
    run.add_argument("--build-manifest", type=Path, help="JSON: source {commit, tree, status}, binary_sha256, build_command")
    compare = commands.add_parser("compare", help="compare all cards against the frozen baseline")
    compare.add_argument("--baseline", type=Path, required=True)
    compare.add_argument("--current", type=Path, required=True)
    compare.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "restore":
            restore_corpus(args.archive, args.out, args.sha256)
            print(f"Restored verified corpus: {args.out}")
            return 0
        if args.command == "inventory":
            corpus = load_corpus(args.cards)
            write_json(args.out, corpus)
            print(json.dumps({key: value for key, value in corpus.items() if key != "cards"}, indent=2))
            return 0
        if args.command == "run":
            return run_audit(args)
        report = compare_snapshots(json.loads(args.baseline.read_text()), json.loads(args.current.read_text()))
        write_json(args.out, report)
        print(json.dumps({key: value for key, value in report.items() if not isinstance(value, list)}, indent=2))
        return 0 if report["compile_campaign_complete"] else 1
    except (ValueError, OSError, sqlite3.Error, subprocess.CalledProcessError, KeyError) as error:
        print(f"card failure campaign: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
