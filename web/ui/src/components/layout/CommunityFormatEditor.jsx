import BuildCatalogList from './BuildCatalogList';
import { buildCardName, setCatalogRows } from '@/lib/build-catalog';
import { resolveBundledSet } from '@/lib/bundled-custom-sets';
import SubtypesPanel from './SubtypesPanel';
import { MechanicForm } from './MechanicsPanel';
import { setMechanics, setSubtypes, updateSetMechanic } from '@/lib/set-workspace';
import { loadCommunityLibrary, saveCommunityItem } from '@/lib/community-library';
import { COMMUNITY_FILE_MAX_BYTES, encodeCommunityShare, decodeCommunityShare, communityFilename } from '@/lib/community-sharing';
import { useEffect, useMemo, useRef, useState } from 'react';
import BuildCardSearch from './BuildCardSearch';
import { fetchCardBuildDraft } from '@/lib/scryfall';
import { emptyCommunityFormat, normalizeCommunityFormat } from '@/lib/community-format';
import { resolveCommunityCard, resolveCommunitySet } from '@/lib/community-catalog';
import useUiText from '@/i18n/useUiText';

const field = 'fantasy-field w-full px-3 py-2';
const button = 'stone-pill px-3 py-2 text-sm disabled:opacity-50';
const cardName = buildCardName;

export function CommunityFormatSummary({ format, onRemove }) {
  const ui = useUiText();
  if (!format) return null;
  const groups = [
    ['includedSets', 'Included official sets', set => `${set.name} (${set.code.toUpperCase()}) · ${set.cards.length} cards`],
    ['excludedSets', 'Excluded official sets', set => `${set.name} (${set.code.toUpperCase()})`],
    ['includedCards', 'Included official individual cards', name => name],
    ['excludedCards', 'Excluded individual cards', name => name],
    ['customSets', 'Custom sets', set => `${set.name} · ${set.cards.length} cards`],
    ['customCards', 'Custom individual cards', cardName],
  ];
  return <section className="fantasy-sheet-section grid gap-3 p-4" aria-label={ui('Community format card pool')}>
    <h3 className="font-bold">{format.name || ui('Community format')}</h3>
    <p className="text-sm text-muted-foreground">{ui('Only included cards may be played. Exclusions always win, even when a whole set includes the card. Normal constructed deck rules apply: at least 60 cards, with up to four copies except basic lands and cards that say otherwise.')}</p>
    {!groups.some(([key]) => format[key]?.length) && <p className="text-sm text-muted-foreground">{ui('Add sets or cards to define your card pool.')}</p>}
    {groups.filter(([key]) => format[key]?.length).map(([key, label, display]) => <div key={key}>
      <h4 className="text-sm font-semibold">{ui(label)}</h4>
      {format[key]?.length ? <ul className="grid gap-1 text-sm">{format[key].map((entry, index) => <li key={index} className="flex items-center justify-between gap-2">
        {key === 'customSets' ? <details><summary>{display(entry)}</summary><ul>{entry.cards.map((card, i) => <li key={i}>{cardName(card)}</li>)}</ul></details> : <span>{display(entry)}</span>}{onRemove && <button type="button" className={button} aria-label={`${ui('Remove')} ${display(entry)}`} onClick={() => onRemove(key, index)}>{ui('Remove')}</button>}
      </li>)}</ul> : <p className="text-sm text-muted-foreground">{ui('None')}</p>}
    </div>)}
  </section>;
}

export default function CommunityFormatEditor({ value, onChange, kind = 'format', active = true, onEditCard, initialView = 'cards', initialMechanic = null, onBackToCard }) {
  const ui = useUiText();
  const isSet = kind === 'set';
  const contents = useMemo(() => setCatalogRows(value), [value]);
  const mechanics = contents.mechanics.map(row => row.definition);
  const subtypes = contents.subtypes.map(row => row.definition);
  const [setView, setSetView] = useState(initialView);
  const [editingMechanic, setEditingMechanic] = useState(() => isSet && initialMechanic ? mechanics.find(def => def.keyword === initialMechanic) || null : null);
  const [library, setLibrary] = useState({ formats: [], sets: [] });
  const [libraryReady, setLibraryReady] = useState(false);
  const [source, setSource] = useState('includedSets');
  const [official, setOfficial] = useState('');
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const importInput = useRef(null);
  useEffect(() => {
    if (!active) return;
    let live = true;
    loadCommunityLibrary().then(next => { if (live) { setLibrary(next); setLibraryReady(true); } })
      .catch(failure => { if (live) setError(failure.message); });
    return () => { live = false; };
  }, [active, kind]);
  const run = async action => {
    setBusy(true); setError(''); setNotice('');
    try { await action(); } catch (failure) { setError(failure.message); }
    finally { setBusy(false); }
  };
  const entry = () => isSet ? { ...(value.source ? {source:structuredClone(value.source)} : {}), ...(value.code ? {code:value.code} : {}), name: value.name.trim(), cards: structuredClone(value.customCards), customMechanics: value.customMechanics || [], customSubtypes: subtypes } : normalizeCommunityFormat(value);
  const load = async item => { item = await resolveBundledSet(item); setEditingMechanic(null); setSetView('cards'); onChange(isSet ? { ...emptyCommunityFormat(), ...(item.source ? {source:structuredClone(item.source)} : {}), ...(item.code ? {code:item.code} : {}), name: item.name, customCards: structuredClone(item.cards), customMechanics: structuredClone(item.customMechanics || []), customSubtypes: structuredClone(item.customSubtypes || []) } : structuredClone(item)); };
  const savedItems = isSet ? library.sets : library.formats;
  const save = () => run(async () => {
    await saveCommunityItem(kind, entry());
    setLibrary(await loadCommunityLibrary());
    setNotice(ui(isSet ? 'Set saved. Add it to a format in the Formats tab.' : 'Format saved. Choose it from the Custom dropdown when creating a lobby.'));
  });
  const download = () => run(async () => {
    const data = entry();
    const url = URL.createObjectURL(new Blob([encodeCommunityShare(kind, data)], { type: 'application/json' }));
    const link = document.createElement('a');
    link.href = url; link.download = communityFilename(kind, data.name);
    document.body.append(link); link.click(); link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    setNotice(ui('Exported with custom card and mechanic definitions.'));
  });
  const importFile = file => run(async () => {
    if (file.size > COMMUNITY_FILE_MAX_BYTES) throw new Error(ui('Community files must be no larger than 4 MB.'));
    const imported = decodeCommunityShare(await file.text());
    const saved = await saveCommunityItem(imported.kind, imported.data, { keepExisting: true });
    setLibrary(await loadCommunityLibrary());
    if (imported.kind === kind) await load(saved);
    setNotice(ui('Imported {0}. Existing saved items were kept.', { 0: saved.name }));
  });
  const addOfficial = () => run(async () => {
    const item = source.endsWith('Sets') ? await resolveCommunitySet(official) : await resolveCommunityCard(official);
    const identity = item => typeof item === 'string' ? item : item.code;
    onChange({ ...value, [source]: [...value[source].filter(existing => identity(existing) !== identity(item)), item] });
    setOfficial('');
  });
  return <div className="community-format-editor mx-auto grid w-full max-w-7xl content-start gap-5 p-4 md:p-6">
    <header className="flex flex-wrap items-start justify-between gap-4">
      <div className="max-w-2xl"><h2 className="text-xl font-semibold">{ui(isSet ? 'Custom sets' : 'Custom formats')}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{ui(isSet ? 'Browse this set’s cards, mechanics, and subtypes below. Use Library to compare contents across sets.' : 'Define the card pool for your next game. Combine official cards and custom sets, then save a format to use in Play.')}</p>
      </div>
      <div className="flex flex-wrap gap-2">
        {onBackToCard && <button type="button" className={`${button} ui-primary-action`} disabled={busy} onClick={onBackToCard}>{ui('Back to card')}</button>}
        <button type="button" className={button} disabled={busy || Boolean(onBackToCard)} onClick={() => { onChange(emptyCommunityFormat()); setEditingMechanic(null); setSetView('cards'); setNotice(''); setError(''); }}>{ui(isSet ? 'New set' : 'New format')}</button>
        <button type="button" className={button} disabled={busy || !libraryReady || Boolean(onBackToCard)} onClick={() => importInput.current?.click()}>{ui('Import set or format')}</button>
        <button type="button" className={button} disabled={busy || !value.name.trim()} onClick={download}>{ui(isSet ? 'Export custom set' : 'Export format')}</button>
        <input ref={importInput} type="file" accept=".json,application/json" className="sr-only" aria-label={ui('Import Community file')} disabled={busy || !libraryReady} onChange={event => {
          const file = event.target.files?.[0]; event.target.value = ''; if (file) void importFile(file);
        }} />
      </div>
    </header>
        <section className="fantasy-sheet-section grid gap-4 p-4 sm:grid-cols-2">
          <label className="grid gap-2 text-sm">{ui(isSet ? 'Custom set name' : 'Community format name')}<input className={field} maxLength={80} placeholder={ui(isSet ? 'Name your set' : 'Name your format')} value={value.name} disabled={busy} onChange={event => onChange({ ...value, name: event.target.value })} /></label>
          <label className="grid gap-2 text-sm">{ui(isSet ? 'Saved sets' : 'Saved formats')}<select aria-label={ui(isSet ? 'Saved sets' : 'Saved formats')} className={field} value="" disabled={busy || !libraryReady || Boolean(onBackToCard)} onChange={event => { const item = savedItems[Number(event.target.value)]; void run(() => load(item)); }}>
            <option value="">{ui('Load a saved item…')}</option>{savedItems.map((item, index) => <option key={item.name} value={index}>{item.name}</option>)}
          </select></label>
        </section>
    {isSet && <div className="flex flex-wrap items-center gap-3 border-b border-white/15 pb-3">
      <button disabled={busy} className={`${button} ${setView === 'cards' ? 'ui-primary-action' : ''}`} aria-pressed={setView === 'cards'} onClick={() => onBackToCard ? onBackToCard() : setSetView('cards')}>{ui('Set cards')} · {value.customCards.length}</button>
      <button disabled={busy} className={`${button} ${setView === 'mechanics' ? 'ui-primary-action' : ''}`} aria-pressed={setView === 'mechanics'} onClick={() => setSetView('mechanics')}>{ui('Set mechanics')} · {mechanics.length}</button>
      <button disabled={busy} className={`${button} ${setView === 'subtypes' ? 'ui-primary-action' : ''}`} aria-pressed={setView === 'subtypes'} onClick={() => setSetView('subtypes')}>{ui('Set subtypes')} · {subtypes.length}</button>
      <span className="text-xs text-muted-foreground">{ui('One workspace for your cards, mechanics, and subtypes.')}</span>
    </div>}
    {isSet && setView === 'subtypes' ? <section className="fantasy-sheet-section p-4">
      <SubtypesPanel rows={contents.subtypes} setName={value.name} definitions={subtypes} onChange={definitions => {
        const removed = subtypes.filter(def => !definitions.some(next => next.singular === def.singular));
        if (removed.some(def => value.customCards.some(card => card.draft.customSubtypes?.some(item => item.singular === def.singular)))) throw new Error(ui('This subtype is used by a set card. Remove it from those cards first.'));
        onChange({ ...value, customSubtypes: definitions });
      }} />
    </section> : isSet && setView === 'mechanics' ? <div className="grid gap-6 lg:grid-cols-[280px_minmax(0,1fr)]">
      <section className="fantasy-sheet-section space-y-3 p-4">
        <h3 className="font-semibold">{value.name || ui('Untitled set')}</h3>
        <p className="text-sm text-muted-foreground">{ui('Updating a mechanic recompiles the cards using that definition. Other variants, sets and your personal library keep their own definitions.')}</p>
        <button className={button} disabled={busy} onClick={() => setEditingMechanic(null)}>{ui('New custom mechanic')}</button>
        <BuildCatalogList rows={contents.mechanics} label={ui('Search set mechanics')} actions={row => <button className={button} disabled={busy} aria-label={`${ui('Edit')} ${row.name}`} onClick={() => setEditingMechanic(row.definition)}>{ui('Edit')}</button>} />
      </section>
      <section className="fantasy-sheet-section p-5">
        <MechanicForm key={editingMechanic ? JSON.stringify(editingMechanic) : 'new'} initial={editingMechanic} saved={mechanics} onSave={async (definition, compile) => {
          if (!compile) throw new Error(ui('The card compiler must be ready to update set mechanics.'));
          setBusy(true);
          try {
            const next = await updateSetMechanic(value, definition, compile, editingMechanic);
            onChange(next);
            setNotice(ui('Mechanic applied to all set cards. Save set to keep your changes.'));
            return definition;
          } finally { setBusy(false); }
        }} onSaved={() => setEditingMechanic(null)} />
      </section>
    </div> : <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,3fr)_minmax(280px,2fr)]">
      <div className="grid min-w-0 gap-4">
        {!isSet && <section className="fantasy-sheet-section grid gap-3 p-4">
          <h3 className="font-semibold">{ui('Official card pool')}</h3>
          <div className="grid gap-2 sm:grid-cols-[1fr_1fr_auto]">
            <select className={field} aria-label={ui('Official card source')} value={source} disabled={busy} onChange={event => setSource(event.target.value)}>
              <option value="includedSets">{ui('Include official set')}</option><option value="excludedSets">{ui('Exclude official set')}</option>
              <option value="includedCards">{ui('Include official card')}</option><option value="excludedCards">{ui('Exclude official card')}</option>
            </select>
            <input className={field} aria-label={ui('Set code or card name')} placeholder={source.endsWith('Sets') ? ui('Set code, e.g. dom') : ui('Exact card name')} value={official} disabled={busy} onChange={event => setOfficial(event.target.value)} />
            <button type="button" className={button} disabled={busy || !official.trim()} onClick={addOfficial}>{ui(busy ? 'Loading…' : 'Add')}</button>
          </div>
          <p className="text-xs text-muted-foreground">{ui('Only included cards may be played. Exclusions always win.')}</p>
          <label className="grid gap-2 text-sm">{ui('Add custom set')}<select aria-label={ui('Add custom set')} className={field} disabled={busy || !libraryReady} value="" onChange={event => {
            const selected = library.sets[Number(event.target.value)];
            void run(async () => { const set = await resolveBundledSet(selected); onChange({ ...value, customSets: [...value.customSets.filter(entry => entry.name !== set.name), set] }); });
          }}><option value="">{ui('Add a saved custom set…')}</option>{library.sets.map((set, index) => <option key={set.name} value={index}>{set.name}</option>)}</select></label>
          {libraryReady && !library.sets.length && <p className="text-xs text-muted-foreground">{ui('Create your first custom set in the Sets tab.')}</p>}
        </section>}
        <section className="fantasy-sheet-section grid gap-3 overflow-hidden p-4">
          <div className="flex items-center justify-between gap-2"><h3 className="font-semibold">{ui(isSet ? 'Create in this set' : 'Add individual cards')}</h3><span className="text-sm text-muted-foreground">{value.customCards.length + (isSet ? 0 : value.includedCards.length)} {ui('selected')}</span></div>
          {isSet && onEditCard && <div className="grid gap-3 border-b border-white/10 pb-4">
            <p className="text-sm text-muted-foreground">{ui('Start a new design with this set’s mechanics already available, or edit any card in the list.')}</p>
            <button type="button" className={`${button} ui-primary-action justify-self-start`} disabled={busy || !value.name.trim()} onClick={() => onEditCard(null)}>{ui('Create card in set')}</button>
            {!value.name.trim() && <p className="text-xs text-muted-foreground">{ui('Name your set to start creating.')}</p>}
          </div>}
          {active && <BuildCardSearch defaultIncludeCustom disabled={busy} onSelect={option => run(async () => {
            if (!isSet && !option.card) {
              onChange({ ...value, includedCards: [...new Set([...value.includedCards, option.name])] });
              return;
            }
            const card = option.card || { id: `catalog:${option.name}`, draft: await fetchCardBuildDraft(option.name) };
            const next = { ...value, customCards: [...value.customCards.filter(entry => entry.id !== card.id), structuredClone(card)] };
            if (isSet) { setMechanics(next); setSubtypes(next); }
            onChange(next);
          })} />}
          <p className="text-xs text-muted-foreground">{ui('Search for a card and select a result to add it. Enable custom cards to include your saved designs.')}</p>
        </section>
      </div>
      <aside className="grid min-w-0 gap-4 lg:sticky lg:top-0">
        {isSet ? <section className="fantasy-sheet-section grid gap-3 p-4"><h3 className="font-semibold">{value.name || ui('Untitled set')}</h3>
          <p className="text-sm text-muted-foreground">{ui('Selected cards are saved with their custom mechanic definitions.')}</p>
          {!value.customCards.length && <p className="text-sm">{ui('Your first card starts here. Create a card or add an existing design.')}</p>}
          <BuildCatalogList rows={contents.cards} actions={row => <div className="flex gap-2">
            {onEditCard && <button className={button} disabled={busy || !value.name.trim()} aria-label={`${ui('Edit')} ${row.name}`} onClick={() => onEditCard(row.index)}>{ui('Edit')}</button>}
            <button className={button} disabled={busy} aria-label={`${ui('Remove')} ${row.name}`} onClick={() => onChange({ ...value, customCards: value.customCards.filter((_, index) => index !== row.index) })}>{ui('Remove')}</button>
          </div>} />
        </section> : <CommunityFormatSummary format={value} onRemove={busy ? undefined : (key, index) => onChange({ ...value, [key]: value[key].filter((_, i) => i !== index) })} />}
      </aside>
    </div>}
    <footer className="grid gap-2 border-t border-white/15 pt-4">
        <button type="button" className={`${button} ui-primary-action`} disabled={busy || !libraryReady || !value.name.trim()} onClick={save}>{ui(isSet ? 'Save set' : 'Save format')}</button>
        <p className="text-xs text-muted-foreground">{ui('Saved in this browser. Export a file to share or keep a backup. Saving an existing name replaces that item.')}</p>
        {notice && <p role="status" className="text-sm">{ui(notice)}</p>}
        {error && <p role="alert" className="text-sm text-red-300">{error}</p>}
    </footer>
  </div>;
}
