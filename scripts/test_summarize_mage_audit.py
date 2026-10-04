import unittest

from summarize_mage_audit import match_report_rows


class OccurrenceLedgerTests(unittest.TestCase):
    def setUp(self):
        self.inventory = [{'scenario_id': 'a#0', 'test': 'duplicate'},
                          {'scenario_id': 'a#1', 'test': 'duplicate'}]

    def matched_ids(self, rows):
        return [item['scenario_id'] if item else None
                for item, _ in match_report_rows(self.inventory, rows)]

    def test_explicit_later_occurrence_is_not_reassigned_to_first(self):
        self.assertEqual(['a#1'], self.matched_ids([{'scenario_id': 'a#1', 'test': 'duplicate'}]))

    def test_legacy_duplicate_titles_keep_order(self):
        self.assertEqual(['a#0', 'a#1'], self.matched_ids([{'test': 'duplicate'}, {'test': 'duplicate'}]))

    def test_invalid_ids_and_duplicate_rows_do_not_fill_another_occurrence(self):
        rows = [{'scenario_id': 'missing', 'test': 'duplicate'},
                {'scenario_id': 'a#0', 'test': 'duplicate'},
                {'scenario_id': 'a#0', 'test': 'duplicate'}]
        self.assertEqual([None, 'a#0', None], self.matched_ids(rows))


if __name__ == '__main__':
    unittest.main()
