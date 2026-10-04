#!/usr/bin/env python3
"""Path-specific ledger for extended Exile cost pairs and three assigned siblings."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
inventory=read('extended-cost-dependency-candidates.json')
selected=[r for r in inventory['rows']if r['consumer_kind']=='ExileEffect'or r['card']in['Anurid Scavenger','Phyrexian Dreadnought','Firecat Blitz']]
assert len(selected)==47
prior=read('nonmana-x-spell-reviewed-attribution.json');raw=read('nonmana-x-spell-final-execution.json')
reuse=[]
for group in ['findings','controls']:
 for row in prior[group]:
  if row.get('card')in['Firecat Blitz','Flash of Insight','Summons of Saruman']:
   ref=row['source_report'];assert sha(root/ref['path'])==ref['sha256'];reuse.append(row)
comp={r['card']:r for r in raw['compilation']};parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for n in ['Firecat Blitz','Flash of Insight','Summons of Saruman']:
   frozen=json.loads(db.execute('select result_json from result where run_id=? and card_name=?',(run,n)).fetchone()[0]);assert strip(comp[n]['definition'])==strip(frozen['definition'])
   parity.append(dict(card=n,run_id=run,corpus=corpus,artifact_checksum=comp[n]['artifact_checksum'],definition_equal_ignoring_only_card_ids=True))
rows=[]
for r in selected:
 evidence=[x for x in reuse if x['card']==r['card']]
 status='unexercised';scope='No executed exact-path evidence credited.'
 if evidence:
  if r['card']=='Firecat Blitz':
   status='reviewed_missing_search_zone_before_payment';scope='Actual paid source cast, actual offered flashback X0 fails selector before payment. Requested positive X was clamped to0; positive-X consumer outcome remains unreached.'
  else:
   status='reviewed_paid_flashback_cost_and_scoped_outcomes';scope='Actual paid normal source cast then actual offered flashback X0 andX2 pays printed fixed mana, exiles exact resource count and source, verifies card-zone/library/token outcomes. Other resources/targets and Saruman optional mill-cast are not certified.'
 rows.append(dict(**r,coverage_status=status,scope=scope,evidence=[dict(source_report=x['source_report'],source_row=x['source_row'],expected=x['expected'],observed=x['observed'])for x in evidence]))
for report_name in ['extended-escape-reviewed-attribution.json','extended-alternate-exile-reviewed-attribution.json','extended-optional-exile-reviewed-attribution.json','extended-trigger-cost-reviewed-attribution.json']:
 if not(p/report_name).exists():continue
 review=read(report_name)
 for coverage in review['path_coverage']:
  ref=coverage['source_report'];assert sha(root/ref['path'])==ref['sha256']
  matches=[r for r in rows if r['card']==coverage['card']and r['path']==coverage['cost_path']and r['consumer_path']==coverage['consumer_path']]
  assert len(matches)==1
  matches[0].update(coverage_status=coverage['status'],scope=coverage['scope'],evidence=[dict(source_report=ref,source_rows=coverage['source_rows'],reviewed_report=report_name)])
report=dict(scope='Exact47 assigned extended total-cost paths:44 ExileEffect consumers plus Anurid Scavenger, Phyrexian Dreadnought and Firecat Blitz. Evidence binds exact cost context and path, never a whole-card verdict. Absent actions are not forced. Reused rows preserve earlier gates.',inventory=dict(path='reports/runtime-audit/extended-cost-dependency-candidates.json',sha256=sha(p/'extended-cost-dependency-candidates.json')),summary=dict(paths=len(rows),statuses=dict(collections.Counter(r['coverage_status']for r in rows))),reused_parity=parity,rows=rows)
(p/'extended-exile-cost-coverage.json').write_text(json.dumps(report,indent=2)+'\n')
print(report['summary'])
