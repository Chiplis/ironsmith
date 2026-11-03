import { useEffect, useMemo, useState } from 'react';
import { loadCommunityLibrary } from '@/lib/community-library';
import { resolveBundledSet } from '@/lib/bundled-custom-sets';
import { CUSTOM_CARD_LIBRARY_CHANGED, CUSTOM_MECHANICS_CHANGED, listSavedCustomCards, listCustomMechanics } from '@/lib/custom-card-library';
import { CUSTOM_SUBTYPES_CHANGED, listCustomSubtypes } from '@/lib/custom-subtypes';
import { setCatalogRows } from '@/lib/build-catalog';
import useUiText from '@/i18n/useUiText';
import BuildCatalogList from './BuildCatalogList';
import MechanicsPanel from './MechanicsPanel';
import SubtypesPanel from './SubtypesPanel';

export default function BuildLibrary({ currentSet }) {
  const ui = useUiText();
  const [sources, setSources] = useState([]);
  const [kind, setKind] = useState('cards');
  const [scope, setScope] = useState('');
  const [loading, setLoading] = useState(true);
  const [errors, setErrors] = useState([]);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const refresh = () => { setLoading(true); setErrors([]); setRevision(value => value + 1); };
    const events = [CUSTOM_CARD_LIBRARY_CHANGED, CUSTOM_MECHANICS_CHANGED, CUSTOM_SUBTYPES_CHANGED];
    events.forEach(event => window.addEventListener(event, refresh));
    return () => events.forEach(event => window.removeEventListener(event, refresh));
  }, []);
  useEffect(() => {
    let active = true;
    const report = error => { if (active) setErrors(previous => [...previous, error.message]); };
    async function load() {
      const next = [];
      const results = await Promise.allSettled([
        loadCommunityLibrary(),
        Promise.all([listSavedCustomCards(), listCustomMechanics()]).then(([cards, customMechanics]) => ({ name: 'Personal library', customCards: cards, customMechanics, customSubtypes: listCustomSubtypes() })),
      ]);
      if (!active) return;
      if (results[1].status === 'fulfilled') next.push(results[1].value);
      else report(results[1].reason);
      if (results[0].status === 'fulfilled') {
        const queue = [...results[0].value.sets];
        // Limit asset fetches; large bundled collections must not flood the browser.
        await Promise.all(Array.from({ length: 4 }, async () => {
          while (active && queue.length) {
            const entry = queue.shift();
            try {
              const set = await resolveBundledSet(entry);
              next.push({ ...set, customCards: set.cards });
            } catch (error) { report(new Error(`${entry.name}: ${error.message}`)); }
          }
        }));
      } else report(results[0].reason);
      if (active) { setSources(next); setLoading(false); }
    }
    void load();
    return () => { active = false; };
  }, [revision]);
  const catalog = useMemo(() => {
    const values = currentSet.name.trim()
      ? [...sources.filter(source => source.name !== currentSet.name), currentSet]
      : sources;
    return values.map(source => ({ name: source.name, rows: setCatalogRows(source) })).sort((a, b) => a.name.localeCompare(b.name));
  }, [sources, currentSet]);
  const rows = catalog.filter(source => !scope || source.name === scope).flatMap(source => source.rows[kind]);
  return <div className="mx-auto grid w-full max-w-6xl gap-5 p-4 sm:p-6">
    <header className="flex flex-wrap items-start justify-between gap-3"><div><h2 className="text-xl font-semibold">{ui('Library')}</h2><p className="mt-1 text-sm text-muted-foreground">{ui('Browse cards, custom mechanics, and custom subtypes across your sets and personal library. Expand an entry to see its rules and usage. The current set includes unsaved changes.')}</p></div><button className="stone-pill px-3 py-2" disabled={loading} onClick={() => { setLoading(true); setErrors([]); setRevision(value => value + 1); }}>{ui('Refresh library')}</button></header>
    <div className="flex flex-wrap items-end gap-4">
      <div className="flex flex-wrap gap-2" role="group" aria-label={ui('Library contents')}>{[['cards', 'Cards'], ['mechanics', 'Mechanics'], ['subtypes', 'Subtypes']].map(([value, label]) => <button key={value} className={`stone-pill px-3 py-2 ${kind === value ? 'ui-primary-action' : ''}`} aria-pressed={kind === value} onClick={() => setKind(value)}>{ui(label)}</button>)}</div>
      <label className="grid min-w-0 gap-1 text-sm">{ui('Source set')}<select aria-label={ui('Source set')} className="fantasy-field max-w-full px-3 py-2" value={scope} onChange={event => setScope(event.target.value)}><option value="">{ui('All sets and personal library')}</option>{catalog.map(source => <option key={source.name} value={source.name}>{source.name}</option>)}</select></label>
    </div>
    {loading && <p role="status">{ui('Loading set contents…')}</p>}
    {errors.length > 0 && <div role="alert" className="text-sm text-red-300">{ui('Some sources could not be loaded. Refresh to retry.')}<details><summary>{ui('Details')}</summary>{errors.map((error, index) => <p key={index}>{error}</p>)}</details></div>}
    <BuildCatalogList key={kind} rows={rows} showSource label="Search library" />
    {kind !== 'cards' && <details className="fantasy-sheet-section p-4"><summary className="cursor-pointer font-semibold">{ui(kind === 'mechanics' ? 'Official mechanics & personal mechanic editor' : 'Personal subtype editor')}</summary>{kind === 'mechanics' ? <MechanicsPanel /> : <SubtypesPanel />}</details>}
  </div>;
}
