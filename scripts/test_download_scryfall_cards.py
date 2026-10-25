import datetime
import json
from pathlib import Path
import tempfile
import unittest

import download_scryfall_cards as download


class DownloadTests(unittest.TestCase):
    def test_restricted_paper_card_survives_catalog_filter(self):
        recall = {
            "name": "Ancestral Recall",
            "games": ["paper"],
            "legalities": {"vintage": "restricted", "legacy": "banned"},
        }
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.json"
            output = Path(directory) / "cards.json"
            source.write_text(json.dumps([
                recall,
                {**recall, "name": "Digital", "games": ["arena"]},
                {**recall, "name": "Banned", "legalities": {"vintage": "banned"}},
            ]))
            self.assertEqual(download.write_filtered_cards(source, output), (3, 1))
            self.assertEqual(json.loads(output.read_text())[0]["name"], "Ancestral Recall")

    def test_unreleased_paper_card_is_kept_and_marked(self):
        not_legal = {"standard": "not_legal", "vintage": "not_legal", "commander": "not_legal"}
        preview = {
            "name": "Preview Dragon",
            "games": ["paper"],
            "set_type": "expansion",
            "released_at": "2026-11-14",
            "legalities": not_legal,
        }
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.json"
            output = Path(directory) / "cards.json"
            source.write_text(json.dumps([
                preview,
                {**preview, "name": "Old Oddity", "released_at": "2004-01-01"},
                {**preview, "name": "Upcoming Un-card", "set_type": "funny"},
                {**preview, "name": "Upcoming Token", "layout": "token"},
                {**preview, "name": "Upcoming Playtest", "promo_types": ["playtest"]},
                {**preview, "name": "Released Oddity", "released_at": "2026-10-19"},
            ]))
            today = datetime.date(2026, 10, 21)
            self.assertEqual(download.write_filtered_cards(source, output, today), (6, 1))
            [card] = json.loads(output.read_text())
            self.assertEqual(card["name"], "Preview Dragon")
            self.assertIs(card[download.UNRELEASED_MARKER], True)

    def test_filter_change_invalidates_fresh_bulk_cache(self):
        metadata = {"type": "default_cards", "updated_at": "2026-10-07"}
        with tempfile.TemporaryDirectory() as directory:
            cards = Path(directory) / "cards.json"
            sidecar = Path(directory) / "metadata.json"
            cards.write_text("[]")
            download.write_download_metadata(metadata, sidecar, cards_path=cards, total=0, kept=0)
            self.assertTrue(download.local_metadata_matches(metadata, sidecar, cards))
            cached = json.loads(sidecar.read_text())
            cached.pop("filterVersion")
            sidecar.write_text(json.dumps(cached))
            self.assertFalse(download.local_metadata_matches(metadata, sidecar, cards))


if __name__ == "__main__":
    unittest.main()
