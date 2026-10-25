#!/usr/bin/env python3
"""Summarize live renderer_semantic_audit VERIFY output without DB writes.

Usage: python3 scripts/summarize_renderer_semantic_audit.py OUT_DIR
The raw corpus remains the evidence; this screen is not a gameplay proof.
"""
import collections
import json
import pathlib
import re
import sys

out = pathlib.Path(sys.argv[1])
counts = collections.Counter(hidden_mutations=0)
kinds = collections.Counter()
marker_ids = collections.Counter()
errors, poison, hidden, markers, support = [], [], [], [], []
face_names, name_oracle_pairs = set(), set()
with (out / "corpus.jsonl").open() as rows:
    for line in rows:
        r = json.loads(line)
        counts["faces"] += 1
        face_names.add(r["name"])
        name_oracle_pairs.add((r["name"], r.get("oracle")))
        if "error" in r:
            errors.append(r)
            continue
        counts["compiled"] += 1
        counts["static_abilities"] += r.get("static_abilities", 0)
        counts["mutations"] += r.get("mutation_count", 0)
        counts["faces_with_mutations"] += bool(r.get("mutation_count"))
        kinds.update(r.get("mutation_kinds", []))
        if r.get("poison_changed"):
            poison.append(r)
        if r.get("hidden_mutations"):
            hidden.append(r)
            counts["hidden_mutations"] += len(r["hidden_mutations"])
        if any("⟦unverified " in s for s in r.get("audit", [])):
            markers.append(r)
            marker_ids.update({m.casefold() for m in re.findall(r"id: Some‹([^›]+)›", "\n".join(s for s in r["audit"] if "⟦unverified " in s))})
        if r.get("support_issues"):
            support.append({"name": r["name"], "issues": r["support_issues"]})
for name, values in [("compile-errors", errors), ("poison-failures", poison),
                     ("hidden-mutations", hidden), ("audit-markers", markers),
                     ("support-issues", support)]:
    with (out / f"{name}.jsonl").open("w") as f:
        for row in values:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
counts.update(compile_errors=len(errors), poison_failures=len(poison),
              faces_with_hidden_mutations=len(hidden), audit_marker_faces=len(markers),
              support_issue_faces=len(support))
counts["static_abilities_without_selected_mutation"] = counts["static_abilities"] - counts["mutations"]
counts["unique_face_names"] = len(face_names)
counts["unique_name_oracle_pairs"] = len(name_oracle_pairs)
counts["unverified_unique_names"] = len({r["name"] for r in markers})
summary = dict(counts)
summary["mutation_kinds"] = dict(kinds)
summary["unverified_faces_by_static_id"] = dict(marker_ids)
summary["scope"] = "All catalog faces; retained-prose poison and selected static executable mutations. Not exhaustive field mutation or gameplay verification."
(out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
