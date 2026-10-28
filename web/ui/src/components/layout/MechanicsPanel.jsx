import { useEffect, useMemo, useState } from 'react';
import useUiText from '@/i18n/useUiText';
import { useGame } from '@/context/GameContext';
import { CUSTOM_MECHANICS_CHANGED, deleteCustomMechanic, listCustomMechanics, saveCustomMechanic } from '@/lib/custom-card-library';
import { mechanicFormClash, probeMechanic, validateMechanic } from '@/lib/custom-mechanics';
import { FORM_FIELDS, REMINDER_TEMPLATES, customMechanicEntry, defaultForms, mechanicEntries } from '@/lib/mechanics-catalog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

const FILTERS = [['all', 'All'], ['custom', 'Custom'], ['official', 'Official']];
const emptyDraft = () => ({ keyword: '', reminder: '', participle: '', designation: '', costNoun: '', variants: '' });

function MechanicBadge({ entry }) {
  const ui = useUiText();
  return entry.custom
    ? <Badge className="border-amber-300/50 bg-amber-300/15 text-amber-100" data-testid="custom-mechanic-badge">{ui('Custom')}</Badge>
    : <Badge variant="outline" className="text-muted-foreground">{ui('Official')} {entry.rule}</Badge>;
}

function FormRows({ forms }) {
  const ui = useUiText();
  const rows = [
    ['Name', forms.name],
    ['Past participle', forms.participle],
    ['Designation', forms.designation],
    ['Cost name', forms.costNoun && ui('its {0} cost', { 0: forms.costNoun })],
    ['Other forms', forms.variants?.join(', ')],
  ];
  return <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm" data-testid="mechanic-forms">
    {rows.map(([label, value]) => <div key={label} className="contents">
      <dt className="card-forge-section-label self-center">{ui(label)}</dt>
      <dd className={cn('break-words', !value && 'text-muted-foreground')}>{value || '—'}</dd>
    </div>)}
  </dl>;
}

function MechanicDetail({ entry, onDeleted }) {
  const ui = useUiText();
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => { setConfirming(false); setError(''); }, [entry.id]);
  const remove = async () => {
    if (!confirming) return setConfirming(true);
    try { await deleteCustomMechanic(entry.name); onDeleted?.(); }
    catch (failure) { setError(String(failure?.message || failure)); }
  };
  return <article className="space-y-5" data-testid="mechanic-detail">
    <header className="flex flex-wrap items-center gap-3">
      <h3 className="text-xl font-semibold capitalize">{entry.name}</h3>
      <MechanicBadge entry={entry} />
      {!entry.custom && <span className="text-sm text-muted-foreground">{ui(entry.kind === 'action' ? 'Keyword action' : 'Keyword ability')}</span>}
    </header>
    <section className="space-y-2">
      <div className="card-forge-section-label">{ui(entry.custom ? 'Reminder text' : 'Rules summary')}</div>
      <p className="text-sm leading-6">{entry.summary}</p>
      {entry.custom && entry.expansion && entry.expansion !== entry.summary
        && <p className="text-xs text-muted-foreground">{ui('Compiled meaning:')} {entry.expansion}</p>}
      {entry.custom && <p className="text-xs text-muted-foreground">{ui('Defined by {0}', { 0: entry.sourceCard })}</p>}
      {entry.reminderSource && <p className="text-xs text-muted-foreground">{ui('As printed for {0} on {1}', { 0: entry.reminderSource.example, 1: entry.reminderSource.card })}</p>}
    </section>
    <section className="space-y-3">
      <div className="card-forge-section-label">{ui('Forms')}</div>
      <FormRows forms={entry.forms} />
    </section>
    {entry.custom && <section className="space-y-2 border-t border-white/15 pt-4">
      <p className="text-sm text-muted-foreground">{ui('Use this keyword on any card in the Card tab; its saved reminder applies automatically. Deleting it frees the name, and saved cards keep their own copy.')}</p>
      {error && <p role="alert" className="text-sm text-red-300">{error}</p>}
      <Button variant="destructive" onClick={() => void remove()}>{ui(confirming ? 'Confirm delete' : 'Delete mechanic')}</Button>
    </section>}
  </article>;
}

function draftDefinition(draft) {
  const forms = Object.fromEntries(['participle', 'designation', 'costNoun', 'variants']
    .map(field => [field, draft[field].trim()]).filter(([, value]) => value));
  return { keyword: draft.keyword, reminder: draft.reminder, ...(Object.keys(forms).length ? { forms } : {}) };
}

// Compiles the reminder on a test card as the user types, so a definition
// the compiler cannot read shows its error before it is saved.
function useMechanicCompileCheck(definition) {
  const { game } = useGame();
  const compile = useMemo(() => (game?.previewCustomCard ? input => game.previewCustomCard(input) : null), [game]);
  const key = definition ? JSON.stringify(definition) : '';
  const [check, setCheck] = useState({ key: '', status: 'idle' });
  useEffect(() => {
    if (!key || !compile) return undefined;
    let active = true;
    const timeoutId = window.setTimeout(async () => {
      setCheck({ key, status: 'checking' });
      try {
        const result = await probeMechanic(JSON.parse(key), compile);
        if (active) setCheck({ key, status: result.checked ? 'ok' : 'unchecked', ...result });
      } catch (failure) {
        if (active) setCheck({ key, status: 'error', message: String(failure?.message || failure), usage: failure?.usage });
      }
    }, 250);
    return () => { active = false; window.clearTimeout(timeoutId); };
  }, [key, compile]);
  if (!key || !compile) return { status: 'idle', compile };
  return check.key === key ? { ...check, compile } : { status: 'checking', compile };
}

function CompileCheck({ check }) {
  const ui = useUiText();
  if (check.status === 'idle') return null;
  if (check.status === 'checking') return <p className="text-xs text-muted-foreground" data-testid="mechanic-compile-check">{ui('Compiling reminder text…')}</p>;
  if (check.status === 'unchecked') return <p className="text-xs text-muted-foreground" data-testid="mechanic-compile-check">{ui('This kind of mechanic needs a declaration from its card, so its reminder is checked when a card uses it.')}</p>;
  if (check.status === 'ok') return <div className="space-y-1 text-xs" data-testid="mechanic-compile-check">
    <p className="text-emerald-300">{ui('Reminder text compiles.')}</p>
    {check.permanentAbility === false && <p className="text-amber-200" data-testid="mechanic-spell-only">{ui('It compiles as an instruction, so it only runs when an instant or sorcery resolves. On a creature or other permanent it does nothing. Start it with "{Cost}:", "When", or "If … would" to give permanents an ability.')}</p>}
  </div>;
  return <div role="alert" className="space-y-1 text-sm text-red-300" data-testid="mechanic-compile-check">
    <p>{ui('The reminder text does not compile.')}</p>
    {check.usage && <p className="text-xs text-muted-foreground">{ui('Test card text:')} “{check.usage}”</p>}
    <pre className="card-forge-codeblock whitespace-pre-wrap text-xs">{check.message}</pre>
  </div>;
}

const titleCase = keyword => keyword.replace(/^./, letter => letter.toUpperCase());

// A template's card line, written with the keyword being defined.
const templateUsage = (template, keyword) => {
  const name = keyword.trim() ? titleCase(keyword.trim()) : titleCase(template.keyword);
  return template.usage.replace(new RegExp(`^${titleCase(template.keyword)}`), name);
};

function TemplatePicker({ selected, keyword, onPick }) {
  const ui = useUiText();
  const template = REMINDER_TEMPLATES.find(candidate => candidate.kind === selected);
  return <section className="space-y-3 border border-white/10 p-3" data-testid="mechanic-templates">
    <div className="card-forge-section-label">{ui('Start from a template')}</div>
    <p className="text-sm text-muted-foreground">{ui('Pick the kind of mechanic to fill in its reminder, then edit the effect. Write {Cost} where the price goes: each card sets its own price after the keyword, and that price replaces {Cost} when the card compiles.')} {ui('Write {Number} where an amount goes; the card then writes the amount before its price, as in "Suspend 3—{1}{U}".')}</p>
    <div className="flex flex-wrap gap-2" role="group" aria-label={ui('Mechanic templates')}>
      {REMINDER_TEMPLATES.map(candidate => <Button key={candidate.kind} type="button" size="sm"
        variant={candidate.kind === selected ? 'default' : 'secondary'} aria-pressed={candidate.kind === selected}
        data-testid="mechanic-template" onClick={() => onPick(candidate)}>{ui(candidate.kind)}</Button>)}
    </div>
    {template && <div className="grid gap-1 text-xs" data-testid="mechanic-template-detail">
      <p className="text-muted-foreground">{ui(template.note)}</p>
      <p><span className="text-muted-foreground">{ui('On a card:')}</span> “{templateUsage(template, keyword)}”</p>
    </div>}
  </section>;
}

function MechanicForm({ saved, onSaved }) {
  const ui = useUiText();
  const [draft, setDraft] = useState(emptyDraft);
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState('');
  const [template, setTemplate] = useState(null);
  const set = field => event => { setSaveError(''); setDraft(current => ({ ...current, [field]: event.target.value })); };
  const defaults = defaultForms(draft.keyword);
  // Live validation uses exactly the checks saving will apply.
  const { definition, problem } = useMemo(() => {
    if (!draft.keyword.trim()) return { problem: ui('Name the mechanic.') };
    try {
      const def = validateMechanic({ ...draftDefinition(draft), sourceCard: 'the Mechanics tab' });
      const existing = saved.find(mechanic => mechanic.keyword === def.keyword);
      if (existing) return { problem: ui('"{0}" is already saved (from {1}). Delete it first to redefine it.', { 0: def.keyword, 1: existing.sourceCard }) };
      const clash = mechanicFormClash(saved, def);
      return clash ? { problem: clash.message } : { definition: def };
    } catch (failure) { return { problem: String(failure?.message || failure) }; }
  }, [draft, saved, ui]);
  const check = useMechanicCompileCheck(definition && draftDefinition(draft));
  const blocked = check.status === 'checking' || check.status === 'error';
  const save = async () => {
    if (!definition || busy || blocked) return;
    setBusy(true);
    try { const def = await saveCustomMechanic(draftDefinition(draft), { compile: check.compile }); setDraft(emptyDraft()); setTemplate(null); onSaved?.(def); }
    catch (failure) { setSaveError(String(failure?.message || failure)); }
    finally { setBusy(false); }
  };
  const forms = definition ? customMechanicEntry(definition).forms : null;
  return <form className="space-y-5" data-testid="mechanic-form" onSubmit={event => { event.preventDefault(); void save(); }}>
    <header className="space-y-1">
      <h3 className="text-xl font-semibold">{ui('New custom mechanic')}</h3>
      <p className="text-sm text-muted-foreground">{ui('Custom mechanics compile from their reminder text and keep their own identity: a custom keyword never counts as an official one.')}</p>
    </header>
    <label className="card-forge-field"><span className="card-forge-section-label">{ui('Keyword')}</span>
      <input className="card-forge-input" value={draft.keyword} onChange={set('keyword')} placeholder={ui("Blink")} maxLength={64} autoComplete="off" />
    </label>
    <TemplatePicker selected={template} keyword={draft.keyword} onPick={picked => {
      setSaveError(''); setTemplate(picked.kind);
      setDraft(current => ({ ...current, keyword: current.keyword.trim() ? current.keyword : picked.keyword, reminder: picked.reminder }));
    }} />
    <label className="card-forge-field"><span className="card-forge-section-label">{ui('Reminder text')}</span>
      <textarea className="card-forge-input min-h-24" value={draft.reminder} onChange={set('reminder')}
        placeholder={ui("{Cost}: Exile this creature, then return it to the battlefield under its owner's control.")} />
      <span className="text-xs text-muted-foreground">{ui('Write the full meaning without parentheses, as it would appear in the reminder.')}</span>
      <CompileCheck check={check} />
    </label>
    <fieldset className="space-y-4">
      <legend className="card-forge-section-label mb-2">{ui('Forms')}</legend>
      <p className="text-sm text-muted-foreground">{ui('Leave a form blank to use the one derived from the name. Forms let other rules text refer to the mechanic.')}</p>
      {FORM_FIELDS.map(spec => {
        const fallback = defaults[spec.field];
        const value = draft[spec.field].trim() || fallback;
        const first = String(value || '').split(',')[0].trim();
        return <div key={spec.field} className="grid gap-2 border border-white/10 p-3" data-testid={`mechanic-form-${spec.field}`}>
          <label className="card-forge-field"><span className="card-forge-section-label">{ui(spec.label)}</span>
            <input className="card-forge-input card-forge-input--compact" value={draft[spec.field]} onChange={set(spec.field)}
              placeholder={fallback || ui('None')} autoComplete="off" />
          </label>
          <p className="text-xs text-muted-foreground">{ui(spec.help)}</p>
          <ul className="grid gap-1 text-xs" aria-label={ui('Examples from official keywords')}>
            {spec.examples.map(example => <li key={example.keyword}>
              <span className="font-semibold">{example.keyword}</span> → <em>{example.form}</em>
              <span className="text-muted-foreground"> · “{example.text}”</span>
            </li>)}
          </ul>
          {first && <p className="text-xs"><span className="text-muted-foreground">{ui('On your card:')}</span> “{spec.preview(first)}”</p>}
        </div>;
      })}
    </fieldset>
    {forms && <section className="space-y-2"><div className="card-forge-section-label">{ui('Saved forms')}</div><FormRows forms={forms} /></section>}
    {(problem || saveError) && draft.keyword.trim() && <p role="alert" className="text-sm text-red-300">{saveError || problem}</p>}
    <Button type="submit" disabled={!definition || busy || blocked}>{ui(busy ? 'Saving…' : 'Save mechanic')}</Button>
  </form>;
}

export default function MechanicsPanel() {
  const ui = useUiText();
  const [saved, setSaved] = useState([]);
  const [error, setError] = useState('');
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState('all');
  const [selected, setSelected] = useState('new');
  useEffect(() => {
    let active = true;
    const load = async () => {
      try { const mechanics = await listCustomMechanics(); if (active) { setSaved(mechanics); setError(''); } }
      catch (failure) { if (active) setError(String(failure?.message || failure)); }
    };
    void load();
    window.addEventListener(CUSTOM_MECHANICS_CHANGED, load);
    return () => { active = false; window.removeEventListener(CUSTOM_MECHANICS_CHANGED, load); };
  }, []);
  const entries = useMemo(() => mechanicEntries(saved, { query, filter }), [saved, query, filter]);
  const entry = selected === 'new' ? null : mechanicEntries(saved).find(candidate => candidate.id === selected);
  return <div className="mechanics-panel min-h-0 overflow-y-auto p-4 sm:p-6" data-testid="mechanics-panel">
    <div className="mx-auto grid max-w-6xl gap-6 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
      <section className="min-w-0 space-y-3 lg:sticky lg:top-0 lg:self-start" aria-label={ui('Mechanics')}>
        <Button className="w-full" variant={selected === 'new' ? 'default' : 'secondary'} onClick={() => setSelected('new')}>{ui('New custom mechanic')}</Button>
        <input className="card-forge-input" type="search" value={query} onChange={event => setQuery(event.target.value)}
          placeholder={ui('Search mechanics or forms')} aria-label={ui('Search mechanics or forms')} />
        <div className="flex gap-2" role="group" aria-label={ui('Show')}>
          {FILTERS.map(([value, label]) => <Button key={value} size="sm" variant={filter === value ? 'default' : 'outline'}
            aria-pressed={filter === value} onClick={() => setFilter(value)}>{ui(label)}</Button>)}
        </div>
        {error && <p role="alert" className="text-sm text-red-300">{error}</p>}
        <p className="text-xs text-muted-foreground">{ui('{0} mechanics, {1} custom', { 0: entries.length, 1: entries.filter(item => item.custom).length })}</p>
        <ul className="max-h-[60dvh] space-y-1 overflow-y-auto pr-1" data-testid="mechanic-list">
          {entries.map(item => <li key={item.id}>
            <button type="button" onClick={() => setSelected(item.id)} aria-current={selected === item.id || undefined}
              className={cn('flex w-full items-center justify-between gap-2 border border-transparent px-3 py-2 text-left text-sm hover:border-white/20',
                selected === item.id && 'border-amber-200/40 bg-white/5')}>
              <span className="min-w-0 truncate capitalize">{item.name}</span>
              <MechanicBadge entry={item} />
            </button>
          </li>)}
          {!entries.length && <li className="px-3 py-2 text-sm text-muted-foreground">{ui('No mechanics match.')}</li>}
        </ul>
      </section>
      <section className="min-w-0">
        {entry
          ? <MechanicDetail entry={entry} onDeleted={() => setSelected('new')} />
          : <MechanicForm saved={saved} onSaved={def => { setSelected(`custom:${def.keyword}`); setFilter('all'); }} />}
      </section>
    </div>
  </div>;
}
