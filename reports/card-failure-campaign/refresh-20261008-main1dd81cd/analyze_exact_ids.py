#!/usr/bin/env python3
"""Offline-only identity/input/outcome analysis. Never executes compiler or tests."""
import collections as C, gc, gzip, hashlib, json, lzma, sys
from pathlib import Path
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[3]
BASE=ROOT/'reports/current-refresh-20261008'; OUT=BASE/'analysis'
OLD=ROOT/'reports/card-failure-campaign/refresh-20261007-main5cc46c1'
sys.path.insert(0,str(ROOT/'scripts'))
import card_failure_campaign as campaign
FIELDS=('name','oracle_id','oracle_text','mana_cost','type_line','power','toughness','loyalty','defense','color_indicator','colors','attraction_lights','layout','first_printed_set_name')
def load(p):
    opener=gzip.open if str(p).endswith('.gz') else lzma.open if str(p).endswith('.xz') else open
    with opener(p,'rt') as f:return json.load(f)
def save(n,d):
    with open(OUT/n,'w') as f:json.dump(d,f,indent=2,sort_keys=True,ensure_ascii=False);f.write('\n')
def sha(p):return campaign.sha256_file(p)
def identity(c):
    if c.get('oracle_id'):return c['oracle_id']
    ids={f['oracle_id'] for f in c.get('card_faces',[]) if f.get('oracle_id')}
    assert len(ids)==1,(c['name'],ids)
    return next(iter(ids))
def project(c):return {**{k:c.get(k) for k in FIELDS},'card_faces':[{k:f.get(k) for k in FIELDS} for f in c.get('card_faces',[])]}
def read_inputs(p):
    cards=load(p);out={};faces=[]
    for c in cards:
        n=campaign.normalize_name(c['name']);assert n not in out
        s=project(c);out[n]={'oracle_id':identity(c),'semantic':s,'oracle':{'oracle_text':c.get('oracle_text'),'face_oracle_text':[f.get('oracle_text') for f in c.get('card_faces',[])]},'full_record_sha256':campaign.json_digest(c),'alias':not bool(c.get('oracle_id')),'layout':c.get('layout')}
        for i,f in enumerate(c.get('card_faces',[])):
            if i:faces.append({'card_name':n,'oracle_id':identity(c),'layout':c.get('layout'),'face_index':i,'face_name':f.get('name'),'face_oracle_id':f.get('oracle_id'),'status':'not independently measured'})
    del cards;gc.collect();return out,faces

def group(inputs):
    g=C.defaultdict(list)
    for n,c in inputs.items():g[c['oracle_id']].append(n)
    return {i:sorted(ns) for i,ns in g.items()}

def prepare():
    frozen=ROOT/'fixtures/card-failure-campaign/cards-20261003.json.xz'
    old,_=read_inputs(frozen);new,faces=read_inputs(BASE/'data/cards-current.json')
    og,ng=group(old),group(new);changes=[];partitions=C.defaultdict(list)
    for oid in sorted(set(og)|set(ng)):
        an,bn=og.get(oid,[]),ng.get(oid,[])
        if not an:k='new_identity'
        elif not bn:k='removed_identity'
        elif an!=bn:k='entry_or_alias_scope_changed'
        elif any(old[n]['oracle']!=new[n]['oracle'] for n in an):k='oracle_changed'
        elif any(old[n]['semantic']!=new[n]['semantic'] for n in an):k='semantic_metadata_changed'
        else:k='unchanged_semantic_input'
        partitions[k].append(oid)
        if k!='unchanged_semantic_input':changes.append({'oracle_id':oid,'partition':k,'original':{n:old[n]['semantic'] for n in an},'current':{n:new[n]['semantic'] for n in bn}})
    oldhash=campaign.json_digest([old[n]['semantic'] for n in sorted(old)])
    newhash=campaign.json_digest([new[n]['semantic'] for n in sorted(new)])
    oct7=load(OLD/'summary.json')
    assert oldhash==oct7['dataset']['current_semantic_projection_sha256']=='8bcc5e77799e10c86a66efc1719c11645633985f985b5d16cfd5c3c26bb8b517'
    summary={'original_entries':len(old),'current_entries':len(new),'original_unique_ids':len(og),'current_unique_ids':len(ng),'partition_counts':{k:len(v) for k,v in partitions.items()},'original_semantic_projection_sha256':oldhash,'current_semantic_projection_sha256':newhash,'current_alias_entries':sum(c['alias'] for c in new.values()),'current_source_face_payloads':len(new)+len(faces),'supplemental_face_payloads_unmeasured':len(faces),'source_records_changed_any_field':sum(old[n]['full_record_sha256']!=new[n]['full_record_sha256'] for n in old.keys()&new.keys()),'original_archive_sha256':sha(frozen),'current_dataset_sha256':sha(BASE/'data/cards-current.json'),'oct7_input_equivalence_basis':'Retained Oct7 dataset summary records the same complete semantic projection hash as frozen input; full Oct7 snapshot is not retained. Oct7 exact support is reconstructed from complete unresolved-entry inventory and checked unique-ID inventory.'}
    save('dataset-summary.json',summary);save('input-partitions.json',dict(partitions));save('changed-input-evidence.json',changes);save('linked-face-coverage.json',faces)
    save('input-index.json',{n:{k:v for k,v in c.items() if k not in ('semantic','oracle','full_record_sha256')} for n,c in new.items()})
    save('original-input-index.json',{n:{'oracle_id':c['oracle_id']} for n,c in old.items()})
    print(json.dumps(summary,indent=2))

def analyze():
    p=BASE/'audit-authoritative/snapshot.json';run=load(BASE/'audit-authoritative/run.json');assert run['completed'] is True
    current=load(p);assert current['source']['commit']=='1dd81cd84c62f272479f26e16d74719fff24b97b'
    assert current['dataset_sha256']==sha(BASE/'data/cards-current.json')
    rows=campaign.validate_snapshot(current);inputs=load(OUT/'input-index.json');assert rows.keys()==inputs.keys()
    original=load(ROOT/'fixtures/card-failure-campaign/baseline-e8740178.snapshot.json.gz');orows=campaign.validate_snapshot(original)
    oi=load(OUT/'original-input-index.json');assert orows.keys()==oi.keys()
    manifest=load(OLD/'manifest.json')
    for fname,meta in manifest['files'].items():assert sha(OLD/fname)==meta['sha256'],fname
    assert sha(ROOT/'fixtures/card-failure-campaign/baseline-e8740178.snapshot.json.gz')=='c34c06c92ca2a1e1b92d8cf8b1c9480ef2f2445aa7efcba28b8099d5f754cd12'
    historic=load(OLD/'current-failures.json.gz');hf={r['card_name']:r for r in historic};assert len(hf)==2008
    hcomp=load(OLD/'baseline-and-proposal-comparison.json.gz')
    hu={r['oracle_id'] for k in ('still_failing','regressed') for r in hcomp['changed_or_unresolved_identity_outcomes'][k]};assert len(hu)==2005
    assert {r['oracle_id'] for r in historic}==hu
    assert len({r['oracle_id'] for r in historic if r['parse_status']=='parse_failed'})==1994
    ng,og=group(inputs),group(oi)
    def outcome(rs):
        if any(r is None or r.get('parse_status') not in ('strict_compiled','parse_failed','compiled_with_allow_unsupported') for r in rs):return 'unknown'
        return 'supported' if all(campaign.is_supported(r) for r in rs) else 'unresolved'
    after={i:outcome([rows[n] for n in ns]) for i,ns in ng.items()}
    before={i:outcome([orows[n] for n in ns]) for i,ns in og.items()}
    prev={i:('unresolved' if i in hu else 'supported') for i in og}
    partitions=load(OUT/'input-partitions.json');ip={i:k for k,ids in partitions.items() for i in ids}
    def compare(old,label):
        buckets=C.defaultdict(list)
        for i in sorted(set(old)|set(after)):
            a,b=old.get(i,'absent'),after.get(i,'absent')
            k='new_input' if a=='absent' else 'removed_input' if b=='absent' else 'unknown' if 'unknown' in (a,b) else 'recovered' if (a,b)==('unresolved','supported') else 'regressed' if (a,b)==('supported','unresolved') else 'still_supported' if b=='supported' else 'still_unresolved'
            buckets[k].append({'oracle_id':i,'names':ng.get(i,og.get(i)),'input_partition':ip[i],'current_unresolved_entries':[n for n in ng.get(i,[]) if not campaign.is_supported(rows[n])]})
        save(label+'-identity-comparison.json',dict(buckets))
        return {k:len(v) for k,v in buckets.items()}
    failures=[dict(rows[n],oracle_id=inputs[n]['oracle_id'],layout=inputs[n]['layout'],is_reversible_alias=inputs[n]['alias'],diagnostic_signature=campaign.failure_signature(rows[n])) for n in sorted(rows) if not campaign.is_supported(rows[n])]
    semantic=[dict(rows[n],oracle_id=inputs[n]['oracle_id']) for n in sorted(rows) if rows[n].get('semantic_mismatch')]
    save('current-unresolved-entries.json',failures);save('suspected-rendered-text-signals.json',semantic)
    prior_semantic={r['card_name']:r for r in load(OLD/'semantic-mismatch-signals.json.gz')}
    current_semantic={r['card_name']:r for r in semantic}
    save('oct7-rendered-signal-transitions.json',{'newly_flagged':[{'card_name':n,'oracle_id':inputs[n]['oracle_id'],'current_parse_status':rows[n]['parse_status']} for n in sorted(current_semantic.keys()-prior_semantic.keys())],'no_longer_flagged':[{'card_name':n,'oracle_id':inputs[n]['oracle_id'],'current_parse_status':rows[n]['parse_status']} for n in sorted(prior_semantic.keys()-current_semantic.keys())],'still_flagged':sorted(prior_semantic.keys()&current_semantic.keys()),'warning':'Flag transitions are text heuristics, not verified gameplay recovery/regression.'})
    entry_changes=[]
    for n in sorted(orows.keys()&rows.keys()):
        a,b=orows[n],rows[n];reasons=[]
        if campaign.is_supported(a) and not campaign.is_supported(b):reasons.append('supported_to_unresolved')
        if campaign.is_supported(a) and campaign.is_supported(b):
            if b['semantic_mismatch'] and not a['semantic_mismatch']:reasons.append('new_suspected_rendered_text_flag')
            if b['similarity_score']+1e-6<a['similarity_score']:reasons.append('similarity_decrease')
        if reasons:entry_changes.append({'card_name':n,'oracle_id':inputs[n]['oracle_id'],'reasons':reasons,'original_status':a['parse_status'],'current_status':b['parse_status'],'original_similarity':a['similarity_score'],'current_similarity':b['similarity_score']})
    save('original-entry-signal-transitions.json',entry_changes)
    clusters={}
    for r in failures:
        ds=[d for d in r['route_diagnostics'] if d.get('diagnostic')] or [{'route':'authoritative_result','diagnostic':r['parse_error'] or r['parse_loss_reasons'] or r['category']}]
        for d in ds:
            key=r['category']+': '+campaign.normalize_diagnostic(d['diagnostic'])
            g=clusters.setdefault(key,{'cause':key,'oracle_ids':set(),'entries':set(),'evidence':[]})
            g['oracle_ids'].add(r['oracle_id']);g['entries'].add(r['card_name']);g['evidence'].append({'card_name':r['card_name'],**d})
    for g in clusters.values():g['oracle_ids']=sorted(g['oracle_ids']);g['entries']=sorted(g['entries']);g['unique_ids']=len(g['oracle_ids']);g['entry_count']=len(g['entries'])
    save('overlapping-diagnostic-causes.json',sorted(clusters.values(),key=lambda g:(-g['unique_ids'],g['cause'])))
    report={'source':current['source'],'dataset_sha256':current['dataset_sha256'],'snapshot_sha256':sha(p),'entry_status_counts':dict(C.Counter(r['parse_status'] for r in rows.values())),'entry_category_counts':dict(C.Counter(r['category'] for r in rows.values())),'unique_ids':len(ng),'unique_compile_failures':len({inputs[n]['oracle_id'] for n,r in rows.items() if r['parse_status']=='parse_failed'}),'unique_unresolved':sum(s=='unresolved' for s in after.values()),'unknown_identity_outcomes':sum(s=='unknown' for s in after.values()),'unresolved_entries':len(failures),'strict_lossy_entries':sum(r['parse_status']=='strict_compiled' and bool(r['parse_lossy']) for r in rows.values()),'strict_unimplemented_entries':sum(r['parse_status']=='strict_compiled' and bool(r['has_unimplemented']) for r in rows.values()),'parse_loss_flags_all_statuses':sum(bool(r['parse_lossy']) for r in rows.values()),'suspected_rendered_text_entries':len(semantic),'suspected_rendered_text_status_counts':dict(C.Counter(r['parse_status'] for r in semantic)),'versus_original':compare(before,'original'),'versus_oct7':compare(prev,'oct7'),'diagnostic_signature_count':len({r['diagnostic_signature'] for r in failures}),'overlapping_diagnostic_cause_count':len(clusters),'limits':['Compile acceptance does not prove gameplay correctness.','Rendered-text similarity and semantic flags are suspected signals, not confirmed gameplay miscompilations.','Linked/back/adventure/prepared supplemental faces were not independently executed.','Oct7 comparison reconstructs exact support from retained complete unresolved inventory; no full Oct7 compiled-definition or similarity-score comparison is claimed.']}
    save('summary.json',report)
    paths=[p,BASE/'audit-authoritative/run.json',ROOT/'fixtures/card-failure-campaign/baseline-e8740178.snapshot.json.gz',OLD/'manifest.json',OLD/'summary.json',OLD/'current-failures.json.gz',OLD/'baseline-and-proposal-comparison.json.gz',BASE/'data/download-provenance.json']+list(OUT.glob('*.py'))
    save('provenance.json',{'inputs':{str(x.relative_to(ROOT)):{'bytes':x.stat().st_size,'sha256':sha(x)} for x in paths},'mode':'offline analysis only; no compiler/tests/source changes/remote writes'})
    print(json.dumps(report,indent=2))
if __name__=='__main__':
    {'prepare':prepare,'analyze':analyze}[sys.argv[1]]()
