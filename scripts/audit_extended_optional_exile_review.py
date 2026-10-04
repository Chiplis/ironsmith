#!/usr/bin/env python3
"""Review strict canonical Escape pair payments and independent outcomes."""
import collections,hashlib,json,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'reports/runtime-audit';prefix='extended-optional-exile'
read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
def write(n,d):(p/n).write_text(json.dumps(d,indent=2)+'\n')
def strip(x):
 if isinstance(x,list):return[strip(v)for v in x]
 if isinstance(x,dict):return{k:strip(v)for k,v in x.items()if not(k=='id'and'name'in x and'card_types'in x)}
 return x
d=read(prefix+'-reproductions.json');proc=read(prefix+'-process.json');assert proc['exit_code']==0 and proc['binary_sha256_before']==proc['binary_sha256_after']==d['provenance']['binary_sha256'];assert sha(root/'crates/ironsmith-tools/tests/runtime_extended_optional_exile_reproductions.rs')==d['provenance']['source_sha256']
artifacts=read(prefix+'-artifacts.json');parity=[]
for corpus,run in [('corpus','267a16aff3b321196397d0b4'),('actions','e17a4980b0b92c7a5a4cead2')]:
 with sqlite3.connect(f'file:{p/corpus/"results.sqlite3"}?mode=ro',uri=True)as db:
  for a in artifacts:
   result=db.execute('select result_json from result where run_id=? and card_name=?',(run,a['card'])).fetchone();assert result,a['card'];f=json.loads(result[0]);equal=strip(a['definition'])==strip(f['definition']);assert equal,a['card'];parity.append(dict(card=a['card'],corpus=corpus,run_id=run,artifact_checksum=a['artifact_checksum'],frozen_checksum=f['artifact_checksum'],definition_equal_ignoring_only_card_ids=equal))
write(prefix+'-parity.json',dict(scope='Full strict canonical definitions match both frozen corpora ignoring only definition card IDs. No metadata or engine behavior alterations.',rows=parity))
counts=collections.Counter(r['status']for r in d['rows']);assert counts==dict(expected_result_observed=36,semantic_mismatch=1)
r=d['rows'][32];assert r['card']=='Ruthless Radrat'and r['scenario']['variant']=='surplus'and r['expected']['cost']==r['actual']['cost']and r['actual']['outcome']==dict(radrat_permanents=2,two_payments_offered=False)
assert next(a for a in artifacts if a['card']=='Ruthless Radrat')['definition']['optional_costs'][0]['repeatable'] is True
reason='All tested actual selected payments and resulting gameplay states match. Ruthless Radrat has one non-promoted interface diagnostic: the strict optional Squad cost is repeatable, but SelectOptionsContext offers repeatable:false/max_count1 with eight graveyard cards. The fixture selects the one offered payment, exiles exactly four cards, and observes the source plus one token. Two-payment execution was not forced or reached; no claim is made that the engine would reject an otherwise externally submitted repeated selection. This metadata discrepancy is retained separately from confirmed runtime defects.'
excluded=[dict(path='reports/runtime-audit/excluded-pilots/extended-optional-squad-count/validity.json',reason='Initial surplus oracle incorrectly expected two payments after one actual selection; original raw/source/binary preserved and excluded.'),dict(path='reports/runtime-audit/excluded-pilots/extended-optional-stale-binary/validity.json',reason='An inadvertent stale-binary rerun after failed compilation is fully excluded. Final replay follows successful fresh compilation and source/binary hash verification.')]
d.update(summary=dict(paths=9,scenarios=37,fully_matching_rows=36,unpromoted_interface_diagnostics=1,actual_cost_and_game_state_controls=37,confirmed_cards=[],strict_definitions=len(artifacts)),confirmed_cards=[],reviewed_scope='Nine exact Exile cost paths have scoped actual selected payment/outcome controls: six optional collect-evidence costs, collect-evidence ward, Detective\'s Phoenix bestow from hand and actual graveyard, and one Ruthless squad payment. Actual normal casts/declines and insufficient-resource cases are controls only. Repeated squad payment is unexecuted and not promoted.',parity_report=f'reports/runtime-audit/{prefix}-parity.json',process_report=f'reports/runtime-audit/{prefix}-process.json',excluded_pilots=excluded)
write(prefix+'-reproductions.json',d);ref=dict(path=f'reports/runtime-audit/{prefix}-reproductions.json',sha256=sha(p/(prefix+'-reproductions.json')))
paths=[]
for n in sorted({r['card']for r in d['rows']}):
 ix=[i for i,r in enumerate(d['rows'])if r['card']==n];r=d['rows'][ix[0]]
 paths.append(dict(card=n,cost_path=r['scenario']['cost_path'],consumer_path=r['scenario']['consumer_path'],status='paid_cost_scoped_controls_repeat_capacity_unexecuted'if n=='Ruthless Radrat'else'paid_cost_and_scoped_outcome_controls',source_report=ref,source_rows=ix,scope=reason if n=='Ruthless Radrat'else'Actual selected cost, printed mana, exact resource exile and oracle-derived resulting state. Insufficient optional costs are declined; only the selected and offered methods are executed.'))
write(prefix+'-reviewed-attribution.json',dict(confirmed_cards=[],summary=d['summary'],findings=[],source_report=ref,reason=reason,path_coverage=paths,non_promoted_diagnostics=[dict(card_name='Ruthless Radrat',confirmed_cards=[],classification='unverified_interface_diagnostic',source_report=ref,source_row=32,expected=d['rows'][32]['expected'],observed=d['rows'][32]['actual'],reason=reason)],parity_report=d['parity_report'],process_report=d['process_report'],limitations=d['limitations'],excluded_pilots=excluded))
(p/(prefix+'-reproductions.md')).write_text('# Optional, ward and bestow exile-cost controls\n\n37 cases across nine exact paths. Every selected payment and resulting game state matches. One repeat-capacity interface discrepancy remains unpromoted.\n\n'+reason+'\n\n'+d['reviewed_scope']+'\n\n'+f'{len(artifacts)} definitions match both frozen corpora ({len(parity)} comparisons). Stable fresh binary and source hashes verified. Two excluded preliminary runs are preserved separately.\n')
print(d['summary'])
