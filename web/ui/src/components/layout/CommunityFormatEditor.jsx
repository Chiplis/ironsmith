import { loadCommunityLibrary, saveCommunityItem } from '@/lib/community-library';
import { COMMUNITY_FILE_MAX_BYTES, encodeCommunityShare, decodeCommunityShare, communityFilename } from '@/lib/community-sharing';
import { useEffect, useRef, useState } from 'react';
import { listSavedCustomCards, loadBundledCustomCards } from '@/lib/custom-card-library';
import { normalizeCommunityFormat } from '@/lib/community-format';
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
    {groups.map(([key, label, display]) => <div key={key}>
      <h4 className="text-sm font-semibold">{ui(label)}</h4>
      {format[key]?.length ? <ul className="grid gap-1 text-sm">{format[key].map((entry, index) => <li key={index} className="flex items-center justify-between gap-2">
        {key === 'customSets' ? <details><summary>{display(entry)}</summary><ul>{entry.cards.map((card, i) => <li key={i}>{cardName(card)}</li>)}</ul></details> : <span>{display(entry)}</span>}{onRemove && <button type="button" className={button} aria-label={`${ui('Remove')} ${display(entry)}`} onClick={() => onRemove(key, index)}>{ui('Remove')}</button>}
      </li>)}</ul> : <p className="text-sm text-muted-foreground">{ui('None')}</p>}
    </div>)}
  </section>;
}

export default function CommunityFormatEditor({ value, onChange, onBusyChange }) {
  const ui = useUiText();
  const [library, setLibrary] = useState({ formats: [], sets: [] });
  const [libraryReady, setLibraryReady] = useState(false);
  const [notice, setNotice] = useState('');
  const [exportSet, setExportSet] = useState('');
  const importInput = useRef(null);
  const [cards, setCards] = useState([]);
  const [query, setQuery] = useState('');
  const [source, setSource] = useState('includedSets');
  const [official, setOfficial] = useState('');
  const [setName, setSetName] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    loadCommunityLibrary().then(next => { if (active) { setLibrary(next); setLibraryReady(true); } })
      .catch(failure => { if (active) setError(failure.message); });
    Promise.allSettled([listSavedCustomCards(), loadBundledCustomCards()]).then(results => {
      if (!active) return;
      setCards(results.flatMap(result => result.status === 'fulfilled' ? result.value : []));
      const failures = results.filter(result => result.status === 'rejected');
      if (failures.length) setError(failures.map(result => result.reason.message).join(' '));
    });
    return () => { active = false; };
  }, []);
  const run = async action => {
    setBusy(true); onBusyChange(true); setError(''); setNotice('');
    try { await action(); }
    catch (failure) { setError(failure.message); }
    finally { setBusy(false); onBusyChange(false); }
  };
  const save = async (kind, entry, options) => {
    const saved = await saveCommunityItem(kind, entry, options);
    setLibrary(await loadCommunityLibrary());
    setExportSet('');
    return saved;
  };
  const download = (kind, entry) => {
    try {
      const text = encodeCommunityShare(kind, entry);
      const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
      const link = document.createElement('a');
      link.href = url; link.download = communityFilename(kind, entry.name);
      document.body.append(link); link.click(); link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      setError(''); setNotice('Exported with custom card and mechanic definitions.');
    } catch (failure) { setError(failure.message); }
  };
  const importFile = file => run(async () => {
    if (file.size > COMMUNITY_FILE_MAX_BYTES) throw new Error('Community files must be no larger than 4 MB.');
    const imported = decodeCommunityShare(await file.text());
    const saved = await save(imported.kind, imported.data, { keepExisting: true });
    if (imported.kind === 'format') onChange(saved);
    else {
      // Keep the current draft's same-named set too, even if it was unsaved.
      const names = new Set(value.customSets.map(set => set.name));
      let name = saved.name, suffix = 2;
      while (names.has(name)) name = `${saved.name} (${suffix++})`;
      onChange({ ...value, customSets: [...value.customSets, { ...saved, name }] });
    }
    setNotice(`Imported ${saved.name}. Existing saved items were kept.`);
  });
  const addOfficial = async () => {
    setBusy(true); onBusyChange(true); setError('');
    try {
      const entry = source.endsWith('Sets') ? await resolveCommunitySet(official) : await resolveCommunityCard(official);
      const identity = item => typeof item === 'string' ? item : item.code;
      onChange({ ...value, [source]: [...value[source].filter(item => identity(item) !== identity(entry)), entry] });
      setOfficial('');
    } catch (failure) { setError(failure.message); }
    finally { setBusy(false); onBusyChange(false); }
  };
  return <div className="community-format-editor grid w-full min-h-0 content-start gap-4 overflow-auto p-1">
    <p className="text-sm text-muted-foreground">{ui('Build a format from official sets, individual cards, and cards from your custom library. Create a room, then share its invitation link.')}</p>
    <div className="grid gap-2">
      <p className="text-sm text-muted-foreground">{ui('Share JSON files with all card and custom mechanic definitions included. Imports are saved in this browser; matching names are kept as separate copies.')}</p>
      <div className="flex flex-wrap gap-2">
        <input ref={importInput} type="file" accept=".json,application/json" className="sr-only" aria-label={ui('Import Community file')} disabled={busy || !libraryReady} onChange={event => {
          const file = event.target.files?.[0]; event.target.value = ''; if (file) void importFile(file);
        }} />
        <button type="button" className={button} disabled={busy || !libraryReady} onClick={() => importInput.current?.click()}>{ui('Import set or format')}</button>
        <button type="button" className={button} disabled={busy || !value.name.trim()} onClick={() => download('format', value)}>{ui('Export format')}</button>
      </div>
      <label>{ui('Custom set to export')}<select className={field} disabled={busy} value={exportSet} onChange={event => setExportSet(event.target.value)}>
        <option value="">{ui('Choose a custom set…')}</option>
        {library.sets.map((set, index) => <option key={`saved-${index}`} value={`saved:${index}`}>{set.name} — {ui('Saved')}</option>)}
        {value.customSets.map((set, index) => <option key={`current-${index}`} value={`current:${index}`}>{set.name} — {ui('Current format')}</option>)}
      </select></label>
      <button type="button" className={button} disabled={busy || !exportSet} onClick={() => {
        const [source, index] = exportSet.split(':'); download('set', (source === 'saved' ? library.sets : value.customSets)[Number(index)]);
      }}>{ui('Export custom set')}</button>
    </div>
    <label>{ui('Saved formats')}<select className={field} value="" disabled={busy || !libraryReady} onChange={event => { setExportSet(''); onChange(structuredClone(library.formats[Number(event.target.value)])); }}>
      <option value="">{ui('Load a saved format…')}</option>{library.formats.map((format, index) => <option key={index} value={index}>{format.name}</option>)}
    </select></label>
    <label>{ui('Community format name')}<input className={field} maxLength={80} value={value.name} disabled={busy} onChange={event => onChange({ ...value, name: event.target.value })} /></label>
    <div className="flex flex-wrap gap-2">
      <select className={field} aria-label={ui('Official card source')} value={source} disabled={busy} onChange={event => setSource(event.target.value)}>
        <option value="includedSets">{ui('Include official set')}</option><option value="excludedSets">{ui('Exclude official set')}</option>
        <option value="includedCards">{ui('Include official card')}</option><option value="excludedCards">{ui('Exclude official card')}</option>
      </select>
      <input className={field} aria-label={ui('Set code or card name')} placeholder={source.endsWith('Sets') ? ui('Set code, e.g. dom') : ui('Exact card name')} value={official} disabled={busy} onChange={event => setOfficial(event.target.value)} />
      <button type="button" className={button} disabled={busy || !official.trim()} onClick={addOfficial}>{ui(busy ? 'Loading…' : 'Add')}</button>
    </div>
    <label>{ui('Find custom cards')}<input className={field} value={query} disabled={busy} onChange={event => setQuery(event.target.value)} /></label>
    <div className="max-h-48 overflow-auto grid gap-2">
      {cards.filter(card => cardName(card).toLowerCase().includes(query.toLowerCase())).slice(0, 40).map((card, index) => <label key={`${card.id}-${index}`} className="flex gap-2 text-sm">
        <input type="checkbox" disabled={busy} checked={value.customCards.some(entry => entry.id === card.id)} onChange={event => onChange({ ...value, customCards: event.target.checked ? [...value.customCards, card] : value.customCards.filter(entry => entry.id !== card.id) })} />{cardName(card)}
      </label>)}
      {!cards.length && <p>{ui('Save cards in the card creator to add your own designs here.')}</p>}
    </div>
    <div className="flex flex-wrap gap-2">
      <input className={field} aria-label={ui('Custom set name')} maxLength={80} placeholder={ui('Custom set name')} value={setName} disabled={busy} onChange={event => setSetName(event.target.value)} />
      <button type="button" className={button} disabled={busy || !libraryReady || !setName.trim() || !value.customCards.length} onClick={() => run(async () => {
        const set = { name: setName.trim(), cards: structuredClone(value.customCards) };
        await save('set', set);
        onChange({ ...value, customCards: [], customSets: [...value.customSets.filter(entry => entry.name !== set.name), set] });
      })}>{ui('Save selected cards as a set')}</button>
      <select className={field} aria-label={ui('Add custom set')} disabled={busy} value="" onChange={event => {
        const set = library.sets[Number(event.target.value)];
        onChange({ ...value, customSets: [...value.customSets.filter(entry => entry.name !== set.name), structuredClone(set)] });
      }}><option value="">{ui('Add a saved custom set…')}</option>{library.sets.map((set, index) => <option key={index} value={index}>{set.name}</option>)}</select>
    </div>
    <CommunityFormatSummary format={value} onRemove={busy ? undefined : (key, index) => { setExportSet(''); onChange({ ...value, [key]: value[key].filter((_, i) => i !== index) }); }} />
    <button type="button" className={button} disabled={busy || !libraryReady || !value.name.trim()} onClick={() => run(async () => {
      await save('format', normalizeCommunityFormat(value)); setNotice('Format saved in this browser.');
    })}>{ui('Save format')}</button>
    {notice && <p role="status" className="text-sm">{ui(notice)}</p>}
    {error && <p role="alert" className="text-sm text-red-300">{error}</p>}
  </div>;
}
