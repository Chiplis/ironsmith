#!/usr/bin/env python3
"""Find semantic-loss candidates; this is triage, never execution proof.

Reads canonical cards.json and the latest status DB observations without changing
either. Checks each Oracle ability line against the closest rendered compiled
line, preserving numeric constraints and a few scope-sensitive semantic markers.
Every result remains a candidate until inspected in current typed IR and exercised
with expected-result assertions. No hits does not imply a card is correct.
"""

from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
from difflib import SequenceMatcher
import hashlib
import json
from pathlib import Path
import re
import sqlite3
import subprocess

from stream_scryfall_blocks import iter_cards


ROOT = Path(__file__).resolve().parents[1]
NUMBER_WORDS = dict(zip(
    "zero one two three four five six seven eight nine ten eleven twelve thirteen "
    "fourteen fifteen sixteen seventeen eighteen nineteen twenty".split(), range(21)
))
NUMBER_PATTERN = re.compile(r"\b(?:" + "|".join(NUMBER_WORDS) + r")\b")
THRESHOLD_PATTERN = re.compile(r"\b(\d+) or (more|less)\b")


def normalize(text: str) -> str:
    text = text.lower().replace("one hundred", "100")
    text = NUMBER_PATTERN.sub(lambda m: str(NUMBER_WORDS[m.group()]), text)
    # Relative differences ('at least four more cards than you') require
    # expression comparison, which this lexical threshold screen cannot prove.
    text = re.sub(r"\bat least (\d+)\b(?! more\b)", r"\1 or more", text)
    text = re.sub(r"\bat most (\d+)\b", r"\1 or less", text)
    text = re.sub(r"\bor greater\b", "or more", text)
    text = re.sub(r"\bor fewer\b", "or less", text)
    return " ".join(text.split())


def threshold_constraints(text: str) -> Counter:
    # 'one or more' is frequently a batched-event spelling, not a numeric
    # comparison. Its semantics need the independent event-multiplicity audit.
    return Counter((int(n), op) for n, op in THRESHOLD_PATTERN.findall(normalize(text))
                   if int(n) > 1)


def nearest_line(oracle: str, compiled: list[str]) -> tuple[str, float]:
    if not compiled:
        return "", 0.0
    normalized = normalize(oracle)
    def similarity(line: str) -> float:
        candidate = normalize(line)
        ordered = SequenceMatcher(None, normalized, candidate, autojunk=False).ratio()
        # Conditions and subjects often move during rendering; preserve content
        # similarity when 'as long as ...' moves to the front of an ability.
        words = SequenceMatcher(None, sorted(normalized.split()), sorted(candidate.split()),
                                autojunk=False).ratio()
        return 0.35 * ordered + 0.65 * words
    scores = [(line, similarity(line)) for line in compiled]
    return max(scores, key=lambda item: item[1])


def scan_text(oracle: str, compiled: str) -> list[dict]:
    findings = []
    compiled_lines = [line.strip() for line in compiled.splitlines() if line.strip()]
    for index, line in enumerate(oracle.splitlines()):
        if not line.strip():
            continue
        match, confidence = nearest_line(line, compiled_lines)
        missing = threshold_constraints(line) - threshold_constraints(match)
        markers = []
        if missing:
            markers.append({"check": "numeric_threshold_not_preserved",
                            "missing": [{"number": n, "relation": op, "count": count}
                                        for (n, op), count in sorted(missing.items())]})
        if re.search(r"\b(?:for each|number of) graveyards?\b", line, re.I) and not re.search(
                r"\b(?:for each|number of) graveyards?\b", match, re.I):
            markers.append({"check": "graveyard_count_domain_not_preserved"})
        if re.search(r"\btotal (?:mana value|power|toughness)\b", line, re.I) and not re.search(
                r"\btotal (?:mana value|power|toughness)\b", match, re.I):
            markers.append({"check": "aggregate_constraint_not_preserved"})
        if re.search(r"for each of (?:that|the) spell['’]s colors", line, re.I) and not re.search(
                r"\bcolors?\b", match.partition(",")[2], re.I):
            markers.append({"check": "spell_color_count_domain_not_preserved"})
        # Delayed copying must retain its event, rather than resolving a copy
        # effect immediately. This is a targeted high-value family screen.
        if re.search(r"\b(?:when you next|copy the next)\b", line, re.I) and re.search(
                r"\bcopy\b", line, re.I) and not re.search(
                    r"\b(?:when|whenever|next)\b", match.partition(":")[2] or match, re.I):
            markers.append({"check": "delayed_copy_timing_not_preserved"})
        if markers:
            findings.append({"status": "candidate", "oracle_line_index": index,
                             "oracle_ability": line, "compiled_line_candidate": match,
                             "line_alignment_similarity": round(confidence, 5),
                             "checks": markers})
    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cards", type=Path, default=ROOT / "cards.json")
    parser.add_argument("--db", type=Path, default=ROOT / "reports/engine-status.sqlite3")
    parser.add_argument("--out", type=Path, default=ROOT / "reports/runtime-audit/semantic-candidates.json")
    args = parser.parse_args()
    canonical_names = {card["name"] for card in iter_cards(args.cards) if card.get("name")}
    conn = sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    rows = conn.execute("""
        SELECT c.id, c.card_name, c.oracle_text, c.compiled_text, c.parse_status,
               c.compiled_at, c.content_hash, c.similarity_score
        FROM latest_card_observation l
        JOIN card_compilation c ON c.id = l.compilation_id
        ORDER BY c.card_name
    """)
    candidates, seen, statuses = [], set(), Counter()
    for row in rows:
        name = row["card_name"]
        if name not in canonical_names:
            continue
        seen.add(name)
        statuses[row["parse_status"]] += 1
        if row["parse_status"] != "strict_compiled" or not row["compiled_text"]:
            continue
        for finding in scan_text(row["oracle_text"], row["compiled_text"]):
            candidates.append({"card_name": name, "observation_id": row["id"],
                               "compiled_at": row["compiled_at"],
                               "content_hash": row["content_hash"],
                               "similarity_score": row["similarity_score"], **finding})
    conn.close()
    result = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "scanner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "cards_path": str(args.cards.resolve()), "database_path": str(args.db.resolve()),
        "scope": "Latest stored strict compilations of names in canonical cards.json; rendered ability lines only",
        "limitations": [
            "Stored compilations can be stale; recompile and inspect typed IR before confirming defects.",
            "Line alignment is heuristic and can mismatch split/merged abilities.",
            "Render-only differences may preserve executable semantics.",
            "Only listed constraint families are screened; this is not an exhaustive semantic verifier.",
            "Zero findings is not an execution pass; missing or unexercised coverage remains unknown.",
        ],
        "coverage": {"canonical_card_names": len(canonical_names), "observed_names": len(seen),
                     "observation_status_counts": dict(statuses),
                     "missing_observation_names": sorted(canonical_names - seen)},
        "candidate_cards": len({x["card_name"] for x in candidates}),
        "candidate_abilities": len(candidates), "candidates": candidates,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"output": str(args.out), "candidate_cards": result["candidate_cards"],
                      "candidate_abilities": len(candidates), "statuses": dict(statuses)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
