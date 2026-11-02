"""Historical native logs are observations, never retroactive source receipts."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
SPEC=importlib.util.spec_from_file_location('source_case_bindings',Path(__file__).with_name('source-case-bindings.py'))
MODULE=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(MODULE)

class HistoricalObservationTests(unittest.TestCase):
    def fixture(self, directory, target='custom_probe', descriptor_target=None):
        root=Path(directory);log=root/'native.log'
        log.write_text(f'Running tests/{target}.rs (/tmp/binary)\n\nrunning 2 tests\ntest actual_case ... ok\ntest another_case ... FAILED\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored;\n')
        item={'target':descriptor_target or target,'log':str(log),'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'exit':101,'passed':1,'failed':1}
        descriptor=root/'results.json';descriptor.write_text(json.dumps({'results':[item]}))
        return root,log,descriptor

    def test_passed_case_in_failed_target_stays_historical_without_input_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,descriptor=self.fixture(directory)
            observation=MODULE.historical_observations(root,[descriptor])[0]
            self.assertTrue(observation['historical_log_integrity_verified'])
            self.assertEqual(observation['case_results'],{'actual_case':'passed','another_case':'failed'})
            self.assertFalse(observation['source_input_identity_verified'])
            self.assertFalse(observation['current_native_pass'])
            self.assertNotIn('code_sha256',observation)

    def test_changed_log_cannot_be_rebound_using_current_contents(self):
        with tempfile.TemporaryDirectory() as directory:
            root,log,descriptor=self.fixture(directory);log.write_text(log.read_text()+'changed')
            observation=MODULE.historical_observations(root,[descriptor])[0]
            self.assertFalse(observation['historical_log_integrity_verified'])
            self.assertEqual(observation['case_results'],{})

    def test_wrong_target_log_cannot_supply_named_case_results(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,descriptor=self.fixture(directory,descriptor_target='custom_unrelated')
            observation=MODULE.historical_observations(root,[descriptor])[0]
            self.assertFalse(observation['historical_log_integrity_verified'])
            self.assertEqual(observation['case_results'],{})

    def test_captured_summary_counts_must_match_actual_log(self):
        with tempfile.TemporaryDirectory() as directory:
            root,_,descriptor=self.fixture(directory);data=json.loads(descriptor.read_text());data['results'][0]['passed']=2;descriptor.write_text(json.dumps(data))
            observation=MODULE.historical_observations(root,[descriptor])[0]
            self.assertFalse(observation['historical_log_integrity_verified'])
            self.assertEqual(observation['case_results'],{})
if __name__=='__main__':unittest.main()
