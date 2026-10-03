#!/usr/bin/env python3
"""Cluster complete campaign evidence with pinned, offline Scryfall Oracle tags.

Diagnostics define the disjoint primary groups. Card-level functional labels and
keywords are overlapping investigation hints, never compiler or runtime proof.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import gzip
import lzma
import hashlib
import heapq
import json
from pathlib import Path
import sys

import card_failure_campaign as campaign

DEFAULT_FIXTURES = Path(__file__).resolve().parents[1] / "fixtures/card-failure-campaign"
CATEGORY_FILE = "oracle-tags.selected-functional-categories.json"
NOTES = [
    "Each failed canonical compile entry occurs exactly once in the primary diagnostic groups.",
    "Unique Oracle IDs are a separate unit; reversible aliases remain separate compile entries.",
    "Functional labels and keywords overlap. Their counts must not be added to count failures.",
    "Tags describe whole cards, not the failed ability. Presence is not proof of root cause or correctness.",
    "Missing tag membership is unknown, not negative evidence. Ancestor membership is derived, not direct.",
    "Strict compile acceptance and semantic scores do not establish correct runtime behavior.",
]


def checked_file(path: Path, expected_hash: str, expected_bytes: int | None = None) -> bytes:
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != expected_hash:
        raise ValueError(f"SHA-256 mismatch: {path.name}")
    if expected_bytes is not None and len(raw) != expected_bytes:
        raise ValueError(f"byte count mismatch: {path.name}")
    return raw


def local_file(directory: Path, name: str) -> Path:
    # Pins identify sibling files; do not allow absolute/local traversal inputs.
    if not isinstance(name, str) or not name or Path(name).name != name:
        raise ValueError("pin filename must name a sibling file")
    return directory / name


def oracle_ids_for_card(card: dict) -> list[str]:
    """Prefer the top-level identity, falling back to distinct face identities."""
    if card.get("oracle_id"):
        return [card["oracle_id"]]
    return sorted({face["oracle_id"] for face in card.get("card_faces", []) if face.get("oracle_id")})


class TagIndex:
    def __init__(self, records: list[dict], categories: dict, metadata: dict):
        self.metadata = metadata
        self.catalog = {}
        by_id = {}
        self.direct = defaultdict(set)
        self.categories = {}
        tagging_count = annotation_count = 0
        for record in records:
            if record.get("object") != "tag" or record.get("type") != "oracle":
                raise ValueError("non-Oracle record in tag snapshot")
            slug, tag_id = record["slug"], record["id"]
            if not slug or not tag_id or slug in self.catalog or tag_id in by_id:
                raise ValueError("duplicate or empty tag slug/ID")
            self.catalog[slug] = record
            by_id[tag_id] = slug
            seen = set()
            for tagging in record["taggings"]:
                oracle_id = tagging["oracle_id"]
                if not oracle_id or oracle_id in seen:
                    raise ValueError("duplicate or empty direct Oracle tagging")
                seen.add(oracle_id)
                self.direct[oracle_id].add(slug)
                tagging_count += 1
                annotation_count += "annotation" in tagging
        if not self.catalog:
            raise ValueError("empty tag snapshot")
        counts = metadata["counts"]
        if (counts["tags"] != len(records) or counts["taggings"] != tagging_count
                or counts["annotations"] != annotation_count):
            raise ValueError("tag snapshot count mismatch")
        self.parents = {}
        self.children = {}
        for slug, record in self.catalog.items():
            for field in ("parent_ids", "child_ids"):
                if len(record[field]) != len(set(record[field])):
                    raise ValueError("duplicate hierarchy edge")
                if any(tag_id not in by_id for tag_id in record[field]):
                    raise ValueError("unknown hierarchy tag ID")
            self.parents[slug] = {by_id[tag_id] for tag_id in record["parent_ids"]}
            self.children[slug] = {by_id[tag_id] for tag_id in record["child_ids"]}
        for slug in self.catalog:
            if (any(slug not in self.children[parent] for parent in self.parents[slug])
                    or any(slug not in self.parents[child] for child in self.children[slug])):
                raise ValueError("non-reciprocal tag hierarchy")
        # Kahn's algorithm both rejects cycles and calculates ancestor-only hints.
        remaining = {slug: len(parents) for slug, parents in self.parents.items()}
        ready = [slug for slug, count in remaining.items() if not count]
        heapq.heapify(ready)
        self.ancestors = {}
        while ready:
            slug = heapq.heappop(ready)
            self.ancestors[slug] = frozenset(self.parents[slug]).union(
                *(self.ancestors[parent] for parent in self.parents[slug]))
            for child in sorted(self.children[slug]):
                remaining[child] -= 1
                if not remaining[child]:
                    heapq.heappush(ready, child)
        if len(self.ancestors) != len(self.catalog):
            raise ValueError("cycle in tag hierarchy")
        for category, selections in sorted(categories.items()):
            slugs = {selection["slug"] for selection in selections}
            if not slugs or not slugs <= self.catalog.keys():
                raise ValueError("empty or unknown functional category tag")
            self.categories[category] = slugs

    @classmethod
    def load(cls, directory: Path) -> "TagIndex":
        metadata_path = directory / "oracle-tags.metadata.json"
        metadata_raw = metadata_path.read_bytes()
        metadata = json.loads(metadata_raw)
        if metadata.get("schema_version") != 1 or metadata["bulk_metadata"]["type"] != "oracle_tags":
            raise ValueError("unsupported Oracle tag pin")
        raw = checked_file(local_file(directory, metadata["raw_file"]), metadata["raw_sha256"],
                           metadata["bulk_metadata"]["compressed_size"])
        selections = [item for item in metadata["artifacts"] if item["filename"] == CATEGORY_FILE]
        if len(selections) != 1:
            raise ValueError("missing or duplicate functional selection pin")
        selection = selections[0]
        category_raw = checked_file(directory / CATEGORY_FILE, selection["sha256"], selection["bytes"])
        records = [json.loads(line) for line in gzip.decompress(raw).splitlines() if line.strip()]
        result = cls(records, json.loads(category_raw), metadata)
        result.provenance = {
            "metadata_sha256": hashlib.sha256(metadata_raw).hexdigest(),
            "raw_sha256": metadata["raw_sha256"], "raw_file": metadata["raw_file"],
            "functional_selection_sha256": selection["sha256"],
            "source_uri": metadata["bulk_metadata"]["uri"],
            "download_uri": metadata["bulk_metadata"]["jsonl_download_uri"],
            "updated_at": metadata["bulk_metadata"]["updated_at"],
        }
        return result

    def enrich_card(self, card: dict) -> dict:
        ids = oracle_ids_for_card(card)
        direct = set().union(*(self.direct.get(oracle_id, set()) for oracle_id in ids))
        ancestors = set().union(*(self.ancestors[slug] for slug in direct)) - direct
        direct_categories = {category for category, slugs in self.categories.items() if slugs & direct}
        ancestor_categories = {category for category, slugs in self.categories.items() if slugs & ancestors}
        return {
            "oracle_ids": ids,
            "oracle_ids_without_direct_tags": [oracle_id for oracle_id in ids if not self.direct.get(oracle_id)],
            "oracle_tags_direct": sorted(direct),
            "oracle_tags_ancestor_only": sorted(ancestors),
            "functional_categories_direct": sorted(direct_categories),
            "functional_categories_ancestor_only": sorted(ancestor_categories - direct_categories),
            "scryfall_keywords": sorted(set(card.get("keywords", [])) | {
                keyword for face in card.get("card_faces", []) for keyword in face.get("keywords", [])}),
        }


def load_pinned_cards(directory: Path, cards_path: Path | None = None) -> dict:
    manifest_raw = (directory / "manifest.json").read_bytes()
    manifest = json.loads(manifest_raw)
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported corpus pin")
    if cards_path is None:
        archive = checked_file(local_file(directory, manifest["archive"]), manifest["archive_sha256"],
                               manifest["archive_bytes"])
        raw = (lzma.decompress(archive) if manifest["archive"].endswith(".xz")
               else gzip.decompress(archive))
    else:
        raw = cards_path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != manifest["dataset_sha256"]:
        raise ValueError("dataset SHA-256 differs from frozen campaign")
    if len(raw) != manifest["dataset_bytes"]:
        raise ValueError("dataset byte count differs from frozen campaign")
    cards = json.loads(raw)
    if not isinstance(cards, list) or not cards:
        raise ValueError("pinned corpus must be a nonempty array")
    indexed = {}
    # The campaign pin is already the complete canonical corpus. Refuse any
    # unexpected exclusion/deduplication rather than filtering troublesome rows.
    for position, card in enumerate(cards):
        faces = card.get("card_faces") or []
        raw_name = card.get("name")
        if raw_name is None:
            raw_name = faces[0].get("name") if faces else None
        name = campaign.normalize_name(raw_name) if isinstance(raw_name, str) else ""
        legalities = card.get("legalities")
        if (not name or name in indexed or card.get("digital") is True
                or (isinstance(legalities, dict) and legalities and not any(
                    legalities.get(fmt) == "legal" for fmt in campaign.FORMATS))):
            raise ValueError("pinned corpus requires exclusion or duplicate-name filtering")
        indexed[name] = {
            "card_name": name, "source_row_index": position,
            "source_card_sha256": campaign.json_digest(card),
            "layout": card.get("layout"), "top_level_oracle_id": card.get("oracle_id"),
            "face_names": [face.get("name") for face in faces],
            "face_oracle_ids": [face.get("oracle_id") for face in faces],
            "card": card,
        }
    expected = {
        "source_entry_count": len(cards), "canonical_card_count": len(indexed),
        "canonical_names_sha256": campaign.json_digest(sorted(indexed)),
        "unique_nonnull_oracle_id_count": len({card["oracle_id"] for card in cards if card.get("oracle_id")}),
        "entries_without_oracle_id_count": sum(not card.get("oracle_id") for card in cards),
        "source_face_count": sum(len(card.get("card_faces") or []) or 1 for card in cards),
    }
    if any(manifest.get(key) != value for key, value in expected.items()):
        raise ValueError("corpus membership/counts differ from frozen manifest")
    return {"manifest": manifest, "manifest_sha256": hashlib.sha256(manifest_raw).hexdigest(), "cards": indexed}


def unique_oracle_ids(entries: list[dict]) -> set[str]:
    return {oracle_id for entry in entries for oracle_id in entry["oracle_ids"]}


def secondary_groups(entries: list[dict], fields: tuple[str, ...]) -> list[dict]:
    groups = defaultdict(list)
    for entry in entries:
        # Union first: one entry in a category/keyword even when multiple selected
        # tags, keywords, or faces support that same secondary hint.
        for label in set().union(*(set(entry[field]) for field in fields)):
            groups[label].append(entry)
    return [{"label": label, "failed_entry_count": len(members),
             "failed_unique_oracle_id_count": len(unique_oracle_ids(members)),
             "cards": sorted(entry["card_name"] for entry in members),
             **({"direct_entry_count": sum(label in entry[fields[0]] for entry in members),
                 "ancestor_only_entry_count": sum(label in entry[fields[1]] for entry in members)}
                if len(fields) == 2 else {})}
            for label, members in sorted(groups.items(), key=lambda item: (-len(item[1]), item[0]))]


def build_report(snapshot: dict, corpus: dict, tags: TagIndex) -> dict:
    indexed = campaign.validate_snapshot(snapshot)
    manifest = corpus["manifest"]
    if (snapshot["dataset_sha256"] != manifest["dataset_sha256"]
            or snapshot["canonical_names_sha256"] != manifest["canonical_names_sha256"]
            or indexed.keys() != corpus["cards"].keys()):
        raise ValueError("snapshot dataset/membership differs from pinned campaign")
    # Do not trust precomputed supported flags, categories, routes, or counts.
    summary = campaign.summarize(list(indexed.values()))
    if snapshot["summary"] != summary:
        raise ValueError("snapshot summary does not match authoritative rows")
    source = snapshot.get("source", {})
    if not source.get("commit") or not source.get("tree") or source.get("status") != "":
        raise ValueError("snapshot must identify a clean committed compiler source")
    all_entries, failures = [], []
    primary = defaultdict(list)
    for name in sorted(indexed):
        row, source_card = indexed[name], corpus["cards"][name]
        entry = {key: value for key, value in source_card.items() if key != "card"}
        entry.update(tags.enrich_card(source_card["card"]))
        all_entries.append(entry)
        signature = campaign.failure_signature(row)
        if signature is None:
            continue
        entry.update({"diagnostic_signature": signature, "category": campaign.classify(row),
                      "parse_status": row["parse_status"], "parse_error": row.get("parse_error"),
                      "parse_loss_reasons": row.get("parse_loss_reasons"),
                      "oracle_text": row.get("oracle_text"), "raw_oracle_text": row.get("raw_oracle_text"),
                      "route_diagnostics": campaign.route_diagnostics(row),
                      "semantic_mismatch": bool(row.get("semantic_mismatch"))})
        failures.append(entry)
        primary[signature].append(entry)
    groups = []
    for signature, entries in sorted(primary.items(), key=lambda item: (-len(item[1]), item[0])):
        groups.append({
            "diagnostic_signature": signature, "failed_entry_count": len(entries),
            "failed_unique_oracle_id_count": len(unique_oracle_ids(entries)),
            "cards": [entry["card_name"] for entry in entries],
            "functional_categories": secondary_groups(entries, (
                "functional_categories_direct", "functional_categories_ancestor_only")),
            "scryfall_keywords": secondary_groups(entries, ("scryfall_keywords",)),
        })
    oracle_counts = Counter(oracle_id for entry in all_entries for oracle_id in entry["oracle_ids"])
    return {
        "schema_version": 1, "report_kind": "diagnostic_first_tag_enrichment",
        "provenance": {"compiler_source": source, "dataset_sha256": manifest["dataset_sha256"],
                       "canonical_names_sha256": manifest["canonical_names_sha256"],
                       "corpus_manifest_sha256": corpus["manifest_sha256"],
                       "snapshot_content_sha256": campaign.json_digest(snapshot),
                       "oracle_tags": tags.provenance},
        "notes": NOTES,
        "summary": {
            "canonical_compile_entry_count": len(all_entries),
            "supported_compile_entry_count": summary["supported_card_count"],
            "failed_compile_entry_count": len(failures),
            "corpus_unique_oracle_id_count": len(oracle_counts),
            "failed_unique_oracle_id_count": len(unique_oracle_ids(failures)),
            "oracle_ids_shared_by_multiple_compile_entries_count": sum(count > 1 for count in oracle_counts.values()),
            "corpus_reversible_alias_entry_count": sum(
                entry["layout"] == "reversible_card" and not entry["top_level_oracle_id"] for entry in all_entries),
            "failed_reversible_alias_entry_count": sum(
                entry["layout"] == "reversible_card" and not entry["top_level_oracle_id"] for entry in failures),
            "failed_entries_without_oracle_identity_count": sum(not entry["oracle_ids"] for entry in failures),
            "failed_entries_without_direct_tags_count": sum(not entry["oracle_tags_direct"] for entry in failures),
            "semantic_mismatch_entry_count": summary["semantic_mismatch_card_count"],
            "diagnostic_route_record_count": summary["diagnostic_route_record_count"],
            "diagnostic_group_count": len(groups),
        },
        "diagnostic_groups": groups, "failed_compile_entries": failures,
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, required=True, help="completed full-corpus campaign snapshot.json")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, default=DEFAULT_FIXTURES)
    parser.add_argument("--cards", type=Path, help="optional restored exact corpus; default is the pinned corpus archive")
    args = parser.parse_args(argv)
    try:
        if args.out.exists():
            raise ValueError("refusing to overwrite existing report")
        # Check authority before loading large static artifacts.
        raw = args.snapshot.read_bytes()
        snapshot = json.loads(raw)
        campaign.validate_snapshot(snapshot)
        corpus = load_pinned_cards(args.fixtures, args.cards)
        report = build_report(snapshot, corpus, TagIndex.load(args.fixtures))
        report["provenance"]["snapshot_file_sha256"] = hashlib.sha256(raw).hexdigest()
        with args.out.open("x", encoding="utf-8") as output:
            output.write(json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
        print(json.dumps(report["summary"], indent=2, sort_keys=True))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"card failure tag clusters: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
