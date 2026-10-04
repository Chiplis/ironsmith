import tempfile
import unittest
from pathlib import Path

from audit_mage_ports import ROOT, fingerprint
from finalize_mage_checkpoint import verify_fingerprints


class CheckpointEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=ROOT / 'reports/runtime-audit')
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / 'evidence.tap'
        self.path.write_text('ok 1 - actual scenario\n')

    def test_unchanged_evidence_preserves_recorded_shape(self):
        complete = fingerprint(self.path)
        no_mtime = {key: value for key, value in complete.items() if key != 'mtime_ns'}
        self.assertEqual([complete, no_mtime], verify_fingerprints([complete, no_mtime]))

    def test_changed_tap_result_is_rejected(self):
        recorded = fingerprint(self.path)
        self.path.write_text('not ok 1 - actual scenario\n')
        with self.assertRaisesRegex(ValueError, 'changed evidence'):
            verify_fingerprints([recorded])

    def test_missing_evidence_is_rejected(self):
        recorded = fingerprint(self.path)
        self.path.unlink()
        with self.assertRaises(FileNotFoundError):
            verify_fingerprints([recorded])


if __name__ == '__main__':
    unittest.main()
