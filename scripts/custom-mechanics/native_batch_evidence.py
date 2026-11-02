"""Consume the parent helper's actual pre-build input manifest; never restamp it."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT_INPUTS = ('crates', 'vendor', 'fixtures/custom_mechanics', 'scripts/custom-mechanics', 'web/ui/src/lib')
EXTRA_INPUTS = ('Cargo.toml', 'Cargo.lock', '.cargo/config.toml')

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def current_input_paths(root):
    roots = [name for name in ROOT_INPUTS if (root/name).exists()]
    if roots:
        result = subprocess.run(['rg', '--files', *roots], cwd=root, text=True, capture_output=True)
        if result.returncode not in (0, 1):
            raise ValueError('cannot enumerate current native input inventory')
        paths = set(result.stdout.splitlines())
    else:
        paths = set()
    paths.update(name for name in EXTRA_INPUTS if (root/name).is_file())
    return paths

def read_native_batch(root, manifest_path):
    root=Path(root).resolve();manifest_path=Path(manifest_path)
    manifest=json.loads(manifest_path.read_text());errors=[];inputs={}
    if manifest.get('fingerprints_captured_before_build') is not True:
        errors.append('batch has no actual pre-build capture declaration')
    name=manifest.get('input_fingerprints')
    packet=Path(name) if name else None
    if packet is not None and not packet.is_absolute():packet=root/packet
    if packet is None or not packet.is_file() or sha(packet)!=manifest.get('input_fingerprints_sha256'):
        errors.append('missing or changed pre-build fingerprint packet')
    else:
        for item in json.loads(packet.read_text()):
            name=item.get('path');expected=item.get('sha256')
            if not isinstance(name,str) or Path(name).is_absolute() or '..' in Path(name).parts:
                errors.append('invalid captured input path');continue
            if name in inputs:errors.append('duplicate captured input: '+name)
            inputs[name]=expected
            file=root/name
            if not file.is_file() or sha(file)!=expected:errors.append('changed captured input: '+name)
        if set(inputs)!=current_input_paths(root):errors.append('current native input file inventory differs from pre-build capture')
    rows_path=manifest_path.with_name('results.jsonl')
    rows=[json.loads(line) for line in rows_path.read_text().splitlines() if line.strip()] if rows_path.exists() else []
    targets=[item.get('target') for item in rows]
    runs=[]
    for item in rows:
        reasons=list(errors);target=item.get('target')
        if target not in manifest.get('targets',[]):reasons.append('result target absent from captured batch')
        if targets.count(target)!=1:reasons.append('multiple result attempts for target in one batch')
        run={'target':target,'exit_code':item.get('exit'),'log':item.get('log'),'log_sha256':item.get('log_sha256'),
             'inputs_sha256':inputs,'native_batch_manifest':str(manifest_path),'native_batch_manifest_sha256':sha(manifest_path),
             'input_fingerprints':str(packet) if packet else None,
             'input_fingerprints_sha256':manifest.get('input_fingerprints_sha256'),
             'fingerprints_captured_before_build':manifest.get('fingerprints_captured_before_build') is True}
        runs.append((run,reasons))
    return runs
