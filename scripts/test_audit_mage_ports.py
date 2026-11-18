import json
from pathlib import Path
import tempfile
import unittest

from audit_mage_ports import ROOT, classify, completed_files_from_reports, match_planned_rows, parse_tap, port_inventory
from triage_mage_audit import review_family


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

    def test_skip_keeps_the_reason_without_reporting_a_pass(self):
        rows = parse_tap('# Subtest: Fixture.java :: java_only\n'
                         'ok 1 - Fixture.java :: java_only # SKIP Java API contract\n')
        self.assertEqual(rows[0]['status'], 'skipped')
        self.assertEqual(rows[0]['skip_reason'], 'Java API contract')

    def test_node_tap_escaped_operation_keeps_java_source_and_hash_mark(self):
        rows = parse_tap(
            '# Subtest: source :: failed\nnot ok 1 - source :: failed\n'
            "  failureType: 'testCodeFailure'\n  error: 'expected player''s card'\n"
            r'# mage-operation: {"op":"unsupported","source":"check(\\"x\#1\\")"}' + '\n'
            '# Subtest: source :: next\nok 2 - source :: next\n'
        )
        self.assertEqual(rows[0]['operation'], {'op': 'unsupported', 'source': 'check("x#1")'})
        self.assertEqual(rows[0]['error'], "expected player's card")
        self.assertEqual(rows[1]['status'], 'passed')

    def test_failures_are_conservative_categories_not_confirmed_engine_bugs(self):
        self.assertEqual(classify('create clause missing token [rule-path=clause-reading]'),
                         'parser_failure_before_expected_outcome')
        self.assertEqual(classify('dispatch failed: Resolution failed: Cannot resolve value'),
                         'runtime_error_candidate')
        self.assertEqual(classify('expected life 16, got 19'),
                         'outcome_or_fixture_mismatch_candidate')
        self.assertEqual(classify('unsupported library fixture: explicit seed'),
                         'unsupported_harness_or_engine_operation')

    def test_empty_ports_cannot_pass_the_audit_and_repeats_inherit_their_fixture(self):
        spec = {'sourcePath': 'Fixture.java', 'tests': [
            {'name': 'empty', 'operations': []},
            {'name': 'fixture', 'operations': [{'op': 'assertLife'}]},
            {'name': 'repeat', 'operations': [], 'repeat': {'name': 'fixture', 'seeds': [1, 2]}},
            {'name': 'no_seeds', 'repeat': {'name': 'fixture', 'seeds': []}},
        ]}
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            port = Path(directory) / 'fixture.test.mjs'
            port.write_text(f'registerPortedMageTests({json.dumps(spec)});')
            inventory = port_inventory([port])
        rows = [{'test': scenario['test'], 'status': 'passed'} for scenario in inventory]
        self.assertEqual(match_planned_rows(rows, inventory), [])
        self.assertEqual([row['status'] for row in rows], ['failed', 'passed', 'passed', 'failed'])
        self.assertEqual(rows[2]['declared_direct_assertions'], 1)
        self.assertEqual(rows[0]['raw_status'], 'passed')
        self.assertEqual(review_family(rows[0]['classification'], rows[0]['error']),
                         'untranslated_java_test_body')


if __name__ == '__main__':
    unittest.main()
