import collections as C,gzip,json,hashlib
from pathlib import Path
P=Path(__file__).parent;ROOT=P.resolve().parents[2];OLD=ROOT/'reports/card-failure-campaign/refresh-20261007-main5cc46c1'
load=lambda p:json.load(open(p))
s=load(P/'summary.json');u=load(P/'current-unresolved-entries.json');by={r['card_name']:r for r in u};cmp=load(P/'oct7-identity-comparison.json')
old=json.load(gzip.open(OLD/'current-failures.json.gz','rt'));ob={r['card_name']:r for r in old}
reg=[by[n] for r in cmp['regressed'] for n in r['current_unresolved_entries']]
loss=[r for r in u if r['parse_status']=='strict_compiled' and r['parse_lossy']]
newloss=[r for r in loss if r['card_name'] not in ob or ob[r['card_name']]['parse_status']!='strict_compiled']
d={'regression_entry_categories':dict(C.Counter(r['category'] for r in reg)),'parse_failure_regressions':[r for r in reg if r['parse_status']=='parse_failed'],'lossy_gate_regressions':[r for r in reg if r['parse_lossy']],'strict_lossy_current_count':len(loss),'strict_lossy_old_count':4,'new_strict_lossy_previous_status':dict(C.Counter(ob[r['card_name']]['parse_status'] if r['card_name'] in ob else 'supported_strict' for r in newloss)),'strict_lossy_suffix_recovery_count':sum('suffix_object_filter_recovery' in r['parse_loss_reasons'] for r in loss),'warning':'Regression means failure of the strict non-lossy acceptance gate relative to prior audit, not demonstrated new gameplay defect. New loss detection may expose pre-existing parsing problems.'}
(P/'regression-and-lossy-detail.json').write_text(json.dumps(d,indent=2,sort_keys=True)+'\n')
cl=load(P/'overlapping-diagnostic-causes.json')
text=f'''# Oct8 clean-main refresh: exact-ID analysis

Source: {s['source']['commit']}; tree {s['source']['tree']}.
Snapshot SHA-256: {s['snapshot_sha256']}.
Filtered data SHA-256: {s['dataset_sha256']}.

## Measured current results

32,209 compile entries / 32,138 unique Oracle IDs. 1,824 unique IDs fail compilation; 1,925 remain unresolved under the strict, non-lossy, no-unimplemented gate (1,926 entries). Zero unknown identity outcomes.

Entry statuses: 30,377 strict-compiled, 1,825 parse-failed, 7 permissive fallback. Of strict-compiled entries, 94 are lossy and excluded from supported results. Categories distinguish 1,752 parser failures, 69 semantic-output marker rejections, 3 unsupported-mechanic rejections and 1 compiler panic within the parse-failed total.

Against Oct7, 175 unique IDs recovered and 95 failed the gate after previously passing; 1,830 remain unresolved, for a net reduction of 80. The 95 gate regressions are 88 strict-lossy and 7 parser-failed entries. Lossy counts rose from 4 to 94: all original 4 remain; 88 previously supported entries are now lossy and 2 previously failed entries now compile lossily. This does not establish 88 new gameplay bugs: improved loss detection may expose existing parsing problems.

Against frozen original baseline, 1,445 IDs recovered; 1,788 remain unresolved; 137 formerly supported IDs fail the gate; 28,768 remain supported. Oct7 figures (1,994 compile-failing, 2,005 unresolved, 1,379 original recoveries, 151 original regressions) are historical only.

## Input equivalence and coverage

No new or removed IDs, no renamed/alias-scope changes, no Oracle-text or loader-semantic changes. All 32,138 identities are in the unchanged-semantic-input partition. Complete semantic projection SHA-256: 8bcc5e77799e10c86a66efc1719c11645633985f985b5d16cfd5c3c26bb8b517, identical to original and retained Oct7 evidence. Full printing records changed in 32,093 cases outside this projection.

71 reversible alias entries collapse by their unambiguous face Oracle ID only for unique-ID counts. Source contains 33,127 face payloads; 918 supplemental linked/back/adventure/prepared face payloads were not independently executed. Exact alias membership and supplemental face names are retained.

## Suspected rendered-text signals

182 flagged entries (180 strict-compiled, 2 permissive); 49 newly flagged, 1,210 no longer flagged, 133 still flagged relative to retained Oct7 signal inventory. These are rendered-text heuristics, not confirmed gameplay miscompilations or verified gameplay recoveries. Compiler acceptance likewise does not prove gameplay correctness. Runtime scenarios were not executed.

## Diagnostic triage

899 full per-entry diagnostic signatures partition unresolved entries. 911 overlapping diagnostic-cause groups are a separate view; memberships must not be summed as unique cards or treated as independent proven defects.

'''
for g in cl[:12]:text+=f"- {g['unique_ids']} unique IDs / {g['entry_count']} entries: {g['cause']}\n"
text+='\n## Seven new parse-failure gate regressions\n\n'
for r in d['parse_failure_regressions']:text+=f"- {r['card_name']} ({r['oracle_id']}): {r['parse_error']}\n"
text+='''
## Evidence and limitations

analysis/ contains exact original and Oct7 identity transitions, every current unresolved entry with raw diagnostics, suspected rendered-text signals, complete overlapping diagnostic causes, input partitions and face coverage. Original/Oct7 evidence archives were SHA-256 verified against their retained manifest before analysis. The Oct7 full snapshot is not retained: prior exact support is reconstructed from the complete unresolved-entry inventory and cross-checked against exact-ID outcomes; no full Oct7 compiled-definition or similarity-score comparison is claimed.

This is offline analysis only. No source changes, compiler re-execution, extra corpus pass, runtime tests or remote writes were performed. Build/run provenance and untouched raw evidence remain in the parent refresh directory.
'''
(P/'README.md').write_text(text)
manifest={p.name:{'bytes':p.stat().st_size,'sha256':hashlib.file_digest(open(p,'rb'),'sha256').hexdigest()} for p in sorted(P.iterdir()) if p.is_file() and p.name!='manifest.json'}
(P/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
print(json.dumps({k:v for k,v in d.items() if k not in ('parse_failure_regressions','lossy_gate_regressions')},indent=2))
print('Seven new parse failures:',[r['card_name'] for r in d['parse_failure_regressions']])
