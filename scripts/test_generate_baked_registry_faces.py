import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

import generate_baked_registry as registry


class FaceScoresTest(unittest.TestCase):
    def test_face_scores_do_not_inherit_whole_card_score(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "status.sqlite3"
            with sqlite3.connect(path) as db:
                db.execute("CREATE TABLE registry_card (card_name, raw_card_json)")
                db.execute("INSERT INTO registry_card VALUES (?, ?)", (
                    "Front // Back", json.dumps({
                        "name": "Front // Back", "layout": "modal_dfc",
                        "card_faces": [
                            {"name": "Front", "type_line": "Sorcery", "oracle_text": "Draw a card."},
                            {"name": "Back", "type_line": "Sorcery", "oracle_text": "Unsupported text."},
                        ],
                    }),
                ))
                db.execute("CREATE TABLE latest_card_face_compilation (face_name, similarity_score, parse_status, parse_error, has_unimplemented, normalized_oracle_text, compiled_text)")
                db.executemany("INSERT INTO latest_card_face_compilation VALUES (?, ?, ?, ?, ?, ?, ?)", [
                    ("Front", 0.997, "strict_compiled", None, 0, "Draw a card.", "Draw a card."),
                    ("Back", 0, "parse_failed", "Unsupported text", 0, "Unsupported text.", None),
                ])
            scores = registry.load_latest_semantic_scores(path)
            self.assertEqual(scores, {"front": 0.997})
            # Even a legacy aggregate score supplied by a caller cannot rescue Back.
            scores["front // back"] = 1.0
            _, linked, _, _, _ = registry.collect_unique_blocks(path, scores)
            self.assertEqual(linked[0][2], 0.997)
            self.assertEqual(linked[0][5], registry.UNSCORED_SENTINEL)

    def test_missing_face_view_requires_refresh(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, "sync_card_status_db"):
                registry.load_latest_semantic_scores(Path(directory) / "old.sqlite3")


if __name__ == "__main__":
    unittest.main()
