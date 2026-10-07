#!/usr/bin/env python3
import json,sys,collections,gzip
from pathlib import Path
P=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else Path(__file__).resolve().parent; R=P.parent.parent
sys.dont_write_bytecode=True;sys.path.insert(0,str(R/'scripts'))
from card_failure_tag_clusters import TagIndex
load=lambda f:json.loads((P/f).read_text())
tags=TagIndex.load(R/'fixtures/card-failure-campaign')
cards={c['name']:c for c in load('data/cards-current.json')}
prior=load('prior-proposals-55-unresolved.json'); entries=[e for c in prior for e in c['current_failure_entries']]
status={k:{'compile_entries':sum(e['parse_status']==k for e in entries),'unique_oracle_ids':len({e['oracle_id'] for e in entries if e['parse_status']==k})} for k in sorted({e['parse_status'] for e in entries})}
category={k:{'compile_entries':sum(e['category']==k for e in entries),'unique_oracle_ids':len({e['oracle_id'] for e in entries if e['category']==k})} for k in sorted({e['category'] for e in entries})}
summary={'unique_rehold_identities':len(prior),'failing_compile_entries':len(entries),'authoritative_status_counts':status,'diagnostic_category_counts':category,'note':'Authoritative parse_failed includes semantic-output-marker rejections and generated unsupported mechanics. These are not extra failures and are distinct from accepted semantic_mismatch heuristic flags. The primary unresolved gate is unchanged: strict_compiled with no parse error, parse loss or unimplemented content. One Oracle identity can have multiple alias entry outcomes, so status/category unique counts are not additive.','tag_provenance':tags.provenance,'tag_scope':'Pinned October 3 selected functional categories; card-level overlapping triage hints, not root-cause or runtime correctness proof. Ancestor-only labels are derived.'}
for c in prior:
 for e in c['current_failure_entries']:
  info=tags.enrich_card(cards[e['card_name']]);e['selected_functional_categories_direct']=info['functional_categories_direct'];e['selected_functional_categories_ancestor_only']=info['functional_categories_ancestor_only'];e['scryfall_keywords']=info['scryfall_keywords']
(P/'prior-55-status-and-tags.json').write_text(json.dumps({'summary':summary,'cards':prior},indent=2,sort_keys=True,ensure_ascii=False)+'\n')
roots=load('current-root-cause-clusters.json');rootgroups=[]
for g in roots['groups']:
 labels=collections.defaultdict(set);direct=collections.defaultdict(set);anc=collections.defaultdict(set)
 for n in g['cards']:
  info=tags.enrich_card(cards[n]);oids=info['oracle_ids']
  for c in info['functional_categories_direct']:labels[c].update(oids);direct[c].update(oids)
  for c in info['functional_categories_ancestor_only']:labels[c].update(oids);anc[c].update(oids)
 cats=[{'category':c,'unique_oracle_cards':len(v),'direct_unique_oracle_cards':len(direct[c]),'ancestor_only_unique_oracle_cards':len(anc[c]-direct[c])} for c,v in labels.items()];cats.sort(key=lambda x:(-x['unique_oracle_cards'],x['category']))
 rootgroups.append({'diagnostic_root':g['diagnostic_root'],'unique_oracle_card_count':g['unique_oracle_card_count'],'compile_entry_count':g['compile_entry_count'],'top_selected_functional_categories':cats[:12],'example_errors':g['example_errors']})
(P/'tag-enriched-diagnostic-priorities.json').write_text(json.dumps({'tag_provenance':tags.provenance,'notes':summary['tag_scope'],'diagnostic_causes':rootgroups},indent=2,sort_keys=True,ensure_ascii=False)+'\n')
print(json.dumps(summary,indent=2))
