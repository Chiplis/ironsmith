#!/usr/bin/env python3
"""Group frozen source/numeric-reference candidates by required producer; never promote bugs."""
import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

def trigger_names(trigger):
    if not isinstance(trigger, dict):
        return []
    kind = trigger.get('kind')
    if isinstance(kind, str):
        return [kind]
    if not isinstance(kind, dict):
        return []
    names = list(kind)
    for value in kind.values():
        if isinstance(value, dict):
            for branch in ('left', 'right', 'trigger'):
                if branch in value:
                    names.extend(trigger_names(value[branch]))
    return names

def group(row):
    if row['ability_kind'] != 'Triggered':
        return 'activated_ability' if row['ability_kind'] == 'Activated' else 'spell_or_outer_effect'
    nested = '.payload.' in row['ability_path']
    if 'self_etb' in row['etb_contexts']:
        return 'granted_or_created_self_etb' if nested else ('self_and_other_etb' if 'other_or_any_etb' in row['etb_contexts'] else 'self_etb')
    if 'other_or_any_etb' in row['etb_contexts']:
        return 'other_or_any_etb'
    kinds = trigger_names(row['trigger'])
    joined = ' '.join(kinds).lower()
    label = (row['trigger'] or {}).get('label', '').lower()
    if 'isdealtdamage' in joined:
        return 'damage_received'
    if 'damage' in joined or 'damage' in label:
        return 'damage_dealt_or_combat_damage'
    if 'dies' in joined or 'dies' in label:
        return 'death'
    if 'zonechange' in joined:
        zone = (row['trigger'] or {}).get('kind', {}).get('ZoneChange', {})
        if zone.get('from') == 'Battlefield':
            return 'death' if zone.get('to') == 'Graveyard' else 'leaves_battlefield'
        return 'other_zone_change'
    if 'attack' in joined or 'block' in joined:
        return 'attack_or_block'
    if 'tapped' in joined or 'untap' in joined or 'tapsformana' in joined:
        return 'tap_or_untap'
    if 'sagachapter' in joined:
        return 'saga_chapter'
    if 'turnedfaceup' in joined:
        return 'turned_face_up'
    if 'keywordaction' in joined:
        return 'keyword_action'
    if 'counter' in joined:
        return 'counter_changed'
    if 'sacrifice' in joined:
        return 'sacrifice'
    if 'monstrous' in joined or 'mutates' in joined:
        return 'monstrous_or_mutate'
    if 'targeted' in joined:
        return 'becomes_targeted'
    if any(k in joined for k in ('upkeep','endstep','beginning','phase','step')):
        return 'turn_step'
    if 'cast' in joined:
        return 'spell_cast'
    if 'discard' in joined or 'draw' in joined:
        return 'draw_or_discard'
    return 'other_trigger'

PRODUCERS = {
 'self_etb': 'Pay normal canonical cast, resolve exactly its spell through priority/SBAs, verify queued ETB and announced target, then paid Unsummon/Boomerang before trigger. Oracle-derived amounts; preserve entry/kicker/X prerequisites.',
 'other_or_any_etb': 'Paid source cast, then a separate canonical qualifying creature/land enters. Independently remove trigger owner versus entering/damage-source object. Apply subtype/controller/quantity requirements from Oracle.',
 'self_and_other_etb': 'Run own-entry control and another qualifying permanent entry separately; do not conflate trigger owner with event object.',
 'granted_or_created_self_etb': 'Create the granting source or token-producing ability through legal actions first; the nested ability is not directly on the original source.',
 'death': 'Paid source cast then paid Murder or actual lethal damage; retain last-known characteristics, optionally paid Cremate on destination object before trigger.',
 'leaves_battlefield': 'Actually return/destroy the qualifying permanent, preserve departing snapshot and destination object, then optionally move that destination object again before the trigger resolves.',
 'saga_chapter': 'Actual paid Saga cast for chapter I or actual precombat-main lore progression for later chapter; verify lore count and preserve queued chapter trigger during removal.',
 'turned_face_up': 'Actually cast or create face-down object through supported mechanic and pay real turn-face-up cost; verify resulting event before removing the face-up source.',
 'keyword_action': 'Use the actual printed keyword action, commonly paid cycling, with its real source zone and cost; inspect exact matcher/action before sharing a fixture.',
 'counter_changed': 'Use an actual printed ability/spell that adds or removes the qualifying counters, verifying count and event ordinal.',
 'sacrifice': 'Pay a real sacrifice cost or resolve the printed sacrifice instruction, preserving last-known information and actor attribution.',
 'monstrous_or_mutate': 'Pay the monstrous activation or mutate alternative casting cost with valid host; verify exact resulting mechanic event and source identity.',
 'becomes_targeted': 'Actually announce a legal spell targeting the source and leave its target trigger queued for the source-removal response.',
 'other_zone_change': 'Inspect exact from/to zones and actual producer; preserve old incarnation versus explicit destination IDs and snapshot roles.',
 'damage_received': 'Paid Flame Jab/Lightning Bolt into canonical creature, verify actual DamageEvent amount/recipient; normal SBAs; bounce source or exile dead source before reflection trigger.',
 'damage_dealt_or_combat_damage': 'Actual noncombat damage producer or legal attack/block and combat damage. Verify damage source/recipient, event amount and combat restriction before response.',
 'attack_or_block': 'Normal legal declare attackers/blockers after eligibility setup; pause at announced trigger before combat damage, then remove event object separately from owner.',
 'tap_or_untap': 'Legal printed tap ability or an actual spell taps the source; verify tap event and cost, keep trigger queued during removal response.',
 'turn_step': 'Generate a real upkeep/end-step event for the appropriate player; check intervening conditions both when triggering and resolving.',
 'spell_cast': 'Paid qualifying source/spell cast; inspect cast trigger context and resolve removal response while the original spell remains on stack.',
 'draw_or_discard': 'Actual paid draw/discard action, preserving destination cards and player roles; cannot replace with hand mutation.',
 'activated_ability': 'Actual source cast/play, pay printed activation resources, verify stack target assignments, then remove source while ability is pending.',
 'spell_or_outer_effect': 'Actual paid spell cast with selected targets and resources. Tagged damage source is commonly a targeted creature: source leaves may correctly make the target illegal, so no automatic LKI inference.',
 'other_trigger': 'Review exact typed matcher and Oracle before constructing a producer; no generic trigger injection.'}

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--scan',default='reports/runtime-audit/damage-source-candidates.json');ap.add_argument('--coverage',default='reports/runtime-audit/damage-source-reviewed-coverage.json');ap.add_argument('--out',default='reports/runtime-audit/damage-source-producer-groups.json');a=ap.parse_args()
    scan=json.loads(Path(a.scan).read_text());coverage=json.loads(Path(a.coverage).read_text());reviewed={r['card']for r in coverage['rows']};groups=defaultdict(list)
    for r in scan['rows']:
        if r['effective_source_kind'] not in ('tagged_object','tagged_filter_source') and not r['amount_tag_references']:
            continue
        row={k:r[k]for k in ('card','effect_path','effect_kind','ability_path','ability_kind','etb_contexts','effective_source_kind','source_binding_path','amount_tag_references','oracle_text','artifact_checksum')}
        row.update(trigger_kinds=trigger_names(r['trigger']),trigger_label=(r['trigger']or{}).get('label'),has_some_native_evidence_in_focused_ledger=r['card']in reviewed,status='candidate_grouping_only')
        groups[group(r)].append(row)
    output={'scope':'Frozen typed damage-source bindings or tagged numeric references, grouped by actual event producer. Grouping is an execution plan, not a correctness verdict. Some native evidence for a card does not certify every ability path.','scan_sha256':hashlib.sha256(Path(a.scan).read_bytes()).hexdigest(),'coverage_sha256':hashlib.sha256(Path(a.coverage).read_bytes()).hexdigest(),'source_scan':a.scan,'source_coverage':a.coverage,'candidate_paths':sum(map(len,groups.values())),'candidate_payload_names':len({r['card']for rows in groups.values()for r in rows}),'groups':[]}
    for family,rows in sorted(groups.items(),key=lambda kv:-len({r['card']for r in kv[1]if not r['has_some_native_evidence_in_focused_ledger']})):
        names={r['card']for r in rows};remaining={r['card']for r in rows if not r['has_some_native_evidence_in_focused_ledger']};output['groups'].append({'family':family,'candidate_paths':len(rows),'payload_names':len(names),'payload_names_without_focused_native_evidence':len(remaining),'bounded_producer':PRODUCERS[family],'remaining_names':sorted(remaining),'rows':rows})
    Path(a.out).write_text(json.dumps(output,indent=2)+'\n');print(json.dumps({'candidate_paths':output['candidate_paths'],'candidate_payload_names':output['candidate_payload_names'],'groups':[{k:g[k]for k in ('family','candidate_paths','payload_names_without_focused_native_evidence')}for g in output['groups']]},indent=2))
    md=['# Tagged damage source and numeric-reference producer groups','','This is a bounded-fixture plan, not a promotion of any unexecuted card. Name-level prior evidence does not certify every compiled ability path.','', '| Producer family | Paths | Payload names without focused native evidence |', '| --- | ---: | ---: |']
    for g in output['groups']:md.append(f"| {g['family']} | {g['candidate_paths']} | {g['payload_names_without_focused_native_evidence']} |")
    for g in output['groups']:md.extend(['',f"## {g['family']}",'',g['bounded_producer']])
    Path(a.out).with_suffix('.md').write_text('\n'.join(md)+'\n')
if __name__=='__main__':main()
