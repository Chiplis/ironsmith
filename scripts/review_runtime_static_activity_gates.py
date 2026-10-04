#!/usr/bin/env python3
"""Source-bound refinement of the conditional-static index candidate screen.

Only exact direct payload classes with a reviewed StaticAbilityKind implementation
are classified. This is not a gameplay pass or a frozen-binary equivalence claim.
"""
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sqlite3
BASE=Path(__file__).resolve().parents[1]
ROOT=BASE/'reports/runtime-audit'
TYPES={
 'EntersTappedUnlessCondition':('misc.rs','EntersTappedUnlessCondition'),
 'EnterWithCountersIfCondition':('misc.rs','EntersWithCountersIfCondition'),
 'AttachedAbilityGrant':('continuous.rs','AttachedAbilityGrant'),
 'CantAttackUnlessCondition':('combat.rs','CantAttackUnlessCondition'),
 'ConditionalDrawReplacement':('misc/replacements_and_rules.rs','ConditionalDrawReplacement'),
 'ThisSpellCostReduction':('cost_modifiers.rs','ThisSpellCostReduction'),
 'Affinity':('cost_modifiers.rs','ThisSpellCostReduction'),
 'Anthem':('continuous.rs','Anthem'),
 'GrantObjectAbilityForFilter':('continuous/grants.rs','GrantObjectAbilityForFilter'),
 'RuleRestriction':('restrictions.rs','RuleRestriction'),
 'RemoveCardTypes':('continuous.rs','RemoveCardTypesForFilter'),
 'AddCardTypes':('continuous.rs','AddCardTypesForFilter'),
 'SetCardTypes':('continuous.rs','SetCardTypesForFilter'),
 'AddSubtypes':('continuous.rs','AddSubtypesForFilter'),
 'SetBasePowerToughnessForFilter':('continuous.rs','SetBasePowerToughnessForFilter'),
}
WRAPPED_PAYLOADS = {'RemoveCardTypes':'RemoveCardTypes', 'AddCardTypes':'AddCardTypes',
 'SetCardTypes':'SetCardTypes', 'AddSubtypes':'AddSubtypes',
 'SetBasePowerToughnessForFilter':'SetBasePowerToughness'}
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 screen_path=ROOT/'ability-index-structural-candidates.json'
 screen=json.loads(screen_path.read_text())
 sources={}
 rules={}
 for kind,(relative,typ) in TYPES.items():
  path=BASE/'crates/ironsmith-engine/src/static_abilities'/relative
  data=path.read_text()
  m=re.search(r'^impl StaticAbilityKind for '+re.escape(typ)+r' \{\n.*?^\}',data,re.M|re.S)
  if not m:raise ValueError(f'No exact implementation: {typ}')
  sources[str(path.relative_to(BASE))]=sha(path)
  overrides=bool(re.search(r'^    fn is_active\(',m.group(),re.M))
  rules[kind]={'implementation':str(path.relative_to(BASE)), 'type':typ,
   'line':data[:m.start()].count('\n')+1,'overrides_static_is_active':overrides,
   'native_condition_support':bool(re.search(r'fn with_static_condition\(&self, condition: crate::ConditionExpr\) -> Option<StaticAbility> \{\s*Some\(StaticAbility::new\(self.clone\(\).with_condition\(condition\)\)\)\s*\}',m.group()))}
 trait=BASE/'crates/ironsmith-engine/src/static_abilities/mod.rs'
 trait_text=trait.read_text()
 assert re.search(r'fn is_active\(&self, _game: &GameState, _source: ObjectId\) -> bool \{\s*true\s*\}',trait_text)
 sources[str(trait.relative_to(BASE))]=sha(trait)
 interpreter=BASE/'crates/ironsmith-engine/src/static_abilities/model_interpreter.rs'
 sources[str(interpreter.relative_to(BASE))]=sha(interpreter)
 connection=sqlite3.connect((ROOT/'corpus/results.sqlite3').resolve().as_uri()+'?mode=ro',uri=True)
 connection.execute('PRAGMA query_only=ON')
 run_id='267a16aff3b321196397d0b4'
 frozen_rows={}
 rows=[]
 for row in screen['rows']:
  kind=row['static_id'];rule=rules.get(kind)
  direct_path=f"abilities[{row['static_index']}].kind.Static.payload."
  keys=[c['path'][len(direct_path):] for c in row['conditions']]
  nested=all('.ability.' in k or '.follow_up_effects' in k for k in keys)
  direct=all(len(k.split('.'))==2 for k in keys)
  classification='requires_payload_or_wrapper_review'
  wrapper_review=None
  if kind in {'ThisSpellCostReduction','Affinity'} and all(c['value']=='Always' for c in row['conditions']):
   classification='constant_always_activity_condition'
  elif nested:classification='nested_effect_or_granted_ability_condition_not_parent_gate'
  elif direct and rule and not any(k.startswith('Conditional.') for k in keys):
   classification=('direct_static_activity_gate_declared' if rule['overrides_static_is_active'] else 'condition_does_not_override_static_activity_gate')
  elif kind in WRAPPED_PAYLOADS and keys == ['Conditional.condition'] and rule['native_condition_support']:
   if row['card'] not in frozen_rows:
    record=connection.execute('SELECT result_json FROM result WHERE run_id=? AND card_name=?',(run_id,row['card'])).fetchone()
    if not record:raise ValueError(f"Missing frozen definition: {row['card']}")
    frozen_rows[row['card']]=(json.loads(record[0]),hashlib.sha256(record[0].encode()).hexdigest())
   frozen,record_sha=frozen_rows[row['card']]
   model=frozen['definition']['abilities'][row['static_index']]['kind']['Static']
   inner=model.get('payload',{}).get('Conditional',{}).get('ability',{})
   if (model.get('id')==kind and inner.get('id')==kind and
       list(inner.get('payload',{}))==[WRAPPED_PAYLOADS[kind]]):
    classification=('wrapped_native_static_activity_gate_declared' if rule['overrides_static_is_active'] else 'wrapped_native_condition_does_not_override_static_activity_gate')
    wrapper_review={'run_id':run_id,'result_json_sha256':record_sha,
     'inner_payload':WRAPPED_PAYLOADS[kind],
     'reason':'The interpreter delegates Conditional to the inner model with_condition. This exact concrete type returns a native conditional leaf, so no fallback GrantAbility wrapper is used; is_active follows the recorded concrete implementation.'}
  rows.append({'card':row['card'],'static_index':row['static_index'],'static_id':kind,
   'condition_paths':[c['path'] for c in row['conditions']], 'classification':classification,
   'source_review':rule,'wrapper_review':wrapper_review,'gameplay_tested_by_this_review':False,'whole_card_verified':False})
 connection.close()
 counts=Counter(r['classification'] for r in rows)
 out={'generated_at':datetime.now(timezone.utc).isoformat(),
  'scope':'Current-source review of whether the exact scanned condition belongs to a StaticAbilityKind activity gate. Original frozen candidates and separate runtime probes remain visible.',
  'counts':dict(counts),'rows':rows,'type_reviews':rules,
  'provenance':{'screen':screen_path.name,'screen_sha256':sha(screen_path),'generator_sha256':sha(Path(__file__)),'source_files':[{'path':p,'sha256':h} for p,h in sorted(sources.items())]},
  'limitations':['Absence of an is_active override means this implementation inherits the trait default true; its condition can still affect replacement or effect behavior.',
   'A declared activity gate does not prove different indices, reachability of an inactive state, or any runtime failure.',
   'Only exact frozen Conditional payload shapes whose concrete implementation natively accepts a condition are reviewed; other wrappers and payloads remain unresolved.',
   'This source snapshot is distinct from historical frozen executables. The report removes no raw observation and does not certify frozen runtime behavior.',
   'Nested effect conditions do not directly gate the parent static entry, but other parent or granted-ability behavior remains outside this review.']}
 (ROOT/'ability-index-static-gate-review.json').write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps(counts,indent=2))
if __name__=='__main__':main()
