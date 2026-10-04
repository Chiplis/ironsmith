import unittest

from audit_mage_unreported import missing_scenarios, selected_occurrence_rows


class MissingScenarioTests(unittest.TestCase):
    def setUp(self):
        self.inventory = [
            {'scenario_id': 'a#0', 'test': 'a :: duplicate'},
            {'scenario_id': 'a#1', 'test': 'a :: duplicate'},
            {'scenario_id': 'b#0', 'test': 'b :: later'},
        ]

    def report(self, rows=(), pending=None, stable=True):
        return {'engine_shims': False, 'provenance': {'unchanged': stable},
                'rows': list(rows), 'selected_unreported': self.inventory if pending is None else pending}

    def test_later_completed_occurrence_is_not_retried(self):
        reports = [self.report(), self.report(
            [{'scenario_id': 'a#1', 'test': 'a :: duplicate', 'status': 'failed'}], pending=[])]
        self.assertEqual(['a#0', 'b#0'], [item['scenario_id'] for item in missing_scenarios(self.inventory, reports)])

    def test_duplicate_historical_reports_do_not_complete_two_occurrences(self):
        earlier = self.report([{'test': 'a :: duplicate', 'status': 'passed'}])
        self.assertEqual(['a#1', 'b#0'], [item['scenario_id'] for item in missing_scenarios(self.inventory, [earlier, earlier])])

    def test_retry_of_later_duplicate_uses_its_own_tap_result(self):
        occurrences = self.inventory[:2]
        rows = [{'status': 'passed'}, {'status': 'failed'}]
        self.assertEqual([(occurrences[1], rows[1])],
                         selected_occurrence_rows(occurrences, [occurrences[1]], rows))

    def test_timeout_before_later_duplicate_does_not_credit_first_result(self):
        occurrences = self.inventory[:2]
        self.assertEqual([], selected_occurrence_rows(occurrences, [occurrences[1]],
                                                     [{'status': 'passed'}]))

    def test_unstable_or_missing_results_do_not_complete_scenarios(self):
        unstable = self.report([{'test': 'a :: duplicate', 'status': 'passed'}], stable=False)
        self.assertEqual([], missing_scenarios(self.inventory, [unstable]))
        missing = self.report([{'test': 'a :: duplicate', 'status': 'missing_result'}])
        self.assertEqual(self.inventory, missing_scenarios(self.inventory, [missing]))

    def test_older_complete_report_without_unreported_field_is_supported(self):
        report = self.report([{'test': 'a :: duplicate', 'status': 'passed'}])
        del report['selected_unreported']
        self.assertEqual(['a#1', 'b#0'], [item['scenario_id'] for item in missing_scenarios(self.inventory, [self.report(), report])])

    def test_older_unreported_titles_expand_to_each_occurrence(self):
        report = self.report(pending=[{'test': 'a :: duplicate'}])
        self.assertEqual(['a#0', 'a#1'], [item['scenario_id'] for item in missing_scenarios(self.inventory, [report])])


if __name__ == '__main__':
    unittest.main()
