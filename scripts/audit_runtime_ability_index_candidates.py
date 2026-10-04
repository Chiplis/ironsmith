#!/usr/bin/env python3
"""Find typed condition-bearing static abilities before activated/mana entries."""
from collections import Counter
from pathlib import Path
import datetime
import hashlib
import json
import sqlite3

ROOT=Path('reports/runtime-audit')
RUN='267a16aff3b321196397d0b4'

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def conditions(value,path):
    found=[]
    if isinstance(value,dict):
        for key,child in value.items():
            here=f'{path}.{key}'
            if key=='condition' and child is not None:
                found.append({'path':here,'value':child})
            elif isinstance(child,(dict,list)):
                found.extend(conditions(child,here))
    elif isinstance(value,list):
        for index,child in enumerate(value):found.extend(conditions(child,f'{path}[{index}]'))
    return found

def main():
    db=sqlite3.connect((ROOT/'corpus/results.sqlite3').resolve().as_uri()+'?mode=ro',uri=True)
    db.execute('pragma query_only=ON');db.execute('begin')
    rows=[];counts=Counter();covered=set(json.loads((ROOT/'summary.json').read_text())['confirmed_card_outcomes']['confirmed_failure_cards'])
    for name,status,raw in db.execute('select card_name,status,result_json from result where run_id=? order by card_name',(RUN,)):
        counts['payloads']+=1;counts[f'status_{status}']+=1;record=json.loads(raw);definition=record.get('definition')
        if not definition:continue
        counts['definitions_inspected']+=1
        abilities=definition.get('abilities',[])
        for index,ability in enumerate(abilities):
            static=ability.get('kind',{}).get('Static')
            if static is None:continue
            gates=conditions(static,f'abilities[{index}].kind.Static')
            if not gates:continue
            later=[{'index':i,'kind':next(iter(a['kind']))} for i,a in enumerate(abilities[index+1:],start=index+1) if isinstance(a.get('kind'),dict) and any(k in a['kind'] for k in ['Activated','Mana'])]
            if later:rows.append({'card':name,'static_index':index,'static_id':static.get('id'),'conditions':gates,'later_action_abilities':later,'artifact_checksum':record.get('artifact_checksum'),'already_has_any_confirmed_outcome':name in covered,'status':'structural_candidate_only'})
    cards=sorted({r['card'] for r in rows})
    source_paths=['crates/ironsmith-engine/src/continuous.rs','crates/ironsmith-engine/src/decision/legal_actions.rs','crates/ironsmith-engine/src/game_loop/priority_apply.rs','crates/ironsmith-engine/src/game_state/zones_and_characteristics.rs']
    report={'scope':'Typed structural candidate screen over retained full frozen definitions. A non-null condition under a Static ability precedes an Activated or Mana entry. This does not prove that the static is removed by is_active, that either condition state is reachable, or that the advertised index differs at execution. No cards are promoted by this screen.','generated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'counts':dict(counts),'candidate_paths':len(rows),'candidate_payload_names':len(cards),'candidate_names':cards,'rows':rows,'coverage_gaps':['Only explicit serialized condition fields are screened; implicit static conditions and granted/reconfigured abilities need separate checks.','Only top-level ability order is screened, not abilities nested in grants or linked-face transitions.','Candidate input names may include identical front aliases; they are not merged automatically.'],'provenance':{'run_id':RUN,'frozen_manifest_sha256':sha(ROOT/'corpus'/RUN/'manifest.json'),'summary_sha256':sha(ROOT/'summary.json'),'script_sha256':sha(Path(__file__)),'source_references':[{'path':p,'sha256':sha(Path(p))} for p in source_paths]}}
    (ROOT/'ability-index-structural-candidates.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:report[k] for k in ['counts','candidate_paths','candidate_payload_names','candidate_names']}))
if __name__=='__main__':main()
