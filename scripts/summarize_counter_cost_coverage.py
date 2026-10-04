#!/usr/bin/env python3
"""Join typed counter-cost scope with reviewed cases, without inferring whole-card correctness."""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'reports/runtime-audit'
SOURCES=['counter-choice-reviewed-classification.json','counter-choice-sibling-reviewed-classification.json','variable-counter-cost-reviewed-classification.json','counter-cost-misc-reviewed-classification.json','counter-cost-remaining-reviewed-attribution.json','counter-cost-final-reviewed-classification.json','flitterwing-delayed-draw-reproductions.json']

def main():
    candidate=BASE/'counter-removal-cost-candidates.json'
    inventory=json.loads(candidate.read_text())
    evidence={}
    manifests=[]
    for name in SOURCES:
        path=BASE/name
        if not path.exists():
            continue
        report=json.loads(path.read_text())
        manifests.append({'path':name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
        for section in ['rows','controls','legality_candidates']:
            for index,row in enumerate(report.get(section,[])):
                evidence.setdefault(row['card'],[]).append({'report':name,'section':section,'row_index':index,'scenario':row.get('scenario'),'classification':row.get('classification',row.get('reviewed_classification','unreviewed'))})
    by_name={}
    for row in inventory['rows']:
        by_name.setdefault(row['card'],[]).append(row)
    rows=[]
    for name,paths in sorted(by_name.items()):
        observations=evidence.get(name,[])
        classes={r['classification'] for r in observations}
        status='unexercised_in_these_reports'
        if classes & {'runtime_defect_card_reproduced','compiler_semantic_defect_card_reproduced'}:
            status='scoped_defect_reproduced'
        elif classes & {'sampled_semantic_pass','expected_outcome_passed','expected_result_control'}:
            status='scoped_controls_only'
        elif observations:
            status='incomplete_or_unreviewed'
        rows.append({'card':name,'typed_cost_paths':paths,'scoped_status':status,'case_count':len(observations),'reviewed_cases':observations})
    from collections import Counter
    output={'scope':'Every RemoveAnyCountersAmongEffect cost payload in pinned compiled definitions joined only with explicitly reviewed native cases. A sampled pass never clears a card or every cost branch.','inventory':{'path':candidate.name,'sha256':hashlib.sha256(candidate.read_bytes()).hexdigest(),'records_scanned':inventory['records_scanned'],'retained_definitions':inventory['retained_definitions'],'candidate_paths':inventory['candidate_paths'],'candidate_names':inventory['candidate_names']},'reviewed_sources':manifests,'summary':dict(Counter(r['scoped_status'] for r in rows)),'case_count':sum(r['case_count'] for r in rows),'case_classifications':dict(Counter(case['classification'] for r in rows for case in r['reviewed_cases'])),'rows':rows,'limitations':['Native results are tied to each report binary; fresh-definition parity does not establish historical runtime equality.','Typed RemoveCounters costs and other effect shapes are outside this inventory; some individual reports include adjacent branch controls.','Combined-name payload aliases remain independent inventory entries.','Unexercised here may have unrelated evidence elsewhere in the broader audit.']}
    (BASE/'counter-removal-cost-family-coverage.json').write_text(json.dumps(output,indent=2)+'\n')
    lines=['# Counter-removal cost scope','',f"All {inventory['records_scanned']:,} pinned records screened; typed scope covers {inventory['candidate_paths']} paths across {inventory['candidate_names']} payload names.",'','| Scoped evidence | Payload names |','|---|---:|']
    lines += [f'| {k} | {v} |' for k,v in output['summary'].items()]
    lines += ['',f"Reviewed case records: {output['case_count']}. These are limited scenarios, not whole-card or whole-path correctness claims.",'','| Card | Scoped status | Cases |','|---|---|---:|']
    lines += [f"| {r['card']} | {r['scoped_status']} | {r['case_count']} |" for r in rows]
    (BASE/'counter-removal-cost-family-coverage.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps({'summary':output['summary'],'cases':output['case_count']}))

if __name__=='__main__':
    main()
