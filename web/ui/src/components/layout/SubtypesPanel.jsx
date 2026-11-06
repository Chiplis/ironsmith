import BuildCatalogList from './BuildCatalogList';
import useUiText from '@/i18n/useUiText';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { useGame } from '@/context/GameContext';
import { SUBTYPE_FAMILIES, listCustomSubtypes, mergeSubtypes, storeCustomSubtypes, suggestSubtypePlural, validateSubtype } from '@/lib/custom-subtypes';

export default function SubtypesPanel({ definitions: scopedDefinitions, onChange, setName, rows }) {
  const { game } = useGame();
  const ui = useUiText();
  const [personalDefinitions, setDefinitions] = useState(() => { try { return listCustomSubtypes(); } catch { return []; } });
  const definitions = scopedDefinitions ?? personalDefinitions;
  const persist = next => onChange ? onChange(next) : setDefinitions(storeCustomSubtypes(next));
  const [singular, setSingular] = useState('');
  const [plural, setPlural] = useState('');
  const [family, setFamily] = useState('Creature');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  async function save(event) {
    event.preventDefault(); setError(''); setBusy(true);
    try {
      const def = validateSubtype({ singular, plural, family });
      const next = mergeSubtypes(definitions, [def]);
      const cardType = family === 'Spell' ? 'Sorcery' : family === 'Planar' ? 'Plane' : family;
      const preview = await game.previewCustomCard({ customSubtypes: next.filter(item => item.singular === def.singular), layout: 'single', faces: [{
        name: 'Subtype validation', cardTypes: [cardType], subtypes: [def.singular], oracleText: '',
        ...(family === 'Creature' ? { power: '1', toughness: '1' } : {}),
        ...(family === 'Planeswalker' ? { loyalty: 1 } : {}),
        ...(family === 'Battle' ? { defense: 1 } : {}),
      }] });
      if (!preview?.canCreate) throw new Error('The compiler did not accept this subtype.');
      await persist(next); setSingular(''); setPlural('');
    } catch (cause) { setError(String(cause?.message || cause)); }
    finally { setBusy(false); }
  }
  async function remove(def) {
    try { await persist(definitions.filter(item => item.singular !== def.singular || item.family !== def.family)); }
    catch (cause) { setError(String(cause?.message || cause)); }
  }
  return <div className="space-y-5 p-5">
    {setName != null && <h3 className="font-semibold">{setName || ui('Untitled set')} · {ui('Set subtypes')}</h3>}
    {onChange && <p className="text-sm text-muted-foreground">{ui('Subtypes created here belong to this set. Save the set to keep your changes.')}</p>}
    <p className="text-sm text-muted-foreground">{ui('Define subtype names for your cards. Only types referenced by cards in a game join that game’s type choices, changeling, and “every type” effects. Saved cards carry their definitions.')}</p>
    <form onSubmit={save} className="flex flex-wrap items-end gap-3">
      <label className="grid gap-1 text-sm">{ui('Singular')}<input disabled={busy} required maxLength={32} className="rounded border p-2" value={singular} onChange={event => { setSingular(event.target.value); setPlural(suggestSubtypePlural(event.target.value)); }} placeholder={ui('Moonkin')} /></label>
      <label className="grid gap-1 text-sm">{ui('Plural')}<input disabled={busy} required maxLength={32} className="rounded border p-2" value={plural} onChange={event => setPlural(event.target.value)} placeholder={ui('Moonkin')} /></label>
      <label className="grid gap-1 text-sm">{ui('Family')}<select disabled={busy} className="rounded border p-2" value={family} onChange={event => setFamily(event.target.value)}>{SUBTYPE_FAMILIES.map(value => <option className="bg-background" key={value} value={value}>{ui(value)}</option>)}</select></label>
      <Button disabled={busy || !game} type="submit">{ui(busy ? 'Checking…' : 'Add subtype')}</Button>
    </form>
    <p className="text-xs text-muted-foreground">{ui('Names may contain words, letters, numbers, hyphens, or apostrophes. Check the suggested plural; irregular and identical forms are supported. Subtypes do not grant inherent abilities.')}</p>
    {error && <p role="alert" className="text-sm text-red-300">{error}</p>}
    <BuildCatalogList label={ui('Search subtypes')} rows={rows || definitions.map((definition, index) => ({ name: definition.singular, index, definition, detail: `${definition.plural} · ${definition.family}` }))} actions={row => <Button disabled={busy} variant="outline" size="sm" aria-label={`${ui('Remove')} ${row.name}`} onClick={() => remove(row.definition)}>{ui('Remove')}</Button>} />
  </div>;
}
