"""Pre-build packet validation is read-only and cannot restamp legacy batches."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from native_batch_evidence import read_native_batch,current_input_paths

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()

class NativeBatchEvidenceTests(unittest.TestCase):
    def batch(self,directory):
        root=Path(directory)
        for name in ['crates/code.rs','vendor/input.txt','fixtures/custom_mechanics/probe.json','scripts/custom-mechanics/report.py','web/ui/src/lib/source.js','Cargo.toml','Cargo.lock','.cargo/config.toml']:
            file=root/name;file.parent.mkdir(parents=True,exist_ok=True);file.write_text(name)
        output=root/'output';output.mkdir()
        packet=output/'input-fingerprints.json'
        packet.write_text(json.dumps([{'path':name,'sha256':sha(root/name)} for name in sorted(current_input_paths(root))]))
        manifest=output/'batch.json';manifest.write_text(json.dumps({'targets':['custom_probe'],'fingerprints_captured_before_build':True,'input_fingerprints':str(packet),'input_fingerprints_sha256':sha(packet)}))
        (output/'results.jsonl').write_text(json.dumps({'target':'custom_probe','exit':0,'log':str(output/'custom_probe.log'),'log_sha256':'captured-log-hash'})+'\n')
        return root,manifest

    def test_real_prebuild_packet_supplies_captured_inputs_without_invented_code_stamp(self):
        with tempfile.TemporaryDirectory() as directory:
            root,manifest=self.batch(directory);run,errors=read_native_batch(root,manifest)[0]
            self.assertEqual(errors,[])
            self.assertEqual(run['inputs_sha256']['crates/code.rs'],sha(root/'crates/code.rs'))
            self.assertNotIn('code_sha256',run)

    def test_changed_or_added_current_source_invalidates_prebuild_identity(self):
        for action in ['change','add']:
            with tempfile.TemporaryDirectory() as directory:
                root,manifest=self.batch(directory)
                (root/('crates/code.rs' if action=='change' else 'crates/added.rs')).write_text('new contents')
                _,errors=read_native_batch(root,manifest)[0]
                self.assertTrue(errors)

    def test_missing_prebuild_declaration_cannot_be_inferred_from_present_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root,manifest=self.batch(directory);data=json.loads(manifest.read_text());del data['fingerprints_captured_before_build'];manifest.write_text(json.dumps(data))
            _,errors=read_native_batch(root,manifest)[0]
            self.assertIn('batch has no actual pre-build capture declaration',errors)

    def test_legacy_batch_is_not_retroactively_stamped(self):
        with tempfile.TemporaryDirectory() as directory:
            root,manifest=self.batch(directory);manifest.write_text(json.dumps({'targets':['custom_probe']}))
            run,errors=read_native_batch(root,manifest)[0]
            self.assertTrue(errors)
            self.assertEqual(run['inputs_sha256'],{})
            self.assertFalse(run['fingerprints_captured_before_build'])
if __name__=='__main__':unittest.main()
