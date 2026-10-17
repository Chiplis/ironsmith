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
