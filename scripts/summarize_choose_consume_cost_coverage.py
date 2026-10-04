#!/usr/bin/env python3
"""Join explicitly reviewed evidence onto each frozen choose/consume cost path.

No card-name-only observation outside the listed family reviews is credited.
A reviewed path can have a first-gate defect, scoped controls, or another
semantic issue; this ledger never infers that its consumer actually executed.
"""
import collections
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / 'reports/runtime-audit'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    dependencies = {}
    def read(name):
        path = REPORTS / name
        dependencies[str(path.relative_to(ROOT))] = digest(path)
        return json.loads(path.read_text())
    inventory = read('choose-consume-cost-candidates.json')
    rows = [dict(r, coverage_level='unexercised_path', evidence=[]) for r in inventory['rows']]
    by_key = {(r['card'], r['path']): r for r in rows}
    assert len(by_key) == len(rows) == 453
    def attach(card, path, level, evidence):
        row = by_key[(card, path)]
        row['evidence'].append(evidence)
        rank = {'unexercised_path':0,'partial_observation':1,'scoped_reviewed':2}
        if rank[level] > rank[row['coverage_level']]:
            row['coverage_level'] = level
    # Family ledgers preserve exact paths, independent cost alternatives and scopes.
    for name in ['fixed-tap-cost-family-coverage.json', 'fixed-exile-cost-coverage.json',
                 'single-return-cost-path-coverage.json', 'single-move-cost-family-coverage.json',
                 'single-exile-cost-coverage.json', 'unattach-cost-path-coverage.json',
                 'alternative-tap-cost-path-coverage.json', 'single-tap-station-path-coverage.json', 'single-tap-effect-path-coverage.json', 'single-tap-mana-path-coverage.json', 'single-tap-source-path-coverage.json', 'single-tap-special-six-path-coverage.json', 'root-single-tap-cost-path-coverage.json']:
        data = read(name)
        for i, entry in enumerate(data['rows']):
            status = entry['status']
            if status == 'unexercised_path':
                continue
            level = 'partial_observation' if 'not_measured' in status or 'announcement_only' in status else 'scoped_reviewed'
            attach(entry['card'], entry['path'], level,
                   dict(ledger='reports/runtime-audit/'+name,ledger_sha256=dependencies['reports/runtime-audit/'+name],
                        ledger_row=i, reviewed_status=status,
                        scope=entry.get('scope',data.get('scope')), details=entry))
    # These family reports were explicitly reviewed for the only matching
    # choose/consume path on each listed payload, including earlier legality gates.
    specs = [
        ('multi-return-cost-reviewed-classification.json',['rows'],
         {'fixed_multiple / ReturnToHandEffect'},None,6),
        ('nonmana-x-control-reviewed-classification.json',['rows'],
         {'dynamic_x / SacrificePlayerEffect','dynamic_x / ExileEffect'},None,6),
        ('nonmana-x-sibling-reviewed-attribution.json',['findings','controls'],
         {'dynamic_x / SacrificePlayerEffect','dynamic_x / ExileEffect'},None,8),
        ('graveyard-selection-cost-reviewed-attribution.json',['findings','controls'],
         {'other_count / ExileEffect','fixed_multiple / MoveToZoneEffect'},None,3),
        ('self-sacrifice-effect-cost-reproductions.json',['rows'],
         {'other_count / SacrificePlayerEffect'},{'Emrakul\'s Evangel','Sword of the Ages'},2),
        ('exile-cost-family-reviewed-attribution.json',['findings'],
         {'other_count / ExileEffect'},None,6),
    ]
    accepted = {'sampled_semantic_pass','expected_outcome_passed','expected_result_control',
                'runtime_defect_card_reproduced','compiler_semantic_defect_card_reproduced'}
    for name, collections_to_read, groups, only_names, expected_paths in specs:
        data = read(name); linked = set()
        if isinstance(data.get('source_report'),str) and data.get('source_sha256'):
            raw = REPORTS/data['source_report']; assert digest(raw)==data['source_sha256']
            dependencies[str(raw.relative_to(ROOT))]=digest(raw)
        for collection in collections_to_read:
            for i, finding in enumerate(data[collection]):
                card = finding.get('payload_name',finding.get('card',finding.get('card_name')))
                if only_names is not None and card not in only_names:
                    continue
                classification = finding.get('classification',finding.get('reviewed_classification'))
                assert classification in accepted,(name,collection,i,classification)
                matches = [r for r in rows if r['card']==card and r['count_shape']+' / '+r['consumer_kind'] in groups]
                assert len(matches)==1,(name,card,len(matches))
                candidate = matches[0]; linked.add((card,candidate['path']))
                source = finding.get('source_report')
                if isinstance(source,dict):
                    raw=ROOT/source['path']; assert digest(raw)==source['sha256']
                    dependencies[str(raw.relative_to(ROOT))]=digest(raw)
                attach(card,candidate['path'],'scoped_reviewed',
                       dict(reviewed_report='reports/runtime-audit/'+name,
                            reviewed_sha256=dependencies['reports/runtime-audit/'+name],
                            collection=collection,reviewed_row=i,classification=classification,
                            scenario=finding.get('scenario'),failure_stage=finding.get('failure_stage'),
                            scope=finding.get('fixture_review',finding.get('finding',finding.get('reason',finding.get('consumer_execution_status',data.get('scope'))))),
                            binding='Explicit family whitelist; exactly one matching typed cost path for this payload. Earlier legal-action failures do not execute the later cost or effect.'))
        assert len(linked)==expected_paths,(name,len(linked),expected_paths)
    groups=collections.defaultdict(collections.Counter)
    for row in rows:
        groups[row['count_shape']+' / '+row['consumer_kind']][row['coverage_level']]+=1
    output=dict(scope=__doc__.strip(),candidate_run_id=inventory['run_id'],summary=dict(paths=len(rows),payload_names=len({r['card'] for r in rows}),
          coverage_levels=dict(collections.Counter(r['coverage_level'] for r in rows)),groups={g:dict(v) for g,v in sorted(groups.items())}),
          rows=rows,dependencies=dependencies,generator_sha256=digest(Path(__file__)),limitations=[
              'This is a path-coverage index, not a new source of confirmed card defects or whole-card certification.',
              'Each alternative branch remains separate. A passing card observation does not cover unrelated paths.',
              'Reviewed failures may occur before selection/payment or in a later independent effect; consult the source attribution.',
              'Scenario rows may recur across indexes; aggregate counts here are paths only, never summed test or card findings.',
              'Only adjacent positive-tag cost pairs are inventoried. Other costs and nonadjacent dependencies remain outside this ledger.'])
    (REPORTS/'choose-consume-cost-family-coverage.json').write_text(json.dumps(output,indent=2)+'\n')
    lines=['# Choose/consume activation-cost coverage','','This index joins explicit evidence to each typed cost path. It makes no new defect promotions.','','| Cost shape / consumer | Scoped reviewed | Partial | Unexercised |','| --- | ---: | ---: | ---: |']
    for group,c in sorted(groups.items()):lines.append(f"| {group} | {c['scoped_reviewed']} | {c['partial_observation']} | {c['unexercised_path']} |")
    lines+=['','Exact paths, source row numbers, scopes and hashes are retained in `choose-consume-cost-family-coverage.json`. Reviewed coverage can be an early legality failure or a bounded passing control; it does not imply all costs/effects were reached.']
    (REPORTS/'choose-consume-cost-family-coverage.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps(output['summary'],indent=2))

if __name__=='__main__':main()
