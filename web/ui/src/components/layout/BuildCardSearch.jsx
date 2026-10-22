import { useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";
import { CUSTOM_CARD_LIBRARY_CHANGED, loadBundledCustomCards, listSavedCustomCards, customCardOptions } from "@/lib/custom-card-library";

export default function BuildCardSearch({ onSelect, disabled, actions, secondaryControls }) {
  const { game, semanticThreshold } = useGame();
  const ui = useUiText();
  const [query, setQuery] = useState("");
  const [includeCustom, setIncludeCustom] = useState(false);
  const [cards, setCards] = useState([]);
  const [options, setOptions] = useState([]);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    const load = async () => {
      const results = await Promise.allSettled([loadBundledCustomCards(), listSavedCustomCards()]);
      if (!active) return;
      setCards(results.flatMap(result => result.status === "fulfilled" ? result.value : []));
      setError(results.filter(result => result.status === "rejected").map(result => String(result.reason?.message || result.reason)).join(". "));
    };
    void load();
    window.addEventListener(CUSTOM_CARD_LIBRARY_CHANGED, load);
    return () => { active = false; window.removeEventListener(CUSTOM_CARD_LIBRARY_CHANGED, load); };
  }, []);
  useEffect(() => {
    let active = true;
    const timer = setTimeout(async () => {
      try {
        const names = query.trim() ? await game?.autocompleteCardNames(query.trim(), 8) || [] : [];
        if (active) setOptions([
          ...(includeCustom && query.trim() ? customCardOptions(cards, query) : []),
          ...names.map(name => ({ name })),
        ]);
      } catch (error) { if (active) setError(String(error?.message || error)); }
    }, 150);
    return () => { active = false; clearTimeout(timer); };
  }, [query, game, includeCustom, cards, semanticThreshold]);
  return <section className="build-card-search">
    <div className="build-card-search-toolbar">
    <label className="card-forge-field">
      <span className="card-forge-section-label">{ui("Search cards")}</span>
      <input className="card-forge-input" value={query} disabled={disabled}
        placeholder={ui("Find a card to add or edit")}
        onChange={event => { setQuery(event.target.value); setOptions([]); }} />
    </label>
    {actions}
    </div>
    <div className="build-card-search-options">
    <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={includeCustom} onChange={event => setIncludeCustom(event.target.checked)} />{ui("Include custom cards")}</label>
    {secondaryControls}
    </div>
    {error ? <p role="status">{error}</p> : null}
    {options.length ? <div className="build-card-search-results" aria-label={ui("Matching cards")}>
      {options.map((option, index) => <button type="button" className="stone-pill text-left px-3 py-2" key={option.card?.id || `${option.name}-${index}`} disabled={disabled}
        onClick={() => { onSelect(option); setQuery(""); setOptions([]); }}>
        {option.name}{option.card ? ` · ${ui("Custom")}` : ""}
      </button>)}
    </div> : null}
  </section>;
}
