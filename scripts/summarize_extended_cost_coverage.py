#!/usr/bin/env python3
"""Bind scoped reviewed executions to the extended TotalCost dependency inventory."""
from collections import Counter
import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];REPORTS=ROOT/'reports/runtime-audit'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 inventory=REPORTS/'extended-cost-dependency-candidates.json';data=json.loads(inventory.read_text());deps={str(inventory.relative_to(ROOT)):digest(inventory)}
 rows=[dict(r,coverage_level='unexercised_path',evidence=[]) for r in data['rows']];by_key={(r['card'],r['path'],r['consumer_path']):r for r in rows};assert len(by_key)==len(rows)==208
 for filename in ['spell-return-land-reviewed-classification.json','spell-web-return-reviewed-classification.json','return-morph-kicker-reviewed-classification.json','spell-alt-tap-reviewed-classification.json','spell-tap-flashback-reviewed-classification.json','ward-waterbend-reviewed-classification.json','optional-tap-outcome-reviewed-classification.json']:
  p=REPORTS/filename;review=json.loads(p.read_text());raw=REPORTS/review['source_report'];assert digest(raw)==review['source_sha256'];raw_data=json.loads(raw.read_text());deps[str(p.relative_to(ROOT))]=digest(p);deps[str(raw.relative_to(ROOT))]=digest(raw)
  for entry in review['path_coverage']:
   row=by_key[(entry['card'],entry['path'],entry['consumer_path'])]
   for i in entry['source_rows']:assert raw_data['rows'][i]['card']==entry['card']
   row['coverage_level']='scoped_reviewed';row['evidence'].append(dict(reviewed_report=str(p.relative_to(ROOT)),reviewed_sha256=digest(p),source_report=str(raw.relative_to(ROOT)),source_sha256=digest(raw),details=entry))
 for filename in ['extended-exile-cost-coverage.json']:
  p=REPORTS/filename;ledger=json.loads(p.read_text());assert ledger['inventory']['sha256']==digest(inventory);deps[str(p.relative_to(ROOT))]=digest(p)
  for entry in ledger['rows']:
   if entry['coverage_status']=='unexercised':continue
   for evidence in entry['evidence']:
    source=evidence['source_report'];raw=ROOT/source['path'];assert digest(raw)==source['sha256'];deps[source['path']]=source['sha256']
   row=by_key[(entry['card'],entry['path'],entry['consumer_path'])];row['coverage_level']='scoped_reviewed';row['evidence'].append(dict(ledger=str(p.relative_to(ROOT)),ledger_sha256=digest(p),details=entry))
 for filename in ['additional-withid-simple-path-coverage.json']:
  p=REPORTS/filename;ledger=json.loads(p.read_text());deps[str(p.relative_to(ROOT))]=digest(p)
  for entry in ledger['rows']:
   for evidence in entry['source_evidence']:
    source=evidence['source_report'];raw=ROOT/source['path'];assert digest(raw)==source['sha256'];deps[source['path']]=source['sha256']
   row=by_key[(entry['card'],entry['path'],entry['consumer_path'])];row['coverage_level']='scoped_reviewed';row['evidence'].append(dict(ledger=str(p.relative_to(ROOT)),ledger_sha256=digest(p),details=entry))
 result=dict(scope=__doc__,summary=dict(paths=len(rows),payload_names=len({r['card'] for r in rows}),coverage_levels=dict(Counter(r['coverage_level'] for r in rows))),rows=rows,dependencies=deps,generator_sha256=digest(Path(__file__)),limitations=['Reviewed coverage is bounded to explicit scenarios and may stop at a reproduced early payment failure.','This ledger creates no new card-defect promotions and cannot certify whole-card correctness.','Positive tag dependencies outside the listed structural scope remain outside this index.'])
 (REPORTS/'extended-cost-dependency-coverage.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result['summary'],indent=2))
if __name__=='__main__':main()
