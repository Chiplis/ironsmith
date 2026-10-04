#!/usr/bin/env python3
"""Summarize reviewed reports without promoting unexecuted typed candidates."""
import argparse
import json
from collections import Counter
from pathlib import Path
DEFAULT_REPORTS=['ability-damage-distribution-reproductions.json','damage-source-lki-reproductions.json','damage-source-event-reproductions.json','damage-source-etb-batch-reproductions.json','damage-source-noncreature-reproductions.json','damage-source-aura-reproductions.json','damage-source-death-reproductions.json','damage-source-combat-reproductions.json']
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--directory',default='reports/runtime-audit');ap.add_argument('--reports',nargs='*',default=DEFAULT_REPORTS);a=ap.parse_args();p=Path(a.directory)
    scan=json.loads((p/'damage-source-candidates.json').read_text());candidates=set(scan['tagged_or_event_source_cards']);by={};confirmed=set()
    for f in a.reports:
        d=json.loads((p/f).read_text());confirmed.update(d.get('confirmed_cards',[]))
        for r in d['rows']:
            if r['status']not in ('expected_result_observed','semantic_mismatch','resolution_failed'):
                continue
            n=r['card'];z=by.setdefault(n,{'card':n,'reports':set(),'cases':0,'status_counts':Counter(),'in_frozen_tagged_source_461_payload_screen':n in candidates,'typed_source_kinds':sorted({x['effective_source_kind']for x in scan['rows']if x['card']==n}),'numeric_tag_references':sorted({tag for x in scan['rows']if x['card']==n for tag in x.get('amount_tag_references',[])})});z['reports'].add(f);z['cases']+=1;z['status_counts'][r['status']]+=1
    for z in by.values():
        z['reports']=sorted(z['reports']);z['status_counts']=dict(z['status_counts']);z['classification']='runtime_defect_card_reproduced'if z['card']in confirmed else('candidate_requires_review'if z['status_counts'].get('semantic_mismatch',0)+z['status_counts'].get('resolution_failed',0)>0 else'control_only_for_executed_scenarios')
    report={'scope':'Reviewed actual gameplay coverage from focused source-reference/distributed-damage reports. Confirmation comes only from each reviewed report confirmed_cards. Unexecuted typed candidates are not promoted. Passing cases are limited controls, not exhaustive source-removal certification.','source_reports':a.reports,'frozen_tagged_source_payload_candidates':len(candidates),'reviewed_primary_cards_in_frozen_tagged_source_screen':len(candidates&by.keys()),'remaining_screen_payload_names_without_these_direct_evidence_rows':len(candidates-by.keys()),'total_reviewed_primary_cards':len(by),'total_executed_cases':sum(r['cases']for r in by.values()),'confirmed_primary_cards':len(confirmed&by.keys()),'rows':sorted(by.values(),key=lambda r:r['card']),'important_provenance_exception':'Gandalf, Spark Starter frozen event source is Source; pinned actions and native typed source is Tagged(triggering). Definition parity for each report is recorded in its own sibling files. Terror of the Peaks exercises a tagged numeric power reference with enclosing Source, outside the461 tagged-source-binding count.'}
    (p/'damage-source-reviewed-coverage.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k not in ('rows','scope','source_reports','important_provenance_exception')},indent=2))
if __name__=='__main__':main()
