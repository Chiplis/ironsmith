#!/usr/bin/env python3
"""Offline source/filter regressions; no compiler, network, or corpus required."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import download_scryfall_cards as download
import stream_scryfall_blocks as stream


def paper_card(name="Card", **overrides):
    card = {"name": name, "games": ["paper"], "digital": False,
            "legalities": {"commander": "legal"}}
    card.update(overrides)
    return card


class EligibilityTests(unittest.TestCase):
    def test_all_supported_formats_accept_legal_and_restricted(self):
        formats = ("commander", "standard", "modern", "pioneer", "legacy", "vintage")
        self.assertEqual(stream.SUPPORTED_PAPER_FORMATS, formats)
        for fmt in formats:
            for status in ("legal", "restricted"):
                with self.subTest(fmt=fmt, status=status):
                    self.assertTrue(download.should_keep_card(paper_card(legalities={fmt: status})))

    def test_unsupported_formats_and_statuses_are_not_admitted(self):
        for legalities in ({"pioneer": "banned"}, {"vintage": "not_legal"},
                           {"vintage": "Restricted"}, {"legacy": True},
                           {"vintage": ["restricted"]}, {"modern": {"status": "legal"}},
                           {"alchemy": "legal"}, {"pauper": "legal"}):
            with self.subTest(legalities=legalities):
                self.assertFalse(download.should_keep_card(paper_card(legalities=legalities)))

    def test_download_requires_affirmative_legality(self):
        card = paper_card()
        del card["legalities"]
        self.assertFalse(download.should_keep_card(card))
        for value in (None, {}, [], ["legal"], "legal", False, 1):
            with self.subTest(value=value):
                card["legalities"] = value
                self.assertFalse(download.should_keep_card(card))
                self.assertTrue(stream.is_legal_in_supported_paper_format(
                    card, allow_missing_legalities=True))

    def test_digital_and_nonpaper_printings_remain_excluded(self):
        for legalities in ({"commander": "legal"}, {"pioneer": "legal"},
                           {"vintage": "restricted"}):
            for overrides in ({"digital": True}, {"games": ["arena"]},
                              {"games": ["mtgo"]}, {"games": ["arena", "mtgo"]}):
                with self.subTest(legalities=legalities, overrides=overrides):
                    self.assertFalse(download.should_keep_card(
                        paper_card(legalities=legalities, **overrides)))

    def test_existing_print_metadata_fallback_is_preserved(self):
        for games in (None, [], "paper", [None, ""], [" PAPER ", "arena"]):
            with self.subTest(games=games):
                self.assertTrue(download.should_keep_card(paper_card(games=games)))
        card = paper_card()
        del card["games"]
        self.assertTrue(download.should_keep_card(card))

    def test_explicit_format_subset_is_respected(self):
        restricted = paper_card(legalities={"vintage": "restricted"})
        self.assertTrue(stream.is_legal_in_supported_paper_format(restricted, ("vintage",)))
        self.assertFalse(stream.is_legal_in_supported_paper_format(restricted, ("modern",)))


class FilterTests(unittest.TestCase):
    def test_restricted_printings_dedup_without_nonpaper_overrides(self):
        restricted = paper_card("Restricted", oracle_id="restricted",
                                legalities={"vintage": "restricted"})
        cards = [
            {**restricted, "id": "old", "lang": "en", "released_at": "1993-01-01",
             "set_name": "First Paper Set"},
            {**restricted, "id": "new", "lang": "en", "released_at": "2000-01-01",
             "set_name": "Later Paper Set"},
            {**restricted, "id": "foreign", "lang": "fr", "released_at": "2010-01-01"},
            {**restricted, "id": "digital", "digital": True, "released_at": "1990-01-01",
             "set_name": "Not a paper origin"},
            {**restricted, "id": "mtgo", "games": ["mtgo"], "released_at": "2026-01-01"},
            paper_card("Pioneer", oracle_id="pioneer", legalities={"pioneer": "legal"}),
            paper_card("Ordinary", oracle_id="ordinary"),
            paper_card("Banned", legalities={"vintage": "banned"}),
        ]
        original = copy.deepcopy(cards)
        with tempfile.TemporaryDirectory() as directory:
            source, destination = Path(directory) / "raw.json", Path(directory) / "cards.json"
            source.write_text(json.dumps(cards), encoding="utf-8")
            self.assertEqual(download.write_filtered_cards(source, destination), (8, 3))
            result = json.loads(destination.read_text(encoding="utf-8"))
        self.assertEqual([c["name"] for c in result], ["Ordinary", "Pioneer", "Restricted"])
        self.assertEqual(result[2]["id"], "new")
        self.assertEqual(result[2]["first_printed_set_name"], "First Paper Set")
        self.assertEqual(cards, original)

    def test_missing_oracle_ids_keep_existing_name_dedup_boundary(self):
        cards = [paper_card("Same", id="first"), paper_card("Same", id="second"),
                 paper_card("Other")]
        with tempfile.TemporaryDirectory() as directory:
            source, destination = Path(directory) / "raw.json", Path(directory) / "cards.json"
            source.write_text(json.dumps(cards), encoding="utf-8")
            self.assertEqual(download.write_filtered_cards(source, destination), (3, 2))
            self.assertEqual([card["name"] for card in json.loads(destination.read_text())],
                             ["Other", "Same"])


class MetadataTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.cards = Path(self.directory.name) / "cards.json"
        self.sidecar = download.default_metadata_path(self.cards)
        self.cards.write_text("[]\n", encoding="utf-8")
        self.remote = {"type": "default_cards", "updated_at": "2026-10-12T00:00:00Z"}
        download.write_download_metadata(self.remote, self.sidecar, cards_path=self.cards,
                                         total=100, kept=0)

    def test_current_filter_policy_and_matching_content_can_skip(self):
        payload = json.loads(self.sidecar.read_text())
        self.assertEqual(payload["filter_policy_version"], 2)
        self.assertTrue(download.local_metadata_matches(self.remote, self.sidecar, self.cards))

    def test_legacy_and_other_policy_sidecars_force_refilter(self):
        original = json.loads(self.sidecar.read_text())
        for version in (None, 1, 3, "2"):
            payload = dict(original)
            if version is None:
                del payload["filter_policy_version"]
            else:
                payload["filter_policy_version"] = version
            self.sidecar.write_text(json.dumps(payload), encoding="utf-8")
            with self.subTest(version=version):
                self.assertFalse(download.local_metadata_matches(self.remote, self.sidecar, self.cards))

    def test_same_policy_does_not_bypass_data_identity(self):
        for remote in ({**self.remote, "type": "oracle_cards"},
                       {**self.remote, "updated_at": "later"}):
            with self.subTest(remote=remote):
                self.assertFalse(download.local_metadata_matches(remote, self.sidecar, self.cards))
        self.cards.write_text('[{"name":"changed"}]', encoding="utf-8")
        self.assertFalse(download.local_metadata_matches(self.remote, self.sidecar, self.cards))

    def test_malformed_sidecars_do_not_validate(self):
        for contents in ("invalid JSON", "null", "[]", "{}"):
            self.sidecar.write_text(contents, encoding="utf-8")
            with self.subTest(contents=contents):
                self.assertFalse(download.local_metadata_matches(self.remote, self.sidecar, self.cards))


if __name__ == "__main__":
    unittest.main()
