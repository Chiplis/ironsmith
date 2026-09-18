import { useEffect, useMemo, useState } from "react";
import {
  competitiveDeckToLobbyText,
  DEFAULT_COMPETITIVE_DECK_CATALOG_URL,
  loadCompetitiveDeckCatalog,
  searchCompetitiveDecks,
} from "@/lib/competitive-deck-catalog";

const fieldClass = "fantasy-field w-full px-3 py-2 text-[13px] text-foreground outline-none";

export default function CompetitiveDeckPicker({ onApply, format = "modern" }) {
  const [catalog, setCatalog] = useState(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    loadCompetitiveDeckCatalog()
      .then((nextCatalog) => {
        if (active) setCatalog(nextCatalog);
      })
      .catch((loadError) => {
        if (active) setError(loadError.message || "Could not load the deck catalog");
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, []);

  const decks = useMemo(
    () => searchCompetitiveDecks(catalog, { query, format, limit: 12 }),
    [catalog, format, query],
  );

  function applyDeck(deck) {
    onApply?.({ ...competitiveDeckToLobbyText(deck), deck });
  }

  return (
    <section className="lobby-sheet-panel fantasy-sheet-section grid gap-3 p-3" aria-label="Competitive deck catalog">
      <div className="flex items-baseline justify-between gap-3">
        <div>
          <h3 className="text-[13px] font-semibold uppercase tracking-[0.16em] text-foreground">
            Buscar deck competitivo
          </h3>
          <p className="text-[12px] text-muted-foreground">
            Catálogo local actualizado por el sincronizador
          </p>
        </div>
        <span className="text-[11px] uppercase tracking-[0.12em] text-muted-foreground">{format}</span>
      </div>
      <input
        className={fieldClass}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Arquetipo, carta o evento"
        aria-label="Buscar decks"
      />
      {loading ? <p className="text-[12px] text-muted-foreground">Cargando catálogo…</p> : null}
      {error ? (
        <p className="text-[12px] text-red-300">
          {error}. Fuente: {DEFAULT_COMPETITIVE_DECK_CATALOG_URL}
        </p>
      ) : null}
      {!loading && !error && decks.length === 0 ? (
        <p className="text-[12px] text-muted-foreground">No hay decks para esta búsqueda.</p>
      ) : null}
      <div className="grid gap-2">
        {decks.map((deck) => (
          <article key={deck.id} className="grid gap-2 border border-white/10 bg-black/10 p-3">
            <div className="flex items-start justify-between gap-2">
              <div>
                <h4 className="text-[13px] font-semibold text-foreground">{deck.name}</h4>
                <p className="text-[12px] text-muted-foreground">
                  {deck.archetype || "Sin arquetipo"} · {deck.event || deck.source || "Fuente desconocida"}
                </p>
              </div>
              <button
                type="button"
                className="stone-pill px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.1em]"
                onClick={() => applyDeck(deck)}
              >
                Usar
              </button>
            </div>
            <p className="text-[11px] leading-5 text-muted-foreground">
              {deck.mainboard.reduce((total, card) => total + card.count, 0)} cartas · {deck.source || "sin fuente"}
              {deck.placement ? ` · puesto #${deck.placement}` : ""}
            </p>
            <div className="flex flex-wrap gap-1">
              {deck.mainboard.slice(0, 6).map((card) => (
                <span key={`${deck.id}-${card.name}`} className="rounded border border-white/10 px-1.5 py-0.5 text-[10px] text-muted-foreground">
                  {card.count} {card.name}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
