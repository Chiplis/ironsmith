import { useEffect, useMemo, useState } from 'react';
import useUiText from '@/i18n/useUiText';
import { CUSTOM_MECHANICS_CHANGED, deleteCustomMechanic, listCustomMechanics, saveCustomMechanic } from '@/lib/custom-card-library';
import { mechanicFormClash, validateMechanic } from '@/lib/custom-mechanics';
import { FORM_FIELDS, customMechanicEntry, defaultForms, mechanicEntries } from '@/lib/mechanics-catalog';
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

function MechanicForm({ saved, onSaved }) {
  const ui = useUiText();
  const [draft, setDraft] = useState(emptyDraft);
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState('');
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
  const save = async () => {
    if (!definition || busy) return;
    setBusy(true);
    try { const def = await saveCustomMechanic(draftDefinition(draft)); setDraft(emptyDraft()); onSaved?.(def); }
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
    <label className="card-forge-field"><span className="card-forge-section-label">{ui('Reminder text')}</span>
      <textarea className="card-forge-input min-h-24" value={draft.reminder} onChange={set('reminder')}
        placeholder={ui('As an additional cost to cast this spell, you may pay its blink cost.')} />
      <span className="text-xs text-muted-foreground">{ui('Write the full meaning without parentheses, as it would appear in the reminder.')}</span>
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
    <Button type="submit" disabled={!definition || busy}>{ui(busy ? 'Saving…' : 'Save mechanic')}</Button>
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
