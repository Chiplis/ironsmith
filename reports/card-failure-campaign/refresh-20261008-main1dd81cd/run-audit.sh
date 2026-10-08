#!/bin/bash
set -euo pipefail
ROOT=$(pwd)
export RUSTUP_HOME="$ROOT/reports/current-refresh-20261008/toolchain/rustup"
export CARGO_HOME="$ROOT/reports/current-refresh-20261008/toolchain/cargo"
export PATH="$CARGO_HOME/bin:$PATH" PYTHONDONTWRITEBYTECODE=1
export CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=256 CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_OPT_LEVEL=1 RAYON_NUM_THREADS=1
python3 - <<'PY'
import hashlib,json,subprocess
from pathlib import Path
b=Path('reports/current-refresh-20261008')
assert (b/'build-exit-code.txt').read_text().strip()=='0'
git=lambda *a:subprocess.check_output(['git',*a],text=True).strip()
assert git('rev-parse','HEAD')=='1dd81cd84c62f272479f26e16d74719fff24b97b'
assert not git('status','--porcelain')
p={'source':{'commit':git('rev-parse','HEAD'),'tree':git('rev-parse','HEAD^{tree}'),'status':git('status','--porcelain')},'binary_sha256':hashlib.file_digest(open('target/release/sync_card_status_db','rb'),'sha256').hexdigest(),'build_command':['cargo','build','--locked','-p','ironsmith-tools','--bin','sync_card_status_db','--release'],'environment':{'CARGO_BUILD_JOBS':'1','CARGO_PROFILE_RELEASE_CODEGEN_UNITS':'256','CARGO_PROFILE_RELEASE_DEBUG':'0','CARGO_PROFILE_RELEASE_OPT_LEVEL':'1','RAYON_NUM_THREADS':'1'},'started_at':(b/'build-start.txt').read_text().strip(),'finished_at':(b/'build-finish.txt').read_text().strip(),'rustc_verbose':(b/'rustc-version.txt').read_text(),'cargo_verbose':(b/'cargo-version.txt').read_text()}
(b/'build-manifest.json').write_text(json.dumps(p,indent=2)+'\n')
PY
python3 scripts/card_failure_campaign.py run --repo "$ROOT" --cards "$ROOT/reports/current-refresh-20261008/data/cards-current.json" --out-dir "$ROOT/reports/current-refresh-20261008/audit-authoritative" --expected-commit 1dd81cd84c62f272479f26e16d74719fff24b97b --sync-bin "$ROOT/target/release/sync_card_status_db" --build-manifest "$ROOT/reports/current-refresh-20261008/build-manifest.json" > reports/current-refresh-20261008/audit-harness.stdout.log 2> reports/current-refresh-20261008/audit-harness.stderr.log
