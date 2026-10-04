#!/usr/bin/env python3
"""Bind full-corpus simultaneous-action leads to reviewed reachable outcomes."""
from collections import Counter, defaultdict
from pathlib import Path
import datetime
import hashlib
import json

ROOT = Path('reports/runtime-audit')

def read(path):
    raw=path.read_bytes()
    return json.loads(raw), {'path':str(path),'sha256':hashlib.sha256(raw).hexdigest()}

def main():
    candidates=defaultdict(list);sources=[]
    for filename in ['corpus-triage-ledger.jsonl','actions-triage-ledger.jsonl']:
        path=ROOT/filename;raw=path.read_bytes();sources.append({'path':str(path),'sha256':hashlib.sha256(raw).hexdigest()})
        for row in map(json.loads,raw.splitlines()):
            for finding in row['findings']:
                if finding['family']=='unsupported_simultaneous_action':
                    candidates[row['card']].append({'ledger':filename,'source_path':finding['source_path'],'observation':finding['observation'],'artifact_checksum':row['artifact_checksum']})
    path=ROOT/'confirmed-outcomes.jsonl';raw=path.read_bytes();sources.append({'path':str(path),'sha256':hashlib.sha256(raw).hexdigest()})
    observed=defaultdict(list);other=defaultdict(list)
    for result in map(json.loads,raw.splitlines()):
        row=result['row'];names=row.get('confirmed_cards') or [row.get('card')]
        actual=row.get('actual',row.get('observed',{}))
        group=observed if 'generic each-player action lacks simultaneous proposal support' in json.dumps(actual) else other
        for name in names:
            if name:
                group[name].append({'report':result['source'],'row_index':result['source_row'],'category':result['category'],'scenario':row.get('scenario'),'actual':actual})
    inventory,desc=read(ROOT/'corpus/267a16aff3b321196397d0b4/inventory.json');sources.append(desc)
    payloads={p['name']:p for p in inventory['cards']}
    explicit_aliases={
        'Aclazotz, Deepest Betrayal // Temple of the Dead':'Aclazotz, Deepest Betrayal',
        'Etali, Primal Conqueror // Etali, Primal Sickness':'Etali, Primal Conqueror',
    }
    alias_rows=[];aliases={}
    for name,front in explicit_aliases.items():
        a,b=payloads[name],payloads[front]
        effective_a=a.get('parse_name') or a['name'];effective_b=b.get('parse_name') or b['name']
        equal=effective_a==effective_b and a['parse_input']==b['parse_input']
        assert equal,(name,front)
        alias_rows.append({'inventory_name':name,'compiled_front_name':front,'effective_parse_name':effective_a,'canonical_compile_input_identical':equal,'parse_input_sha256':hashlib.sha256(a['parse_input'].encode()).hexdigest(),'scope':'Same front compilation input only. No linked-face or transform execution is asserted; no extra confirmed card is added.'})
        aliases[name]=front
    rows=[]
    for name in sorted(candidates):
        lookup=aliases.get(name,name);matches=observed[lookup]
        early_legality = [r for r in other[name] if r['category']=='silent_wrong_result' and r['actual'].get('cast_offered') is False]
        status=('same_failure_family_reproduced_via_identical_front_input' if name in aliases else 'same_failure_family_reproduced') if matches else 'earlier_legality_failure_reproduced_branch_unreached' if name=='Fatal Grudge' and early_legality else 'unreviewed'
        rows.append({'card':name,'candidate_findings':candidates[name],'coverage_status':status,'input_alias_of':aliases.get(name),'same_family_observation_references':matches,'earlier_failure_references':early_legality if name=='Fatal Grudge' else [],'every_candidate_path_executed':False})
    report={'scope':'Coverage of the unsupported_simultaneous_action family in both complete immutable 33,874-payload campaigns. Reviewed occurrences reproduce a family-level reachable failure; they do not prove every flagged path was executed. Fatal Grudge is blocked earlier at legal-action discovery. Explicit front aliases are not extra card confirmations.','generated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'all_cards_verified':False,'all_candidate_branches_verified':False,'candidate_payload_names':len(rows),'coverage_counts':dict(Counter(r['coverage_status'] for r in rows)),'rows':rows,'front_input_aliases':alias_rows,'sources':sources,'provenance':{'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'worker_front_name_rule':'audit_runtime_worker.rs builds with request.parse_name.as_deref().unwrap_or(&request.name); equivalent effective name and full parse_input are checked above.'}}
    (ROOT/'simultaneous-family-coverage.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report['coverage_counts']))

if __name__=='__main__':main()
