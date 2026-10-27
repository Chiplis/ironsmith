"""Select generic definitions before comparing how descriptive they are.

A longer price, a second keyword's explanation, or a card's extra permission
must never make its reminder the family definition. Rejected observations are
retained as supplemental regression inputs, not silently rewritten as evidence.
"""
import re
import json
from pathlib import Path

GENERIC_DEFINITIONS = json.loads(Path(__file__).with_name("keyword_reminder_generic_definitions.json").read_text())

# Reviewed restrictions on the *definition*, not card-name exceptions. Parameters
# (costs, amounts, qualities) remain valid; variants that change the base mechanic
# belong in the all-wordings regression sweep.
EXCLUSIONS = {
    '701.22': [(r'\b(?:treasure|token|artifact)\b', 'includes a separate token-creation instruction')],
    '701.37': [(r'\{s\} can be paid', 'includes the activation cost’s snow-mana explanation')],
    '701.67': [(r'\bexhaust\b', 'combines the payment definition with Exhaust')],
    '702.11': [(r'\bhexproof from\b', 'restricted Hexproof variant')],
    '702.14': [(r'\b\w+walk\b\s*,', 'combines multiple landwalk abilities')],
    '702.16': [(r'\band from\b', 'combines multiple protection qualities')],
    '702.19': [(r'\bover planeswalkers\b', 'Trample-over-planeswalkers variant')],
    '702.21': [(r'\b(?:waterbend|collect evidence)\b|they can tap', 'embeds another mechanic in the Ward price')],
    '702.24': [(r'\{s\} can be paid', 'includes a separate snow-mana explanation')],
    '702.29': [(r'^(?!cycling\b)', 'typecycling variant rather than base Cycling')],
    '702.33': [(r'\band/or\b', 'combines two separately payable Kicker costs')],
    '702.34': [(r'\bto behold\b', 'includes another keyword’s cost definition')],
    '702.37': [(r'\bmegamorph\b', 'Megamorph variant rather than base Morph')],
    '702.40': [(r'copies become tokens', 'permanent-spell copy explanation, not part of Storm')],
    '702.41': [(r'\baffinity for (?:outlaws|historic permanents)\b', 'expands a batching term rather than the affinity parameter')],
    '702.43': [(r'\bsunburst\b', 'Modular—Sunburst replaces the ordinary counter parameter')],
    '702.49': [(r'\bcommander ninjutsu\b', 'Commander Ninjutsu adds a command-zone permission')],
    '702.54': [(r'^bloodthirst x\b', 'Bloodthirst X has a special definition')],
    '702.56': [(r'copies become tokens', 'permanent-spell copy explanation, not part of Replicate')],
    '702.82': [(r'\bwhere x is\b', 'the source card supplies an additional value definition')],
    '702.119': [(r'^emerge from\b', 'variant sacrifices a different permanent quality')],
    '702.122': [(r"creatures can't be attached", 'includes an unrelated attachment rule')],
    '702.124': [(r'^(?!partner[.!]?\s*$)', 'separate partner variant')],
    '702.132': [(r'you choose the value of x', 'includes the source spell’s X announcement')],
    '702.174': [(r'^gift (?!a card\b)', 'non-card gift is a distinct supplied effect parameter')],
    '702.184': [(r"artifact creature at", 'includes this card’s station threshold')],
    '702.185': [(r'hand or (?:your )?graveyard', 'includes a separate graveyard casting permission')],
}
DECLARATION_ONLY = {'702.11', '702.14', '702.16', '702.19', '702.29', '702.37',
                    '702.33', '702.41', '702.43', '702.49', '702.54', '702.82', '702.119', '702.124', '702.174'}


def normalized(value):
    return re.sub(r'\s+', ' ', value.replace('’', "'").replace('—', '-').replace('–', '-')).strip().casefold()


def declaration(probe):
    return normalized(probe.get('declaration', probe['baseline']))


def rejection_reasons(probe, entry):
    decl, text = declaration(probe), normalized(probe['text'])
    reasons = []
    for pattern, reason in EXCLUSIONS.get(entry['rule'], []):
        subject = decl if entry['rule'] in DECLARATION_ONLY else decl + '\n' + text
        if re.search(pattern, subject, re.I):
            reasons.append(reason)
    if re.search(r'\bwhere x is\b', decl):
        reasons.append('source card supplies the value of X separately from the keyword')
    if re.search(r'\b(?:he|she|his|her)\b', text):
        reasons.append('uses a card-specific gendered source reference')
    source_name = normalized(probe.get('card') or probe.get('name', ''))
    # A proper-name self reference is valid on its source card, but cannot be
    # copied into the definition of an arbitrary custom mechanic.
    names = {source_name, source_name.split(',')[0]}
    if any(len(name) > 3 and re.search(r'(?<!\w)' + re.escape(name) + r'(?!\w)', text)
           for name in names):
        reasons.append('definition refers to the source card by name')
    if re.search(r'\bonly\s*\.\s*\.\s*\.\s*once', text):
        reasons.append('card-specific humorous wording')
    aliases = entry.get('aliases', [entry['title']])
    if any(len(re.findall(r'(?<![\w-])' + re.escape(normalized(alias)) + r'(?![\w-])', decl)) > 1 for alias in aliases):
        reasons.append('combines repeated instances of the keyword')
    if re.search(r'\b(?:twice|again|times)\b', decl):
        reasons.append('repeats an action rather than defining one occurrence')
    if re.search(r'\b(?:have|has|gain|gains)\s+(?:' + '|'.join(re.escape(normalized(a)) for a in aliases) + r')\b', decl):
        reasons.append('reminder is scoped to a granted ability')
    return reasons


def descriptive_rank(probe, entry):
    decl = declaration(probe)
    # Cost/number spelling cannot win a length comparison. The declaration's
    # primary keyword outranks aliases representing a different variant.
    prose = re.sub(r'\{[^}]+\}(?:\s*\{[^}]+\})*', '{cost}', normalized(probe['text']))
    prose = re.sub(r'\b(?:\d+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\b', 'N', prose)
    primary = bool(re.match(re.escape(normalized(entry['title'])) + r'(?![\w-])', decl))
    standalone = any(re.match(re.escape(normalized(alias)) + r'(?![\w-])', decl)
                     for alias in entry.get('aliases', [entry['title']]))
    general_source = (entry["rule"] in {"702.11", "702.12", "702.16", "702.18", "702.21"} and "this permanent" in prose)
    return primary, standalone, general_source, len(prose.split()), len(prose), prose, normalized(probe['text']), probe['name']


def selection_report(probes, catalog):
    report = []
    for entry in catalog:
        candidates = [probe for probe in probes if probe['rule'] == entry['rule']]
        eligible, rejected = [], []
        for probe in candidates:
            reasons = rejection_reasons(probe, entry)
            if reasons:
                rejected.append(dict(text=probe['text'], declaration=probe.get('declaration', probe['baseline']),
                                     card=probe.get('card', probe['name']), reasons=reasons))
            else:
                eligible.append(probe)
        chosen = max(eligible, key=lambda probe: descriptive_rank(probe, entry)) if eligible else None
        reviewed = GENERIC_DEFINITIONS.get(entry['rule'])
        reason = 'Most descriptive eligible generic definition; costs and amounts do not increase its rank.'
        if reviewed and candidates:
            # Preserve the observed evidence without presenting derived prose as
            # a quotation from that card. Use an explicit neutral creature probe.
            source = chosen or min(candidates, key=lambda probe: (probe['text'], probe['name']))
            chosen = dict(source, observed_text=source['text'], observed_declaration=source.get('declaration', source['baseline']),
                          text=reviewed['text'], baseline=reviewed['declaration'], declaration=reviewed['declaration'],
                          name='Generic reminder probe', metadata=reviewed.get('metadata', 'Type: Creature\nMana cost: {3}\nPower/Toughness: 2/2'),
                          full_oracle_text=reviewed['declaration'], definition_source='official-rule-derived',
                          definition_rule=reviewed['rule'])
            reason = reviewed['reason']

        report.append(dict(rule=entry['rule'], candidate_count=len(candidates), eligible_count=len(eligible),
                           selected=chosen, rejected=rejected,
                           reason=reason if chosen else
                           'No eligible generic observed reminder. Requires a reviewed full-rule definition; excluded reminders are not a fallback.'))
    return report


def select_representatives(probes, catalog):
    return [dict(row['selected'], selection_reason=row['reason'])
            for row in selection_report(probes, catalog) if row['selected'] is not None]
