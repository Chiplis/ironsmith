#!/usr/bin/env python3
"""Find optional spell costs that require choosing among payment branches."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / 'reports/runtime-audit'
RUN = '267a16aff3b321196397d0b4'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    db = sqlite3.connect(f'file:{REPORT}/corpus/results.sqlite3?mode=ro', uri=True)
    db.execute('BEGIN')
    rows, count = [], 0
    for name, status, raw in db.execute("select card_name,json_extract(result_json,'$.status'),json_extract(result_json,'$.definition.optional_costs') from result where run_id=?", (RUN,)):
        count += 1
        for index, cost in enumerate(json.loads(raw or '[]')):
            if 'OneOf' in cost.get('cost', {}).get('kind', {}):
                rows.append({'card': name, 'compile_status': status,
                             'path': f'$.optional_costs[{index}].cost.kind.OneOf',
                             'optional_cost': cost})
    db.close()
    sources = [ROOT / 'crates/ironsmith-engine/src/game_loop/priority_cast.rs',
               ROOT / 'crates/ironsmith-core/src/cost_model.rs']
    report = {
        'scope': 'Typed whole-corpus screen for optional spell costs with alternative OneOf payment branches. This report is a candidate list and does not execute cards.',
        'generated_at': datetime.now(timezone.utc).isoformat(),
        'run_id': RUN, 'scanned_rows': count, 'rows': rows,
        'runtime_contract': 'collect_spell_cost_steps extracts paid optional non-mana components through TotalCost::costs, whose precondition is All, while these costs contain OneOf. A legal paid optional-cost announcement is required to prove reachability.',
        'provenance': {'generator_sha256': digest(Path(__file__)),
                       'worker_manifest': json.loads((REPORT / 'corpus' / RUN / 'manifest.json').read_text()),
                       'reviewed_current_sources': [{'path': str(p), 'sha256': digest(p)} for p in sources]},
        'limitations': ['Rows whose strict compilation failed have no typed definition and cannot be checked.',
                        'A candidate is not an execution confirmation. Optional decline and each accepted payment branch need their own legal scenarios.',
                        'Current source hashes localize the mechanism; frozen-worker backtraces retain independent binary provenance.']}
    (REPORT / 'optional-oneof-cost-candidates.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'scanned': count, 'candidates': [r['card'] for r in rows]}))

if __name__ == '__main__':
    main()
