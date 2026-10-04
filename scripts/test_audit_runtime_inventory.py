"""Inventory regression through the real worker and canonical payload loader."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKER = Path(os.environ.get("AUDIT_RUNTIME_WORKER", ROOT / "target/release/audit_runtime_worker"))


@unittest.skipUnless(WORKER.exists(), "build audit_runtime_worker or set AUDIT_RUNTIME_WORKER")
class FaceInventoryTests(unittest.TestCase):
    def test_every_explicit_face_layout_is_requested_and_exclusions_are_visible(self):
        rows = []
        expected_faces = set()
        for layout in ["adventure", "modal_dfc", "prepare", "reversible_card", "transform", "split", "flip"]:
            front, back = f"Audit {layout} Front", f"Audit {layout} Back"
            expected_faces.update([front, back])
            rows.append({"name": f"{front} // {back}", "layout": layout, "digital": False,
                         "games": ["paper"], "card_faces": [
                             {"name": front, "mana_cost": "{1}", "type_line": "Artifact", "oracle_text": "{T}: Add {C}."},
                             {"name": back, "mana_cost": "{U}", "type_line": "Instant", "oracle_text": "Draw a card."},
                         ]})
        rows.append({"name": "Audit Digital Front // Audit Digital Back", "layout": "modal_dfc", "digital": True,
                     "games": ["arena"], "card_faces": [
                         {"name": "Audit Digital Front", "type_line": "Artifact", "oracle_text": ""},
                         {"name": "Audit Digital Back", "type_line": "Instant", "oracle_text": "Draw a card."},
                     ]})
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "cards.json"
            path.write_text(json.dumps(rows))
            result = subprocess.run([str(WORKER), "--inventory", str(path)], check=True,
                                    capture_output=True, text=True, timeout=30)
        inventory = json.loads(result.stdout)
        payloads = {card["name"]: card for card in inventory["cards"]}
        self.assertTrue(expected_faces.issubset(payloads))
        self.assertEqual(inventory["unique_face_names"], 16)
        self.assertTrue(inventory["explicit_named_face_inventory"])
        self.assertEqual({row["name"] for row in inventory["face_exclusions"]},
                         {"Audit Digital Front", "Audit Digital Back"})
        self.assertEqual({row["name"] for row in inventory["exclusions"]},
                         {"Audit Digital Front // Audit Digital Back"})
        for name in expected_faces:
            if name.endswith(" Back"):
                self.assertIn("Draw a card.", payloads[name]["parse_input"])
                self.assertEqual(payloads[name]["oracle_text"], "Draw a card.")


if __name__ == "__main__":
    unittest.main()
