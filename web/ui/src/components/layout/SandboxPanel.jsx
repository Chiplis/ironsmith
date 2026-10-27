import { useEffect, useState } from 'react';
import { useGame } from '@/context/GameContext';
import useUiText from '@/i18n/useUiText';
import { CUSTOM_CARD_LIBRARY_CHANGED, listSavedCustomCards, prepareCustomDraftForCreation } from '@/lib/custom-card-library';
import { parseDeckList } from '@/lib/decklists';
import { normalizeDraftForApi } from '@/lib/custom-card-draft';
import { SANDBOX_RULES, sandboxerDraft, buildSandboxPlan } from '@/lib/sandbox';
import { readSandboxPreferences, saveSandboxPreferences } from '@/lib/sandbox-session';
import { setCompiledCardNames, setCustomCardArtUrls } from '@/lib/scryfall';
import { clearCardPresentations, setCardPresentation } from '@/lib/card-glint/printing-choices.mjs';
import { Button } from '@/components/ui/button';

export default function SandboxPanel({ currentDraft, onStarted, onBusyChange }) {
  const ui = useUiText();
  const { game, multiplayer, runWasmInteraction, refresh } = useGame();
  const [saved, setSaved] = useState([]);
  const [selections, setSelections] = useState([]);
  const [testedId, setTestedId] = useState('');
  const [preferences, setPreferences] = useState(readSandboxPreferences);
  const { options, bothPlayers } = preferences;
  useEffect(() => { saveSandboxPreferences(preferences); }, [preferences]);
  const [startingLife, setStartingLife] = useState(20);
  const [deck, setDeck] = useState('');
  const [opponentDeck, setOpponentDeck] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const locked = !game || multiplayer?.mode !== 'idle' || multiplayer?.matchStarted;
  useEffect(() => {
    let active = true;
    const load = async () => {
      try { const cards = await listSavedCustomCards(); if (active) setSaved(cards); }
      catch (error) { if (active) setError(String(error?.message || error)); }
    };
    void load();
    window.addEventListener(CUSTOM_CARD_LIBRARY_CHANGED, load);
    return () => { active = false; window.removeEventListener(CUSTOM_CARD_LIBRARY_CHANGED, load); };
  }, []);
  const add = card => {
    if (selections.some(selection => selection.id === card.id)) return;
    setSelections(current => [...current, { ...card, copies: 4 }]);
    if (!testedId) setTestedId(card.id);
  };
  const remove = id => {
    const remaining = selections.filter(card => card.id !== id);
    setSelections(remaining);
    if (testedId === id) setTestedId(remaining[0]?.id || '');
  };
  const rulesText = sandboxerDraft(options).faces[0].oracleText;
  const start = async () => {
    if (locked || busy) return;
    setError(''); setBusy(true); onBusyChange?.(true);
    try {
      const preparedSelections = [];
      for (const selection of selections) {
        const prepared = await prepareCustomDraftForCreation(selection.draft, input => game.previewCustomCard(normalizeDraftForApi(input)));
        preparedSelections.push({ ...selection, draft: prepared.draft });
      }
      const plan = buildSandboxPlan({ selections: preparedSelections, testedId, options, bothPlayers, startingLife,
        deck: parseDeckList(deck), opponentDeck: parseDeckList(opponentDeck) });
      await runWasmInteraction(async () => {
        const result = await game.startSandbox(plan);
        clearCardPresentations();
        const faces = result.created.flatMap(card => card.draft.faces);
        setCompiledCardNames(faces.map(face => face.name));
        setCustomCardArtUrls(faces.map(face => ({ name: face.name, artUrl: face.artUrl })));
        for (const card of result.created) {
          if (card.draft.presentation) setCardPresentation(card.id, card.draft.faces.map(face => face.name), card.draft.presentation, { builderCreated: true });
        }
        await refresh('Sandbox ready');
        onStarted?.();
      });
    } catch (error) { setError(String(error?.message || error)); }
    finally { setBusy(false); onBusyChange?.(false); }
  };
  return <div className="sandbox-panel overflow-y-auto p-4 sm:p-6" data-testid="sandbox-panel">
    <div className="mx-auto grid max-w-6xl gap-6 lg:grid-cols-2">
      <fieldset disabled={busy || locked} className="min-w-0 space-y-4">
        <legend className="text-lg font-semibold">{ui('Cards to test')}</legend>
        <p className="text-sm text-muted-foreground">{ui('Choose saved cards and their copy counts. The selected test card always starts in your opening hand.')}</p>
        <label className="card-forge-field">
          <span>{ui('Saved custom card')}</span>
          <select className="card-forge-input" value="" onChange={event => { const card = saved.find(card => card.id === event.target.value); if (card) add(card); }}>
            <option value="">{ui('Add a saved card…')}</option>
            {saved.map(card => <option key={card.id} value={card.id} disabled={selections.some(selection => selection.id === card.id)}>{card.draft.faces[0].name}</option>)}
          </select>
        </label>
        <Button variant="secondary" onClick={() => add({ id: 'current-draft', draft: structuredClone(currentDraft) })} disabled={!currentDraft || selections.some(card => card.id === 'current-draft')}>{ui('Use current card draft')}</Button>
        {!saved.length && <p className="text-sm text-muted-foreground">{ui('Save a card in the Card tab, or use your current draft.')}</p>}
        <div className="space-y-3">
          {selections.map(card => <div key={card.id} className="flex flex-wrap items-center gap-3 border border-white/15 p-3">
            <label className="flex min-w-0 flex-1 items-center gap-2">
              <input type="radio" name="sandbox-tested-card" checked={testedId === card.id} onChange={() => setTestedId(card.id)} />
              <span className="break-words">{card.draft.faces[0].name}</span>
            </label>
            <label className="flex items-center gap-2 text-sm">{ui('Copies')}
              <input aria-label={`${ui('Copies of')} ${card.draft.faces[0].name}`} className="card-forge-input w-20" type="number" min="1" max="1000" value={card.copies}
                onChange={event => setSelections(current => current.map(entry => entry.id === card.id ? { ...entry, copies: event.target.value } : entry))} />
            </label>
            <button type="button" className="text-sm underline" onClick={() => remove(card.id)} aria-label={`${ui('Remove')} ${card.draft.faces[0].name}`}>{ui('Remove')}</button>
          </div>)}
        </div>
        <label className="card-forge-field"><span>{ui('Starting life for both players')}</span>
          <input className="card-forge-input" type="number" min="1" max="1000000" value={startingLife} onChange={event => setStartingLife(event.target.value)} />
        </label>
        <details>
          <summary className="cursor-pointer text-sm font-semibold">{ui('Custom decks (optional)')}</summary>
          <p className="my-3 text-sm text-muted-foreground">{ui('Leave blank for basic lands in the test card’s colors and vanilla creatures. Selected custom copies are added to your deck; one is reserved for your opening hand.')}</p>
          <label className="card-forge-field"><span>{ui('Your base deck')}</span><textarea className="card-forge-input min-h-32" placeholder={'24 Forest\n32 Grizzly Bears'} value={deck} onChange={event => setDeck(event.target.value)} /></label>
          <label className="card-forge-field mt-3"><span>{ui('Opponent deck')}</span><textarea className="card-forge-input min-h-32" placeholder={ui('Leave blank for a practice deck')} value={opponentDeck} onChange={event => setOpponentDeck(event.target.value)} /></label>
        </details>
      </fieldset>
      <fieldset disabled={busy || locked} className="min-w-0 space-y-4">
        <legend className="text-lg font-semibold">IronSmith Sandboxer</legend>
        <p className="text-sm text-muted-foreground">{ui('Each enabled option adds one ability to this enchantment on the battlefield. Remove the enchantment to turn its abilities off.')}</p>
        <div className="grid gap-3 sm:grid-cols-2">
          {SANDBOX_RULES.map(rule => <label key={rule.id} className="flex items-start gap-2 text-sm">
            <input type="checkbox" checked={Boolean(options[rule.id])} onChange={event => setPreferences(current => ({ ...current, options: { ...current.options, [rule.id]: event.target.checked } }))} />{ui(rule.label)}
          </label>)}
        </div>
        <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={bothPlayers} onChange={event => setPreferences(current => ({ ...current, bothPlayers: event.target.checked }))} />{ui('Give each player a Sandboxer')}</label>
        <section className="border border-amber-200/25 bg-black/20 p-4" aria-label={ui('Sandboxer rules preview')}>
          <p className="font-semibold">IronSmith Sandboxer <span className="float-right">{0}</span></p>
          <p className="mb-3 text-sm text-muted-foreground">{ui('Enchantment')}</p>
          <div className="whitespace-pre-line text-sm leading-6">{rulesText || ui('Enable an option to add its rules text. No Sandboxer is created when all options are off.')}</div>
        </section>
        <p className="text-sm text-muted-foreground">{ui('After starting, use the Card tab to add existing or custom cards to either player’s hand, library, graveyard, exile, or battlefield. Switch seats to control either player.')}</p>
      </fieldset>
      <div className="space-y-3 border-t border-white/15 pt-4 lg:col-span-2">
        {locked && <p role="status">{ui('Sandbox is available only outside multiplayer lobbies.')}</p>}
        {error && <p role="alert" className="text-red-300">{error}</p>}
        <p className="text-sm text-muted-foreground">{ui('Starting a sandbox replaces the current local table.')}</p>
        <Button disabled={busy || locked || !testedId} onClick={() => void start()}>{ui(busy ? 'Starting sandbox…' : 'Start sandbox')}</Button>
      </div>
    </div>
  </div>;
}
