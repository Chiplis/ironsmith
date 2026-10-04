import json
from pathlib import Path
import tempfile
import unittest

from audit_runtime_corpus import Worker, connect, summarize


class CorpusIsolationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.binary = self.root / "worker"
        self.binary.write_text('''#!/usr/bin/env python3
import json, os, sys, time
for line in sys.stdin:
    row = json.loads(line)
    if row['name'] == 'crash': os._exit(9)
    if row['name'] == 'hang': time.sleep(10)
    if row['name'] == 'noise': print('diagnostic text', flush=True)
    print(json.dumps({'name':row['name'], 'status':'compiled'}), flush=True)
''')
        self.binary.chmod(0o755)
        self.worker = Worker(self.binary, 3.0, self.root / "stderr.log")

    def tearDown(self):
        self.worker.close()
        self.temp.cleanup()

    def test_timeout_is_recorded_and_next_card_runs_in_a_new_process(self):
        self.assertEqual(self.worker.run({"name": "hang"})["status"], "timeout")
        self.assertEqual(self.worker.run({"name": "healthy"})["status"], "compiled")

    def test_native_crash_does_not_abort_remaining_cards(self):
        row = self.worker.run({"name": "crash"})
        self.assertEqual(row["status"], "worker_crashed")
        self.assertEqual(row["exit_code"], 9)
        self.assertEqual(self.worker.run({"name": "healthy"})["name"], "healthy")

    def test_unexpected_stdout_does_not_corrupt_card_identity(self):
        self.assertEqual(self.worker.run({"name": "noise"})["name"], "noise")
        self.assertIn("NON_PROTOCOL_STDOUT", (self.root / "stderr.log").read_text())

    def test_persistence_reports_unvisited_cards_and_semantics_as_unknown(self):
        db = connect(self.root / "audit.sqlite3")
        row = {"name": "healthy", "status": "compiled", "execution": [
            {"status": "executed"}, {"status": "not_matched"}
        ]}
        db.execute("insert into result(run_id,card_name,status,result_json) values (?,?,?,?)",
                   ("run", "healthy", "compiled", json.dumps(row)))
        db.commit()
        db.close()
        reopened = connect(self.root / "audit.sqlite3")
        summary = summarize(reopened, "run", {"payload_count": 2, "exclusions": [{"name": "excluded"}]}, self.root)
        self.assertEqual(summary["remaining"], 1)
        self.assertEqual(summary["excluded_source_names"], 1)
        self.assertEqual(summary["execution_observations"], {"executed": 1, "not_matched": 1})
        self.assertFalse(summary["all_cards_correct"])
        reopened.close()


if __name__ == "__main__":
    unittest.main()
