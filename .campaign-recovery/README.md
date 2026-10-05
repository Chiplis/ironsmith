# Card-campaign recovery archive

This branch preserves committed source in Git bundles after temporary workspace loss. Every checkpoint is **UNVALIDATED**: no builds, tests, compiler probes, corpus runs, or engine execution are implied. The manifest records each bundle's prerequisite base, exact refs, byte count, SHA-256, and Git blob SHA. Earlier bundles remain available.

Archive files belong only on `checkpoint/card-campaign-20261005-recovery-bundles`; keep `.campaign-recovery/` out of source PRs and `main`. Recover source refs from a bundle, rather than merging this archive branch.

## Restore the latest complete snapshot

Run from a clone of `Chiplis/ironsmith` with `origin` pointing to that repository. These commands only read/import Git objects and write a temporary bundle; they do not execute project code or overwrite existing user branches.

```sh
git fetch --no-tags origin refs/heads/checkpoint/card-campaign-20261005-recovery-bundles
recovery_archive=$(git rev-parse FETCH_HEAD)
recovery_dir=$(mktemp -d)
git show "$recovery_archive:.campaign-recovery/manifest.json" > "$recovery_dir/manifest.json"

python3 - "$recovery_archive" "$recovery_dir" <<'PY'
import hashlib, json, pathlib, subprocess, sys, uuid

archive, directory = sys.argv[1], pathlib.Path(sys.argv[2])
manifest = json.loads((directory / 'manifest.json').read_text())
# To restore an older multi-ref snapshot, select its exact `file` here instead.
entry = next(item for item in reversed(manifest['checkpoints']) if 'refs' in item)
base = entry['requires_base']
subprocess.run(['git', 'cat-file', '-e', base + '^{commit}'], check=True)
data = subprocess.check_output(['git', 'show', archive + ':.campaign-recovery/' + entry['file']])
if len(data) != entry['bytes'] or hashlib.sha256(data).hexdigest() != entry['sha256']:
    raise SystemExit('Bundle byte count or SHA-256 mismatch; stop.')
bundle = directory / 'source.bundle'
bundle.write_bytes(data)
blob = subprocess.check_output(['git', 'hash-object', str(bundle)], text=True).strip()
if blob != entry['blob_sha']:
    raise SystemExit('Git blob SHA mismatch; stop.')
subprocess.run(['git', 'bundle', 'verify', str(bundle)], check=True)
heads = subprocess.check_output(['git', 'bundle', 'list-heads', str(bundle)], text=True)
actual = {line.split()[1]: line.split()[0] for line in heads.splitlines()}
expected = {(ref if ref.startswith('refs/heads/') else 'refs/heads/' + ref): sha
            for ref, sha in entry['refs'].items()}
if actual != expected:
    raise SystemExit('Bundle refs differ from the manifest; stop.')
namespace = 'refs/recovery/card-campaign-' + uuid.uuid4().hex
existing = subprocess.check_output(['git', 'for-each-ref', '--format=%(refname)', namespace + '/'], text=True)
if existing.strip():
    raise SystemExit('Restore namespace already exists; stop.')
subprocess.run(['git', '-c', 'gc.auto=0', 'fetch', '--no-tags', '--no-write-fetch-head',
                str(bundle), 'refs/heads/*:' + namespace + '/*'], check=True)
for ref, sha in sorted(expected.items()):
    restored = namespace + '/' + ref.removeprefix('refs/heads/')
    actual_sha = subprocess.check_output(['git', 'rev-parse', restored], text=True).strip()
    if actual_sha != sha:
        raise SystemExit('Restored ref mismatch: ' + restored)
    print(sha, restored)
print('Verified prerequisite base:', base)
print('Restored bundle:', entry['file'])
print('Temporary files:', directory)
PY
```

The printed `refs/recovery/card-campaign-.../` namespace is fresh and contains the exact saved commits. Inspect them with `git show <printed-ref>` or `git log <printed-ref>`; decide separately which source changes to integrate. No checkout, merge, force push, deletion, release tag, or workflow dispatch is part of recovery.

If the prerequisite-base check fails, fetch the listed base from `origin` (`git fetch --no-tags origin <requires_base>`) or use a full clone, then rerun. Stop on any hash, ref, or bundle-integrity mismatch. The two oldest single-commit bundles also remain listed in the manifest; inspect their heads with `git bundle list-heads` if restoring those specifically.
