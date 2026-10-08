#!/usr/bin/env python3
"""Offline provenance and identity accounting; does not execute compiler/tests."""
import collections, gzip, hashlib, json, sys
from pathlib import Path
BASE=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else Path(__file__).resolve().parent
REPO=BASE.parent.parent
sys.dont_write_bytecode=True
sys.path.insert(0,str(REPO/'scripts'))
import card_failure_campaign as campaign

def save(name,data):
    (BASE/name).write_text(json.dumps(data,indent=2,sort_keys=True,ensure_ascii=False)+'\n')

def identity(c):
    if c.get('oracle_id'): return c['oracle_id']
    ids={f['oracle_id'] for f in c.get('card_faces',[]) if f.get('oracle_id')}
    if len(ids)==1: return next(iter(ids))
    raise ValueError('Ambiguous or absent Oracle identity: '+c['name'])

def name(c): return campaign.normalize_name(c['name'])

def oracle(c):
    return {'oracle_text':c.get('oracle_text'),'face_oracle_text':[f.get('oracle_text') for f in c.get('card_faces',[])]}

SEMANTIC_FIELDS=('name','oracle_id','oracle_text','mana_cost','type_line','power','toughness','loyalty','defense','color_indicator','colors','attraction_lights','layout','first_printed_set_name')
def semantic(c):
    return {**{k:c.get(k) for k in SEMANTIC_FIELDS},'card_faces':[{k:f.get(k) for k in SEMANTIC_FIELDS} for f in c.get('card_faces',[])]}

def load(path): return json.loads(path.read_text())
def group(cards):
    out=collections.defaultdict(list)
    for c in cards: out[identity(c)].append(c)
    return out

old=load(BASE/'data/cards-frozen-20261003.json')
new=load(BASE/'data/cards-current.json')
old_names={name(c):c for c in old}; new_names={name(c):c for c in new}
old_ids=group(old); new_ids=group(new)
matched=set(old_ids)&set(new_ids)
changed=[]; renamed=[]
for oid in sorted(matched):
    a=old_ids[oid]; b=new_ids[oid]
    an={name(c) for c in a}; bn={name(c) for c in b}
    if an!=bn: renamed.append({'oracle_id':oid,'original_names':sorted(an),'current_names':sorted(bn)})
    for n in sorted(an&bn):
        ac=old_names[n]; bc=new_names[n]
        if oracle(ac)!=oracle(bc) or semantic(ac)!=semantic(bc):
            changed.append({'oracle_id':oid,'card_name':n,'oracle_text_changed':oracle(ac)!=oracle(bc),'semantic_metadata_changed':semantic(ac)!=semantic(bc),'original':semantic(ac),'current':semantic(bc)})
source_changes=sum(old_names[n]!=new_names[n] for n in old_names.keys()&new_names.keys())
summary={
 'original_dataset_sha256':campaign.sha256_file(BASE/'data/cards-frozen-20261003.json'),
 'current_dataset_sha256':campaign.sha256_file(BASE/'data/cards-current.json'),
 'original_source_entries':len(old),'current_source_entries':len(new),
 'original_unique_oracle_identities_including_reversible_aliases':len(old_ids),
 'current_unique_oracle_identities_including_reversible_aliases':len(new_ids),
 'current_nonnull_top_level_oracle_ids':len({c['oracle_id'] for c in new if c.get('oracle_id')}),
 'current_entries_without_top_level_oracle_id':sum(not c.get('oracle_id') for c in new),
 'current_alias_excess':len(new)-len(new_ids),
 'current_source_face_entries':sum(len(c.get('card_faces',[])) or 1 for c in new),
 'current_multifaced_source_entries':sum(bool(c.get('card_faces')) for c in new),
 'current_multifaced_layout_counts':dict(collections.Counter(c.get('layout') for c in new if c.get('card_faces'))),
 'current_source_layout_counts':dict(collections.Counter(c.get('layout') for c in new)),
 'matched_oracle_identities':len(matched),'genuinely_new_oracle_identities':len(set(new_ids)-set(old_ids)),
 'missing_original_oracle_identities':len(set(old_ids)-set(new_ids)),
 'renamed_or_alias_changed_oracle_identities':len(renamed),
 'oracle_text_changed_compile_entries':sum(c['oracle_text_changed'] for c in changed),
 'semantic_metadata_changed_compile_entries':sum(c['semantic_metadata_changed'] for c in changed),
 'source_records_changed_in_any_field':source_changes,
 'original_semantic_projection_sha256':campaign.json_digest([semantic(c) for c in sorted(old,key=name)]),
 'current_semantic_projection_sha256':campaign.json_digest([semantic(c) for c in sorted(new,key=name)]),
 'added_compile_entry_names':sorted(new_names.keys()-old_names.keys()),
 'missing_compile_entry_names':sorted(old_names.keys()-new_names.keys()),
 'new_cards':[{'oracle_id':i,'names':sorted(name(c) for c in new_ids[i])} for i in sorted(new_ids.keys()-old_ids.keys())],
 'missing_cards':[{'oracle_id':i,'names':sorted(name(c) for c in old_ids[i])} for i in sorted(old_ids.keys()-new_ids.keys())],
 'renamed_or_alias_changed':renamed,
 'identity_rule':'Top-level Oracle ID, otherwise unique face Oracle ID; reversible aliases are retained as compile entries and collapse only for unique-card counts.',
 'comparison_scope':'All selected source entries. Oracle text compared top-level and every face; semantic metadata projection includes fields consumed by loader and all faces. Other Scryfall record changes can include printing, prices, imagery and ranks.',
 'linked_face_note':'Canonical status rows compile the selected front payload; other linked/back/adventure/prepared faces are not independently measured by this refresh.'
}
save('dataset-comparison.json',summary);save('changed-oracle-and-metadata.json',changed)
if not (BASE/'audit/snapshot.json').exists():
    print(json.dumps(summary,indent=2));sys.exit(0)
base=json.load(gzip.open(REPO/'fixtures/card-failure-campaign/baseline-e8740178.snapshot.json.gz','rt'))
current=load(BASE/'audit/snapshot.json')
assert base['source']['commit']=='e8740178a7f7367ffa3147e7642607042079237c'
assert base['dataset_sha256']=='9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c'
assert campaign.sha256_file(REPO/'fixtures/card-failure-campaign/baseline-e8740178.snapshot.json.gz')=='c34c06c92ca2a1e1b92d8cf8b1c9480ef2f2445aa7efcba28b8099d5f754cd12'
assert current['source']['commit']=='5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1'
assert current['dataset_sha256']=='de4cedd51e320d2983d80b38713213c0273fc7272d5e2a89908ec050061aeca5'
assert load(BASE/'audit/run.json')['completed'] is True
base_rows=campaign.validate_snapshot(base); current_rows=campaign.validate_snapshot(current)
assert set(old_names)==set(base_rows)
assert set(new_names)==set(current_rows)

def status_by_identity(cards, rows):
    grouped=collections.defaultdict(list)
    for c in cards: grouped[identity(c)].append(rows[name(c)])
    return {oid:{'supported':all(campaign.is_supported(r) for r in rs),'has_parse_failure':any(r['parse_status']=='parse_failed' for r in rs),'names':[r['card_name'] for r in rs],'failing_entries':[r['card_name'] for r in rs if not campaign.is_supported(r)],'rows':rs} for oid,rs in grouped.items()}
before=status_by_identity(old,base_rows); after=status_by_identity(new,current_rows)
buckets=collections.defaultdict(list)
changed_oids={c['oracle_id'] for c in changed if c['oracle_text_changed']}
for oid in sorted(matched):
    a=before[oid];b=after[oid]
    k=('still_supported' if b['supported'] else 'regressed') if a['supported'] else ('recovered' if b['supported'] else 'still_failing')
    buckets[k].append({'oracle_id':oid,'original_names':sorted(a['names']),'current_names':sorted(b['names']),'oracle_text_changed':oid in changed_oids,'current_failing_entries':b['failing_entries']})
assert len(before)==len(after)==32138
assert len(buckets['recovered'])+len(buckets['still_failing'])==3233
assert len(buckets['regressed'])+len(buckets['still_supported'])==28905
assert len(buckets['still_failing'])+len(buckets['regressed'])==sum(not a['supported'] for a in after.values())
metrics={
    'current_entry_summary':current['summary'],
    'current_unique_cards':len(after),
    'current_unique_compile_failures':sum(a['has_parse_failure'] for a in after.values()),
    'current_unique_unresolved_including_lossy_and_permissive':sum(not a['supported'] for a in after.values()),
    'original_unique_unresolved':sum(not a['supported'] for a in before.values()),
    'matched_original_identity_comparison':{k:len(v) for k,v in sorted(buckets.items())},
    'matched_original_identity_comparison_oracle_unchanged':{k:sum(not r['oracle_text_changed'] for r in v) for k,v in sorted(buckets.items())},
    'current_strict_compiled_lossy_entries':sum(r['parse_status']=='strict_compiled' and r['parse_lossy'] for r in current_rows.values()),
    'semantic_mismatch_by_compile_status':dict(collections.Counter(r['parse_status'] for r in current_rows.values() if r['semantic_mismatch'])),
    'old_historical_counts':{'baseline_unique_unresolved':3233,'previously_measured_recoveries':40,'previously_measured_remainder':3193,'source_claimed_coverage_is_not_measured':1256},
    'compile_acceptance_is_not_gameplay_correctness':True,
    'linked_faces_independently_measured':False,
}
save('current-metrics.json',metrics)
save('original-identity-comparison.json',dict(buckets))
failures=[]
for n,r in sorted(current_rows.items()):
    if not campaign.is_supported(r):
        failures.append({'oracle_id':identity(new_names[n]),'card_name':n,'layout':new_names[n].get('layout'),'is_reversible_alias':not bool(new_names[n].get('oracle_id')),'face_names':[f.get('name') for f in new_names[n].get('card_faces',[])],**{k:r[k] for k in ('parse_status','category','parse_error','parse_lossy','parse_loss_count','parse_loss_reasons','semantic_mismatch','oracle_text','raw_oracle_text','compiled_text','route_diagnostics')},'diagnostic_signature':campaign.failure_signature(r)})
save('current-failures.json',failures)
save('current-semantic-mismatches.json',[{'oracle_id':identity(new_names[n]),'card_name':n,**{k:r[k] for k in ('parse_status','category','supported','parse_lossy','similarity_score','oracle_coverage','compiled_coverage','line_delta','oracle_text','raw_oracle_text','compiled_text')}} for n,r in sorted(current_rows.items()) if r['semantic_mismatch']])
save('current-strict-lossy.json',[r for r in failures if r['parse_status']=='strict_compiled' and r['parse_lossy']])
entry_regressions=[]
changed_definitions=[]
for n in sorted(base_rows.keys() & current_rows.keys()):
    a,b=base_rows[n],current_rows[n]
    reasons=[]
    if campaign.is_supported(a) and not campaign.is_supported(b): reasons.append('previously_supported_entry_now_unresolved')
    if campaign.is_supported(a) and campaign.is_supported(b):
        if b['semantic_mismatch'] and not a['semantic_mismatch']: reasons.append('new_semantic_mismatch_heuristic')
        if b['similarity_score'] + 1e-6 < a['similarity_score']: reasons.append('similarity_score_decrease')
    if reasons: entry_regressions.append({'oracle_id':identity(new_names[n]),'card_name':n,'reasons':reasons,'original_similarity_score':a['similarity_score'],'current_similarity_score':b['similarity_score'],'original_semantic_mismatch':bool(a['semantic_mismatch']),'current_semantic_mismatch':bool(b['semantic_mismatch'])})
    if a['compiled_definition_sha256'] != b['compiled_definition_sha256']: changed_definitions.append(n)
save('entry-regression-signals.json',entry_regressions)
save('changed-compiled-definitions.json',changed_definitions)
metrics['entry_regression_signal_counts']=dict(collections.Counter(reason for r in entry_regressions for reason in r['reasons']))
metrics['changed_compiled_definition_entries']=len(changed_definitions)
metrics['unique_oracle_identity_regression_signals']=len({r['oracle_id'] for r in entry_regressions})
save('current-metrics.json',metrics)
coverage=load(REPO/'fixtures/card-failure-campaign/source-coverage.json')
historical_verified_ids={oid for r in coverage['cards'] if r['verified_compile_recovery'] for oid in r['oracle_ids']}
assert len(historical_verified_ids)==40
historical_verified_current=[{'oracle_id':oid,'names':after.get(oid,{}).get('names',[]),'supported':after.get(oid,{}).get('supported'),'failing_entries':after.get(oid,{}).get('failing_entries',[])} for oid in sorted(historical_verified_ids)]
save('historical-40-current-status.json',historical_verified_current)
proposed_ids={oid for r in coverage['cards'] if r['source_coverage_status']=='implemented_pending_full_corpus_validation' for oid in r['oracle_ids']}
assert len(proposed_ids)==1256
save('historical-source-1256-current-status.json',[{'oracle_id':oid,'names':after.get(oid,{}).get('names',[]),'supported':after.get(oid,{}).get('supported'),'failing_entries':after.get(oid,{}).get('failing_entries',[])} for oid in sorted(proposed_ids)])
metrics['historical_source_proposed_1256_current_status']={'supported':sum(after[i]['supported'] for i in proposed_ids if i in after),'unresolved':sum(not after[i]['supported'] for i in proposed_ids if i in after),'missing':len(proposed_ids-after.keys())}

metrics['historical_40_current_status']={'still_supported':sum(r['supported'] is True for r in historical_verified_current),'currently_unresolved':sum(r['supported'] is False for r in historical_verified_current),'missing':sum(r['supported'] is None for r in historical_verified_current)}
save('current-metrics.json',metrics)
family_results=[]
for family in coverage.get('families',[]):
    names=family.get('baseline_entries',[])
    if not names: continue
    present=[n for n in names if n in current_rows]
    unresolved=[n for n in present if not campaign.is_supported(current_rows[n])]
    family_results.append({'family_id':family['id'],'listed_baseline_compile_entries':len(names),'historical_family_status':family.get('status'),'present_entries':len(present),'supported_entries':len(present)-len(unresolved),'unresolved_entries':len(unresolved),'unresolved_cards':unresolved,'missing_entries':sorted(set(names)-set(present))})
save('source-family-current-results.json',family_results)
save('current-diagnostic-clusters.json',current['summary']['diagnostic_groups'])
roots={}
for r in failures:
    evidence=[x for x in r['route_diagnostics'] if x.get('diagnostic')]
    if not evidence: evidence=[{'route':'authoritative_result','diagnostic':r['parse_error'] or r['parse_loss_reasons'] or r['category']}]
    for e in evidence:
        root=campaign.normalize_diagnostic(e['diagnostic'])
        key=r['category']+': '+root
        g=roots.setdefault(key,{'diagnostic_root':key,'category':r['category'],'cards':set(),'oracle_ids':set(),'diagnostic_record_count':0,'route_counts':collections.Counter(),'example_errors':[]})
        g['cards'].add(r['card_name']);g['oracle_ids'].add(r['oracle_id']);g['diagnostic_record_count']+=1;g['route_counts'][e['route']]+=1
        if len(g['example_errors'])<3:g['example_errors'].append({'card_name':r['card_name'],'route':e['route'],'diagnostic':e['diagnostic']})
root_groups=[]
for g in roots.values():
    g['compile_entry_count']=len(g['cards']);g['unique_oracle_card_count']=len(g['oracle_ids']);g['cards']=sorted(g['cards']);g['oracle_ids']=sorted(g['oracle_ids']);g['route_counts']=dict(g['route_counts']);root_groups.append(g)
root_groups.sort(key=lambda g:(-g['unique_oracle_card_count'],g['diagnostic_root']))
save('current-root-cause-clusters.json',{'method':'Group each primary/fallback or authoritative-result diagnostic by normalized reported cause, preserving raw examples. Card membership is deduplicated by Oracle ID within each group. One card can appear in multiple diagnostic causes; groups are triage evidence, not proven implementation defects or parser-route identities.','group_count':len(root_groups),'groups':root_groups})
print(json.dumps({k:v for k,v in metrics.items() if k!='current_entry_summary'},indent=2))
