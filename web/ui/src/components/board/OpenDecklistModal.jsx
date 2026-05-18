import { X } from "lucide-react";

function sanitizeCards(cards) {
  if (!Array.isArray(cards)) return [];
  return cards.map((card) => String(card || "").trim()).filter(Boolean);
}

function groupedCards(cards) {
  const counts = new Map();
  for (const card of sanitizeCards(cards)) {
    counts.set(card, (counts.get(card) || 0) + 1);
  }
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

function DeckSection({ title, cards }) {
  const grouped = groupedCards(cards);
  return (
    <section className="min-h-0">
      <div className="mb-2 flex items-baseline justify-between gap-3 border-b border-[#32465d] pb-1">
        <h3 className="text-[12px] font-bold uppercase tracking-[0.18em] text-[#a9c4df]">
          {title}
        </h3>
        <span className="text-[12px] font-semibold text-[#d9e8f8]">
          {sanitizeCards(cards).length}
        </span>
      </div>
      {grouped.length > 0 ? (
        <div className="grid gap-1">
          {grouped.map((entry) => (
            <div
              key={entry.name}
              className="grid grid-cols-[2.25rem_minmax(0,1fr)] items-baseline gap-2 rounded-none border border-[#26384d] bg-[#07111c]/72 px-2 py-1 text-[13px]"
            >
              <span className="font-bold tabular-nums text-[#f0d28e]">{entry.count}</span>
              <span className="truncate font-semibold text-[#e5eef8]" title={entry.name}>
                {entry.name}
              </span>
            </div>
          ))}
        </div>
      ) : (
        <div className="rounded-none border border-[#26384d] bg-[#07111c]/55 px-3 py-2 text-[13px] font-semibold text-[#7f98b2]">
          Empty
        </div>
      )}
    </section>
  );
}

export default function OpenDecklistModal({ decklist, onClose }) {
  if (!decklist) return null;
  const deck = sanitizeCards(decklist.deck);
  const sideboard = sanitizeCards(decklist.sideboard);
  const commanders = sanitizeCards(decklist.commanders);
  const playerName = String(decklist.playerName || "Player");

  return (
    <div
      className="fixed inset-0 z-[180] flex items-center justify-center bg-black/62 px-4 py-5"
      role="dialog"
      aria-modal="true"
      aria-label={`${playerName} decklist`}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose?.();
      }}
    >
      <div className="grid max-h-[min(760px,92vh)] w-full max-w-[760px] grid-rows-[auto_minmax(0,1fr)] overflow-hidden rounded-none border border-[#48627d] bg-[linear-gradient(180deg,#0b1724,#050b12)] shadow-[0_22px_70px_rgba(0,0,0,0.58)]">
        <header className="flex min-w-0 items-center gap-3 border-b border-[#31485f] px-4 py-3">
          <div className="min-w-0">
            <div className="truncate text-[18px] font-bold uppercase tracking-[0.08em] text-[#f0d28e]">
              {playerName}
            </div>
            <div className="mt-0.5 text-[12px] font-semibold uppercase tracking-[0.16em] text-[#8da8c4]">
              Open decklist
            </div>
          </div>
          <button
            type="button"
            className="ml-auto inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-none border border-[#47627f] bg-[#07111c] text-[#d9e8f8] transition-colors hover:bg-[#102137]"
            onClick={onClose}
            aria-label="Close decklist"
          >
            <X className="h-4 w-4" aria-hidden="true" />
          </button>
        </header>
        <div className="min-h-0 overflow-y-auto px-4 py-4">
          <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,0.72fr)]">
            <DeckSection title="Deck" cards={deck} />
            <div className="grid content-start gap-4">
              {commanders.length > 0 ? <DeckSection title="Commanders" cards={commanders} /> : null}
              <DeckSection title="Sideboard" cards={sideboard} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
