#!/usr/bin/env python3
"""Download and filter Scryfall Oracle Cards into the repo's cards.json."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import tempfile
import urllib.request
from pathlib import Path

from stream_scryfall_blocks import (
    SUPPORTED_PAPER_FORMATS,
    is_legal_in_supported_paper_format,
    is_non_paper_print,
    iter_json_array,
)


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUT = ROOT / "cards.json"
BULK_DATA_URL = "https://api.scryfall.com/bulk-data/oracle-cards"
USER_AGENT = "ironsmith/1.0 (https://github.com/chiplis/ironsmith)"
ACCEPT = "application/json;q=0.9,*/*;q=0.8"


def fetch_json(url: str) -> dict:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": USER_AGENT,
            "Accept": ACCEPT,
        },
    )
    with urllib.request.urlopen(request) as response:
        return json.load(response)


def download_file(url: str, destination: Path) -> None:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": USER_AGENT,
            "Accept": ACCEPT,
        },
    )
    with urllib.request.urlopen(request) as response, destination.open("wb") as out:
        shutil.copyfileobj(response, out)


def should_keep_card(card: dict) -> bool:
    return is_legal_in_supported_paper_format(card) and not is_non_paper_print(card)


def write_filtered_cards(source: Path, destination: Path) -> tuple[int, int]:
    destination.parent.mkdir(parents=True, exist_ok=True)
    total = 0
    kept = 0

    with destination.open("w", encoding="utf-8") as out:
        out.write("[\n")
        wrote_any = False
        for card in iter_json_array(source):
            total += 1
            if not should_keep_card(card):
                continue

            if wrote_any:
                out.write(",\n")
            json.dump(card, out, ensure_ascii=False, separators=(",", ":"))
            wrote_any = True
            kept += 1

        out.write("\n]\n")

    return total, kept


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Download Scryfall Oracle Cards and keep only paper cards that are "
            f"legal in any of: {', '.join(fmt.title() for fmt in SUPPORTED_PAPER_FORMATS)}."
        )
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_OUT,
        help=f"Output JSON path (default: {DEFAULT_OUT})",
    )
    parser.add_argument(
        "--input",
        type=Path,
        help="Use an existing Oracle Cards dump (.json or .json.gz) instead of downloading.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    if args.input is not None:
        source = args.input
        metadata = None
    else:
        metadata = fetch_json(BULK_DATA_URL)
        download_uri = metadata.get("download_uri")
        if not download_uri:
            print("Failed to find download_uri in Scryfall bulk-data metadata.", file=sys.stderr)
            return 1

        with tempfile.TemporaryDirectory(prefix="ironsmith-scryfall-") as tmpdir:
            source = Path(tmpdir) / "oracle-cards.json.gz"
            print(f"[INFO] downloading {download_uri}", file=sys.stderr)
            download_file(download_uri, source)
            total, kept = write_filtered_cards(source, args.out)
            print(
                f"[INFO] wrote {kept} cards to {args.out} "
                f"(from {total} Oracle Cards entries)",
                file=sys.stderr,
            )
            if metadata:
                print(
                    f"[INFO] source updated_at: {metadata.get('updated_at')}",
                    file=sys.stderr,
                )
            return 0

    total, kept = write_filtered_cards(source, args.out)
    print(
        f"[INFO] wrote {kept} cards to {args.out} "
        f"(from {total} Oracle Cards entries)",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
