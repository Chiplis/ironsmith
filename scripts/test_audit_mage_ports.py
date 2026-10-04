import unittest

from audit_mage_ports import classify, completed_files_from_reports, parse_tap


class MageReportIntegrityTests(unittest.TestCase):
    def test_resume_requires_every_occurrence_in_a_stable_report(self):
        inventory = [{'file': 'duplicate.mjs', 'test': 'same'}] * 2
        single = {'provenance': {'unchanged': True},
                  'rows': [{'test': 'same', 'status': 'passed'}]}
        self.assertEqual(completed_files_from_reports(inventory, [single, single]), set())
        complete = {'provenance': {'unchanged': True},
                    'rows': single['rows'] + [{'test': 'same', 'status': 'failed'}]}
        self.assertEqual(completed_files_from_reports(inventory, [complete]), {'duplicate.mjs'})
        complete['provenance']['unchanged'] = False
        self.assertEqual(completed_files_from_reports(inventory, [complete]), set())

    def test_missing_results_do_not_count_as_completed(self):
        rows = parse_tap('# Subtest: source :: first\nok 1 - source :: first\n'
                         '# Subtest: source :: second\n')
        self.assertEqual([row['status'] for row in rows], ['passed', 'missing_result'])
        inventory = [{'file': 'test.mjs', 'test': f'source :: {name}'}
                     for name in ['first', 'second']]
        self.assertEqual(completed_files_from_reports(inventory, [
            {'provenance': {'unchanged': True}, 'rows': rows}]), set())

    def test_failures_are_conservative_categories_not_confirmed_engine_bugs(self):
        self.assertEqual(classify('create clause missing token [rule-path=clause-reading]'),
                         'parser_failure_before_expected_outcome')
        self.assertEqual(classify('dispatch failed: Resolution failed: Cannot resolve value'),
                         'runtime_error_candidate')
        self.assertEqual(classify('expected life 16, got 19'),
                         'outcome_or_fixture_mismatch_candidate')
        self.assertEqual(classify('unsupported library fixture: explicit seed'),
                         'unsupported_harness_or_engine_operation')


if __name__ == '__main__':
    unittest.main()
