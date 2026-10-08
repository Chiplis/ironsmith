#!/usr/bin/env python3
"""Offline metadata/hash/JSON consistency check; no compiler/corpus execution."""
import collections
import gzip
import hashlib
import json
from pathlib import Path

P = Path(__file__).resolve().parent

def meta(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}

def load(name):
    path = P / name
    if not path.exists():
        path = P / (name + ".gz")
    data = path.read_bytes()
    return json.loads(gzip.decompress(data) if path.suffix == ".gz" else data)

manifest = load("manifest.json")
assert set(manifest["files"]) == {str(p.relative_to(P)) for p in P.rglob("*") if p.is_file() and p.name != "manifest.json"}
for name, expected in manifest["files"].items():
    assert meta((P / name).read_bytes()) == expected, name
for name, original in load("packet-provenance.json")["files"].items():
    data = (P / name).read_bytes()
    if name.endswith(".gz"):
        assert data[4:8] == bytes(4), name
        data = gzip.decompress(data)
    assert meta(data) == original["source"], name
s = load("summary.json")
u = load("current-unresolved-entries.json")
assert len(u) == s["unresolved_entries"] == 1926
assert len({r["oracle_id"] for r in u}) == s["unique_unresolved"] == 1925
assert len({r["oracle_id"] for r in u if r["parse_status"] == "parse_failed"}) == s["unique_compile_failures"] == 1824
assert sum(r["parse_status"] == "strict_compiled" and r["parse_lossy"] for r in u) == s["strict_lossy_entries"] == 94
assert len({r["diagnostic_signature"] for r in u}) == s["diagnostic_signature_count"] == 899
for label in ("original", "oct7"):
    comparison = load(label + "-identity-comparison.json")
    assert {k: len(v) for k, v in comparison.items()} == s["versus_" + label]
    ids = [r["oracle_id"] for rows in comparison.values() for r in rows]
    assert len(ids) == len(set(ids)) == s["unique_ids"] == 32138
    assert {r["oracle_id"] for k in ("regressed", "still_unresolved") for r in comparison[k]} == {r["oracle_id"] for r in u}
assert s["versus_original"]["recovered"] == 1445
assert s["versus_oct7"]["recovered"] == 175
assert s["versus_oct7"]["regressed"] == 95
reg = load("regression-and-lossy-detail.json")
assert len(reg["parse_failure_regressions"]) == 7
assert len(reg["lossy_gate_regressions"]) == 95
assert collections.Counter(r["parse_status"] for r in reg["lossy_gate_regressions"]) == {"strict_compiled": 88, "parse_failed": 7}
derived = load("packet-summary.json")
assert derived["parse_loss_flags_all_statuses"] == s["parse_loss_flags_all_statuses"] == 1855
for status, count in (("strict_compiled", 88), ("parse_failed", 7)):
    expected = [{"card_name": r["card_name"], "oracle_id": r["oracle_id"]} for r in reg["lossy_gate_regressions"] if r["parse_status"] == status]
    assert derived["oct7_regression_memberships"][status] == expected
    assert len(expected) == count
signals = load("suspected-rendered-text-signals.json")
assert len(signals) == s["suspected_rendered_text_entries"] == 182
assert dict(collections.Counter(r["parse_status"] for r in signals)) == s["suspected_rendered_text_status_counts"]
assert len(load("linked-face-coverage.json")) == load("dataset-summary.json")["supplemental_face_payloads_unmeasured"] == 918
assert load("changed-input-evidence.json") == []
assert len(load("overlapping-diagnostic-causes.json")) == s["overlapping_diagnostic_cause_count"] == 911
assert load("dataset-summary.json")["partition_counts"] == {"unchanged_semantic_input": 32138}
next07 = load("next07-reconciliation.json")
assert len(next07["rows"]) == 6
assert all(r["parse_status"] == "strict_compiled" and not r["parse_lossy"] and not r["has_unimplemented"] for r in next07["rows"])
assert {r["card_name"] for r in next07["rows"] if r["semantic_mismatch"]} == {"Halfdane"}
assert next07["snapshot_sha256"] == s["snapshot_sha256"]
print("PASS: packet hashes, byte-exact source preservation and retained JSON accounting; no compiler/tests/corpus execution")
