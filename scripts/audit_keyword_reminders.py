#!/usr/bin/env python3
"""Audit CR 701/702 reminder prose with the current strict native compiler.
Usage: python3 scripts/audit_keyword_reminders.py /path/to/official-rules.txt [output-directory] [--all-wordings]
Requires: cargo build -p ironsmith-compiler --example audit_keyword_reminders
"""
import argparse, collections, concurrent.futures, hashlib, json, pathlib, re, subprocess, sys
from keyword_reminder_selection import select_representatives, selection_report
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('rules')
parser.add_argument('output_directory', nargs='?')
parser.add_argument('--all-wordings', action='store_true', help='Regression sweep of every observed wording instead of one descriptive representative per family')
parser.add_argument('--bind-costs', action='store_true', help='Bind explicit its KEYWORD cost slots from the corresponding declaration; preserve raw text in the evidence')
args = parser.parse_args()
ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = pathlib.Path(args.output_directory) if args.output_directory else ROOT / 'reports/keyword-reminder-audit'
OUT.mkdir(parents=True, exist_ok=True)
def norm(s):
    return re.sub(r'\s+', ' ', s.replace('’', "'").replace('−','-').replace('—','-')).strip().lower()
def digest(p):
    return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
local = json.loads(subprocess.check_output(['node','--input-type=module','-e', '''import {MTG_KEYWORD_RULES as rules} from './web/ui/src/lib/mtg-keyword-rules.generated.js'; import {MTG_KEYWORD_REMINDERS as reminders} from './web/ui/src/lib/mtg-keyword-reminders.generated.js'; import {OFFICIAL_KEYWORD_FORMS as forms} from './web/ui/src/lib/keyword-forms.generated.js'; console.log(JSON.stringify({rules,reminders,forms}));'''],cwd=ROOT))
rules_text = pathlib.Path(args.rules).read_text(encoding='utf-8-sig')
entries=[]
for rule,title in re.findall(r'^(70[12]\.\d+)\. (.+)$',rules_text,re.M):
    if rule.endswith('.1'): continue
    old = next((x for x in local['rules'] if norm(x['title'])==norm(title)), None)
    aliases = list(dict.fromkeys([norm(title)] + (old['aliases'] if old else [])))
    # Inflected and phrasal forms come from the compiler's keyword-form lexicon.
    lexicon = next((f for f in local['forms'] if f['name']==norm(title)), None)
    forms = [f for f in ([lexicon['participle'], lexicon['designation']] + lexicon['variants'] if lexicon else []) if f]
    cost_noun = lexicon['costNoun'] if lexicon else None
    entries.append(dict(rule=rule,title=title,kind='ability' if rule.startswith('702') else 'action',aliases=aliases,forms=list(dict.fromkeys(forms)),cost_noun=cost_noun,in_app_catalog=old is not None))
# A separately numbered keyword owns its name, even if the old UI lists it as an alias.
owners={norm(e['title']):e['rule'] for e in entries}
for e in entries:
    e['aliases']=[a for a in e['aliases'] if owners.get(norm(a),e['rule'])==e['rule']]
aliases = sorted([(norm(a), e['rule']) for e in entries for a in e['aliases']],key=lambda x:-len(x[0]))
patterns=[(a,r,re.compile(r'(?<![\w-])'+re.escape(a)+r'(?:s|ed|ing)?(?![\w])')) for a,r in aliases]
# Irregular lexicon forms the suffix heuristic misses ("foretold", "beamed up").
patterns+=[(norm(f),e['rule'],re.compile(r'(?<![\w-])'+re.escape(norm(f))+r'(?![\w])')) for e in entries for f in e['forms']
           if not any(norm(f)==a or re.fullmatch(re.escape(a)+r'(?:s|ed|ing)?',norm(f)) for a in e['aliases'])]
cards=json.loads((ROOT/'cards.json').read_text())
samples={}; ambiguous=[]
action_patterns=json.loads((ROOT/'scripts/keyword_reminder_associations.json').read_text())
excluded=[]
def add(rule,text,example,card,face,method):
    e=next(e for e in entries if e['rule']==rule)
    ancillary=re.search(r'^(This effect lasts indefinitely|It loses all other creature types|Equipment, Auras you control|Assassins, Mercenaries, Pirates|If a creature has multiple instances|A player with ten or more poison counters|as long as this creature is on the battlefield)',text,re.I)
    if e['title'] in ('Solved','Max Speed','Visit') or ancillary:
        excluded.append(dict(rule=rule,card=card['name'],prefix=example,text=text))
        return
    if e['kind']=='action':
        pattern=action_patterns.get(e['title'])
        valid=pattern and re.search(pattern,text,re.I)
    else:
        valid=(e['title']=='Offering' and re.match(r'^\w+ offering\b',norm(example))) or any(re.match(re.escape(norm(a))+r'(?![\w])',norm(example)) or re.search(r'\b(?:have|has|gain|gains) '+re.escape(norm(a))+r'(?: [0-9x]+)?[.!]?$',norm(example)) for a in e['aliases'])
    if e['title']=='Ward' and not re.match(r'^Whenever .*becomes the target',text,re.I): valid=False
    if any(re.match(re.escape(norm(a))+r' (?:costs|abilities|spells|creatures)\b',norm(example)) for a in e['aliases']): valid=False
    if not valid:
        excluded.append(dict(rule=rule,card=card['name'],prefix=example,text=text))
        return
    # One source-card context per distinct reminder wording; retain its declaration.
    metadata='Type: '+face.get('type_line',card.get('type_line','Creature'))
    mana=face.get('mana_cost',card.get('mana_cost',''))
    if mana: metadata+='\nMana cost: '+mana
    power=face.get('power',card.get('power')); toughness=face.get('toughness',card.get('toughness'))
    if power is not None and toughness is not None: metadata+=f'\nPower/Toughness: {power}/{toughness}'
    name=face.get('name',card['name'])
    declaration = example
    if rule in ('702.96', '702.148'):
        label = 'Overload' if rule == '702.96' else 'Cleave'
        lines = face.get('oracle_text', card.get('oracle_text', '')).splitlines()
        example = '\n'.join([example] + [line for line in lines if not re.match(r'^'+label+r'\b', line)])
    if rule in ('702.172', '702.183', '702.155', '702.165'):
        lines = face.get('oracle_text', card.get('oracle_text', '')).splitlines()
        index = next((i for i, line in enumerate(lines) if re.match(r'^(Spree|Tiered|Read ahead|Backup)\b', line)), None)
        if index is not None: example = example + '\n' + '\n'.join(lines[index + 1:])
    # Card names are deliberately preserved: the parser uses them for self references.
    key=(rule,text)
    if key not in samples:
        samples[key]=dict(rule=rule,text=text,baseline=example,declaration=declaration,metadata=metadata,name=name,card=card['name'],oracle_id=card.get('oracle_id'),source=method,full_oracle_text=face.get('oracle_text', card.get('oracle_text', '')))
    return key
for card in cards:
    if card.get('lang','en')!='en' or card.get('border_color')=='silver' or card.get('security_stamp')=='acorn':continue
    if card.get('released_at','')>'2026-10-26': continue
    for face in card.get('card_faces') or [card]:
        text=face.get('oracle_text','')
        # Balanced parentheses include nested mana explanations, unlike the UI sampler.
        depth=0; start=None
        for i,ch in enumerate(text):
            if ch=='(':
                if depth==0:start=i
                depth+=1
            elif ch==')' and depth:
                depth-=1
                if depth:continue
                body=text[start+1:i].strip()
                prefix=text[:start].split('\n')[-1].split(')')[-1].strip()
                prefix=re.split(r'(?<=[.!?])\s+(?=[A-Z])',prefix)[-1]
                if not prefix:continue
                prefix=re.sub(r'\([^()]*\)','',prefix).strip()
                hits=[]
                for alias,rule,pat in patterns:
                    for m in pat.finditer(norm(prefix)):
                        hits.append((m.start(),m.end(),rule,alias))
                # Longest alias at a span wins (manifest dread, not manifest).
                hits=[h for h in hits if not any(o[0]<=h[0] and o[1]>=h[1] and o[1]-o[0]>h[1]-h[0] for o in hits)]
                declared=[norm(k) for k in card.get('keywords',[])]
                if not hits: continue
                direct=any(h[0]==0 and h[2].startswith('702.') for h in hits)
                ability_hits=[h for h in hits if h[2].startswith('702.')]
                action_hits=[h for h in hits if h[2].startswith('701.') and any(e['rule']==h[2] and e['title'] in action_patterns and re.search(action_patterns[e['title']],body,re.I) for e in entries)]
                last=max(action_hits or (ability_hits if direct else hits),key=lambda h:h[1])
                # Don't assign a combined flying/haste reminder to either individually.
                tail=norm(prefix)[last[1]:]
                if len(tail)>200:continue
                distinct=set(h[2] for h in hits)
                if len(distinct)>1 and re.fullmatch(r'[\w ,{}+/-]+',norm(prefix)):
                    ambiguous.append(dict(card=card['name'],prefix=prefix,text=body,rules=sorted(distinct)));continue
                add(last[2],body,prefix,card,face,'nearest preceding keyword')
# Add the app's hand-selected representative when it is from an eligible source card.
for alias,rem in local['reminders'].items():
    rule=next((r for a,r in aliases if a==norm(alias)),None)
    card=next((c for c in cards if c['name']==rem['card'] and c.get('released_at','')<='2026-10-26'),None)
    if not rule or not card:continue
    face=next((f for f in card.get('card_faces',[]) if rem['text'] in f.get('oracle_text','')),card)
    add(rule,rem['text'],rem.get('example') or alias,card,face,'app representative')
if not args.all_wordings:
    selection = selection_report(list(samples.values()), entries)
    (OUT/'reminder-selection.json').write_text(json.dumps(selection, indent=2, ensure_ascii=False)+'\n')
    unresolved = [row['rule'] for row in selection if row['candidate_count'] and row['selected'] is None]
    if unresolved:
        raise SystemExit('Generic reminder review required for '+', '.join(unresolved)+
                         '; see reminder-selection.json. Excluded examples are not fallback definitions.')
    representatives = [dict(row['selected'], selection_reason=row['reason'])
                       for row in selection if row['selected'] is not None]
    samples = {(probe['rule'], probe['text']): probe for probe in representatives}
if args.bind_costs:
    bindings = json.loads(subprocess.check_output(['node', '--input-type=module', '-e',
        "import fs from 'node:fs'; import {expandOfficialReminderDefinition as expandOfficial} from './web/ui/src/lib/official-reminder-definitions.js'; import {bindReminderCostFromDeclaration as bind, bindReminderValueFromDeclaration as bindValue} from './web/ui/src/lib/reminder-costs.js'; console.log(JSON.stringify(JSON.parse(fs.readFileSync(0, 'utf8')).map(p => {const expanded = expandOfficial(p.text, p.rule, p.declaration || p.baseline, p.full_oracle_text); const cost = bind(expanded, p.declaration || p.baseline); const value = bindValue(cost.text, p.declaration || p.baseline); return {...cost, text: value.text, official_expansion: expanded !== p.text, value_binding: value.binding};})));"],
        input=json.dumps(list(samples.values())), cwd=ROOT, text=True))
    for probe, bound in zip(samples.values(), bindings):
        if bound['text'] != probe['text']:
            probe['original_text'] = probe['text']
            probe['text'] = bound['text']
            probe['cost_binding'] = bound['binding']
            probe['value_binding'] = bound['value_binding']
            probe['official_expansion'] = bound['official_expansion']
cached={}
if (OUT/'summary.json').exists() and (OUT/'results.jsonl').exists():
    previous=json.loads((OUT/'summary.json').read_text())
    if previous.get('compiler_sha256')==digest(ROOT/'target/debug/examples/audit_keyword_reminders'):
        old_inputs={x['id']:x for x in map(json.loads,(OUT/'inputs.jsonl').read_text().splitlines())}
        for r in map(json.loads,(OUT/'results.jsonl').read_text().splitlines()):
            q=old_inputs[r['id']]
            cached[(q['rule'],q['text'],q['baseline'],q['metadata'],q['name'])]=r
requests=[]
for i,s in enumerate(samples.values()):
    s['id']=i
    # Cross-references (e.g. flying mentions flying blockers) are not necessarily circular.
    e=next(e for e in entries if e['rule']==s['rule'])
    own=e['aliases']+e['forms']+([e['cost_noun']+' cost'] if e['cost_noun'] else [])
    s['mentions_own_keyword']=any(re.search(r'(?<!\w)'+re.escape(norm(a))+r'(?!\w)',norm(s['text'])) for a in own)
    requests.append(s)
(OUT/'inputs.jsonl').write_text(''.join(json.dumps(s,ensure_ascii=False)+'\n' for s in requests))
(OUT/'catalog.json').write_text(json.dumps(entries,indent=2,ensure_ascii=False)+'\n')
(OUT/'excluded-associations.json').write_text(json.dumps(excluded,indent=2,ensure_ascii=False)+'\n')
(OUT/'ambiguous-reminders.json').write_text(json.dumps(ambiguous,indent=2,ensure_ascii=False)+'\n')
print(f'{len(entries)} official families; {len(requests)} distinct probes; {len(ambiguous)} ambiguous associations excluded',flush=True)
pending=[]
reused=[]
for q in requests:
    old=cached.get((q['rule'],q['text'],q['baseline'],q['metadata'],q['name']))
    if old is not None:
        old['id']=q['id']; reused.append(old)
    else:pending.append(q)
print(f'{len(reused)} cached exact-input results; {len(pending)} new probes',flush=True)
def run_shard(index):
    chunk=pending[index::4]
    with (OUT/f'shard-{index}.jsonl').open('w') as out,(OUT/f'shard-{index}.stderr.log').open('w') as err:
        subprocess.run([str(ROOT/'target/debug/examples/audit_keyword_reminders')],input=''.join(json.dumps(s)+'\n' for s in chunk),stdout=out,stderr=err,check=True,cwd=ROOT,text=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    list(pool.map(run_shard,range(4)))
(OUT/'results.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in reused)+''.join((OUT/f'shard-{i}.jsonl').read_text() for i in range(4)))
results={x['id']:x for x in map(json.loads,(OUT/'results.jsonl').read_text().splitlines())}
assert len(results)==len(requests)
for s in requests:
    result=results[s['id']]
    s['status']=result['reminder']['status']
    s['no_op']=result['no_op'];s['same_structure']=result['same_structure']
    s['error']=result['reminder'].get('error')
    s['loss_count']=result['reminder'].get('loss_count',0)
    s['baseline_status']=result['baseline']['status']
for e in entries:
    ss=[s for s in requests if s['rule']==e['rule']]
    e['samples']=len(ss)
    e['named_control_compiled']=sum(s['baseline_status']=='compiled' for s in ss)
    e['compiled']=sum(s['status']=='compiled' and not s['no_op'] for s in ss)
    e['no_op']=sum(s['no_op'] for s in ss)
    e['rejected']=sum(s['status']=='rejected' for s in ss)
    e['lossy']=sum(s['loss_count']>0 for s in ss)
    e['panics']=sum(s['status']=='panic' for s in ss)
    e['keyword_free_compiled']=sum(s['status']=='compiled' and not s['no_op'] and not s['mentions_own_keyword'] for s in ss)
    e['same_structure']=sum(s['same_structure'] and not s['no_op'] for s in ss)
    e['status']='no sample' if not ss else 'all compile' if e['compiled']==len(ss) else 'mixed' if e['compiled'] else 'none compile'
summary={kind:dict(collections.Counter(e['status'] for e in entries if kind=='total' or e['kind']==kind)) for kind in ['ability','action','total']}
manifest=dict(official_expansion_sha256=digest(ROOT/'web/ui/src/lib/official-reminder-definitions.js'), expanded_official_probes=sum(p.get('official_expansion', False) for p in requests), cost_binding_enabled=args.bind_costs, bound_cost_probes=sum('cost_binding' in p for p in requests), cost_binder_sha256=digest(ROOT/'web/ui/src/lib/reminder-costs.js'), selection='all-wordings' if args.all_wordings else 'generic-definition-per-family', selection_sha256=digest(ROOT/'scripts/keyword_reminder_selection.py'), generic_definitions_sha256=digest(ROOT/'scripts/keyword_reminder_generic_definitions.json'), audit_date='2026-10-26', source_url='https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt', harness_sha256=digest(ROOT/'crates/ironsmith-compiler/examples/audit_keyword_reminders.rs'), audit_script_sha256=digest(__file__), association_patterns_sha256=digest(ROOT/'scripts/keyword_reminder_associations.json'), rules_effective_date=re.search(r'effective as of ([^.]+)',rules_text)[1],rules_sha256=digest(args.rules),cards_sha256=digest(ROOT/'cards.json'),git_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),compiler_sha256=digest(ROOT/'target/debug/examples/audit_keyword_reminders'),summary=summary,sample_count=len(requests))
(OUT/'summary.json').write_text(json.dumps(manifest,indent=2)+'\n')
(OUT/'keywords.json').write_text(json.dumps(entries,indent=2,ensure_ascii=False)+'\n')
(OUT/'probes.json').write_text(json.dumps(requests,indent=2,ensure_ascii=False)+'\n')
print(json.dumps(manifest,indent=2))
# Validate every representative face context without any rules text, so metadata
# errors cannot be mislabeled as reminder-grammar failures.
contexts=list(dict.fromkeys((q['name'],q['metadata']) for q in requests))
context_requests=[dict(id=i,name=n,metadata=m,text='',baseline='') for i,(n,m) in enumerate(contexts)]
checked=subprocess.run([str(ROOT/'target/debug/examples/audit_keyword_reminders')],input=''.join(json.dumps(q)+'\n' for q in context_requests),capture_output=True,text=True,check=True,cwd=ROOT)
context_results=[json.loads(line) for line in checked.stdout.splitlines()]
assert len(context_results)==len(context_requests)
assert all(r['reminder']['status']=='compiled' and r['no_op'] for r in context_results), 'Invalid face metadata: inspect context validation'
(OUT/'context-validation.json').write_text(json.dumps(dict(count=len(contexts),all_compiled=True),indent=2)+'\n')

# Keep a readable complete inventory next to the machine-readable evidence.
lines=['# Official keyword reminder compilation audit', '',
       'Audit date: 2026-10-26. Rules effective '+manifest['rules_effective_date']+'.', '',
       'Rules source: https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt', '',
       '## What was measured', '',
       'Every numbered keyword family in CR 701 and 702 is inventoried. General introductory rules 701.1 and 702.1 are excluded. Aliases and variants (for example landwalk types, cycling types, and partner variants) count under their CR family. Ability words, named tokens, and other game terms outside those sections are outside this denominator.', '',
       ('Each distinct observed reminder wording is compiled once,' if args.all_wordings else 'One eligible generic reminder per CR family is selected; combined definitions, granted scopes, card-specific exceptions, and separate keyword variants are excluded before comparing descriptive detail. The selected definition is compiled') + ' as unparenthesized functional text with the keyword declaration removed, using a representative source card’s name, type line, mana cost, and power/toughness. Other abilities are omitted. The named declaration is separately compiled as a control. No custom mechanic registry or permissive unsupported fallback is supplied.', '',
       'Input reminders come from eligible English entries in the local cards.json, released by the audit date, excluding silver-border and acorn cards (eligible black-border cards in funny sets are included). The script finds balanced parentheses and matches the official keyword names/aliases, including grants that the corpus’s keyword metadata omits. Ability probes require a direct keyword declaration or a simple explicit grant; action probes use reviewed association patterns in scripts/keyword_reminder_associations.json. Ambiguous combined reminders and unrelated explanatory parentheses are excluded and recorded. Repeated copies of the same wording use one source. The default audit chooses one generic reminder per family and records selection/rejection reasons in reminder-selection.json; families without an eligible generic observation require a reviewed full-rule definition rather than falling back to an exception; --all-wordings retains distinct costs, numbers, and wordings for a broader regression sweep. This is a complete keyword-family inventory and a finite observed-wording audit, not an exhaustive enumeration of every possible reminder or parameter.', '',
       'A successful strict parse with nonempty mechanics is counted as compiled. A diagnostic-marked parse, empty result, or panic is separately reported. “No sample” means no eligible standalone reminder was extracted; it does not mean the named keyword is unsupported. Some reminders rely on the missing keyword’s cost, target, selected object, other face, or other card text. Those failures are meaningful for reminder-only compilation but are not failures of the named mechanic.', '',
       '**Compilation is not proof of semantic equivalence.** Generic prose may lower to a restriction or trigger without giving the card the actual named keyword identity. Structural comparisons retain all execution fields examined by the harness and may also differ because of presentation hints or because an action’s original declaration was inside a trigger. Different structures are review leads, not automatic runtime failures. Exact structure matches are stronger evidence, but this audit does not replace gameplay scenario tests.', '',
       'The ordinary official-card path removes parenthetical reminders before semantic parsing (crates/ironsmith-compiler-grammar/src/preprocess.rs, authored_rules_tokens). Some mechanics extract supplemental facts before stripping. These numbers measure the capability to compile reminder prose on its own, not a claim that ordinary official cards are already implemented by reading their reminders.', '',
       ('Explicit named cost references were bound from the corresponding source declaration using the same binder as custom mechanics. Original prose and complete bound prices are retained in probes.json. No price is inferred when the declaration does not supply it.' if args.bind_costs else 'Named cost references are left unbound in this raw-prose audit. Run with --bind-costs to test explicit declaration parameters separately.'), '',
       '## Counts', '',
       '| Category | Abilities | Actions | Total |','|---|---:|---:|---:|']
for status in ['all compile','mixed','none compile','no sample']:
    lines.append('| '+status+' | '+' | '.join(str(summary[k].get(status,0)) for k in ['ability','action','total'])+' |')
lines+=['',f'Distinct reminder probes: **{len(requests)}**. Strict successful nonempty results: **{sum(e["compiled"] for e in entries)}**. Rejections: **{sum(e["rejected"] for e in entries)}**. Empty results: **{sum(e["no_op"] for e in entries)}**. Panics: **{sum(e["panics"] for e in entries)}**. Successful parses recording loss diagnostics: **{sum(e["lossy"] for e in entries)}**.', '',
        '“All compile” means all sampled wordings for that family compiled, not that every possible wording works. “Mixed” means at least one compiled and at least one did not.', '',
        '## Selected findings', '',
        'Named keyword identity and executable behavior are intentionally distinct. Defender’s reminder describes a generic cannot-attack restriction; Flying’s reminder describes a blocking restriction. Custom keywords may share these behaviors without acquiring the official keyword identity. Exact structure matches are therefore diagnostic data, not the correctness criterion; gameplay equivalence and identity need separate tests.', '',
        'Reminders that say “its flashback cost”, “its escape cost”, and similar phrases omit a parameter supplied by the removed keyword declaration. A bare reminder cannot recover that missing value. The --bind-costs audit binds those explicit parameters through the custom-mechanic cost binder; successful binding still requires the surrounding prose to compile.', '',
        'The app catalog is missing these current CR families: '+', '.join(e['title'] for e in entries if not e['in_app_catalog'])+'. They are still included in this inventory.', '',
        '## Full inventory', '',
        '| Rule | Keyword family | Kind | Result | Compiled / sampled | Exact structure matches |',
        '|---|---|---|---|---:|---:|']
for e in entries:
    lines.append(f'| {e["rule"]} | {e["title"]} | {e["kind"]} | {e["status"]} | {e["compiled"]}/{e["samples"]} | {e["same_structure"]} |')
lines+=['','## Evidence and reproduction','','- keywords.json: complete per-family counts and catalog coverage.',
        '- probes.json / inputs.jsonl: every tested reminder, source card, parameterized declaration, metadata, result and error.',
        '- results.jsonl: raw compiled structures for reminder and named control, plus parse-loss diagnostics.',
        '- excluded-associations.json / ambiguous-reminders.json: excluded source associations.',
        '- summary.json: exact rules and card-corpus hashes, compiler binary hash, Git HEAD and totals.',
        '', 'The binary was built from the current working tree, including uncommitted work and the recent reminder parser changes. This is a native compiler audit; it does not verify a deployed WASM bundle.', '',
        '```sh', 'curl -fsSL "https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt" -o /tmp/ironsmith-audit-rules.txt',
        'cargo build -p ironsmith-compiler --example audit_keyword_reminders',
        'python3 scripts/audit_keyword_reminders.py /tmp/ironsmith-audit-rules.txt', '```','']
(OUT/'README.md').write_text('\n'.join(lines))
# Consolidated outputs supersede per-process intermediate files.
for i in range(4):
    (OUT/f'shard-{i}.jsonl').unlink(missing_ok=True)
    (OUT/f'shard-{i}.stderr.log').unlink(missing_ok=True)
