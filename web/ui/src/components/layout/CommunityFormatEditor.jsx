import { loadCommunityLibrary, saveCommunityItem } from '@/lib/community-library';
import { COMMUNITY_FILE_MAX_BYTES, encodeCommunityShare, decodeCommunityShare, communityFilename } from '@/lib/community-sharing';
import { useEffect, useRef, useState } from 'react';
import BuildCardSearch from './BuildCardSearch';
import { fetchCardBuildDraft } from '@/lib/scryfall';
import { emptyCommunityFormat, normalizeCommunityFormat } from '@/lib/community-format';
import { resolveCommunityCard, resolveCommunitySet } from '@/lib/community-catalog';
import useUiText from '@/i18n/useUiText';

const field = 'fantasy-field w-full px-3 py-2';
const button = 'stone-pill px-3 py-2 text-sm disabled:opacity-50';
const cardName = card => card.draft.faces.map(face => face.name).join(' // ');

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

export default function CommunityFormatEditor({ value, onChange, kind = 'format', active = true }) {
  const ui = useUiText();
  const isSet = kind === 'set';
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
  const entry = () => isSet ? { name: value.name.trim(), cards: structuredClone(value.customCards) } : normalizeCommunityFormat(value);
  const load = item => onChange(isSet ? { ...emptyCommunityFormat(), name: item.name, customCards: structuredClone(item.cards) } : structuredClone(item));
  const savedItems = isSet ? library.sets : library.formats;
  const save = () => run(async () => {
    await saveCommunityItem(kind, entry());
    setLibrary(await loadCommunityLibrary());
    setNotice(isSet ? 'Set saved. Add it to a format in the Formats tab.' : 'Format saved. Choose it from the Custom dropdown when creating a lobby.');
  });
  const download = () => run(async () => {
    const data = entry();
    const url = URL.createObjectURL(new Blob([encodeCommunityShare(kind, data)], { type: 'application/json' }));
    const link = document.createElement('a');
    link.href = url; link.download = communityFilename(kind, data.name);
    document.body.append(link); link.click(); link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    setNotice('Exported with custom card and mechanic definitions.');
  });
  const importFile = file => run(async () => {
    if (file.size > COMMUNITY_FILE_MAX_BYTES) throw new Error('Community files must be no larger than 4 MB.');
    const imported = decodeCommunityShare(await file.text());
    const saved = await saveCommunityItem(imported.kind, imported.data, { keepExisting: true });
    setLibrary(await loadCommunityLibrary());
    if (imported.kind === kind) load(saved);
    setNotice(`Imported ${saved.name}. Existing saved items were kept.`);
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
        <p className="mt-1 text-sm text-muted-foreground">{ui(isSet ? 'Collect your custom cards into reusable sets. Save a set, then include it in any custom format.' : 'Define the card pool for your next game. Combine official cards and custom sets, then save a format to use in Play.')}</p>
      </div>
      <div className="flex flex-wrap gap-2">
        <button type="button" className={button} disabled={busy} onClick={() => { onChange(emptyCommunityFormat()); setNotice(''); setError(''); }}>{ui(isSet ? 'New set' : 'New format')}</button>
        <button type="button" className={button} disabled={busy || !libraryReady} onClick={() => importInput.current?.click()}>{ui('Import set or format')}</button>
        <button type="button" className={button} disabled={busy || !value.name.trim() || (isSet && !value.customCards.length)} onClick={download}>{ui(isSet ? 'Export custom set' : 'Export format')}</button>
        <input ref={importInput} type="file" accept=".json,application/json" className="sr-only" aria-label={ui('Import Community file')} disabled={busy || !libraryReady} onChange={event => {
          const file = event.target.files?.[0]; event.target.value = ''; if (file) void importFile(file);
        }} />
      </div>
    </header>
    <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,3fr)_minmax(280px,2fr)]">
      <div className="grid min-w-0 gap-4">
        <section className="fantasy-sheet-section grid gap-4 p-4 sm:grid-cols-2">
          <label className="grid gap-2 text-sm">{ui(isSet ? 'Custom set name' : 'Community format name')}<input className={field} maxLength={80} placeholder={ui(isSet ? 'Name your set' : 'Name your format')} value={value.name} disabled={busy} onChange={event => onChange({ ...value, name: event.target.value })} /></label>
          <label className="grid gap-2 text-sm">{ui(isSet ? 'Saved sets' : 'Saved formats')}<select className={field} value="" disabled={busy || !libraryReady} onChange={event => load(savedItems[Number(event.target.value)])}>
            <option value="">{ui('Load a saved item…')}</option>{savedItems.map((item, index) => <option key={item.name} value={index}>{item.name}</option>)}
          </select></label>
        </section>
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
            const set = library.sets[Number(event.target.value)];
            onChange({ ...value, customSets: [...value.customSets.filter(entry => entry.name !== set.name), structuredClone(set)] });
          }}><option value="">{ui('Add a saved custom set…')}</option>{library.sets.map((set, index) => <option key={set.name} value={index}>{set.name}</option>)}</select></label>
          {libraryReady && !library.sets.length && <p className="text-xs text-muted-foreground">{ui('Create your first custom set in the Sets tab.')}</p>}
        </section>}
        <section className="fantasy-sheet-section grid gap-3 overflow-hidden p-4">
          <div className="flex items-center justify-between gap-2"><h3 className="font-semibold">{ui(isSet ? 'Add cards to this set' : 'Add individual cards')}</h3><span className="text-sm text-muted-foreground">{value.customCards.length + (isSet ? 0 : value.includedCards.length)} {ui('selected')}</span></div>
          {active && <BuildCardSearch defaultIncludeCustom disabled={busy} onSelect={option => run(async () => {
            if (!isSet && !option.card) {
              onChange({ ...value, includedCards: [...new Set([...value.includedCards, option.name])] });
              return;
            }
            const card = option.card || { id: `catalog:${option.name}`, draft: await fetchCardBuildDraft(option.name) };
            onChange({ ...value, customCards: [...value.customCards.filter(entry => entry.id !== card.id), structuredClone(card)] });
          })} />}
          <p className="text-xs text-muted-foreground">{ui('Search for a card and select a result to add it. Enable custom cards to include your saved designs.')}</p>
        </section>
      </div>
      <aside className="grid min-w-0 gap-4 lg:sticky lg:top-0">
        {isSet ? <section className="fantasy-sheet-section grid gap-3 p-4"><h3 className="font-semibold">{value.name || ui('Untitled set')}</h3>
          <p className="text-sm text-muted-foreground">{ui('Selected cards are saved with their custom mechanic definitions.')}</p>
          {!value.customCards.length && <p className="text-sm">{ui('Select cards to start this set.')}</p>}
          <ul className="grid max-h-96 gap-2 overflow-auto">{value.customCards.map((card, index) => <li key={index} className="flex items-center justify-between gap-2 text-sm"><span>{cardName(card)}</span><button type="button" className={button} disabled={busy} aria-label={`${ui('Remove')} ${cardName(card)}`} onClick={() => onChange({ ...value, customCards: value.customCards.filter((_, i) => i !== index) })}>{ui('Remove')}</button></li>)}</ul>
        </section> : <CommunityFormatSummary format={value} onRemove={busy ? undefined : (key, index) => onChange({ ...value, [key]: value[key].filter((_, i) => i !== index) })} />}
        <button type="button" className={`${button} ui-primary-action`} disabled={busy || !libraryReady || !value.name.trim() || (isSet && !value.customCards.length)} onClick={save}>{ui(isSet ? 'Save set' : 'Save format')}</button>
        <p className="text-xs text-muted-foreground">{ui('Saved in this browser. Export a file to share or keep a backup. Saving an existing name replaces that item.')}</p>
        {notice && <p role="status" className="text-sm">{ui(notice)}</p>}
        {error && <p role="alert" className="text-sm text-red-300">{error}</p>}
      </aside>
    </div>
  </div>;
}
