import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from "@/components/ui/sheet";

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
      <div className="decklist-section-heading">
        <h3 >
          {title}
        </h3>
        <span >
          {sanitizeCards(cards).length}
        </span>
      </div>
      {grouped.length > 0 ? (
        <div className="grid gap-1">
          {grouped.map((entry) => (
            <div
              key={entry.name}
              className="decklist-row"
            >
              <span className="font-bold tabular-nums text-[#f0d28e]">{entry.count}</span>
              <span className="font-medium" title={entry.name}>
                {entry.name}
              </span>
            </div>
          ))}
        </div>
      ) : (
        <div className="decklist-empty">
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
    <Sheet open={Boolean(decklist)} onOpenChange={(open) => { if (!open) onClose?.(); }}>
      <SheetContent side="center" className="fantasy-sheet decklist-sheet gap-0 p-0" style={{ maxWidth: "760px" }}>
        <SheetHeader className="fantasy-sheet-header">
          <SheetTitle>{playerName}</SheetTitle>
          <SheetDescription>Open decklist · {deck.length} main · {sideboard.length} sideboard</SheetDescription>
        </SheetHeader>
        <div className="min-h-0 overflow-y-auto px-4 py-4">
          {decklist.available === false ? (
            <p className="decklist-empty" role="status">No shared decklist is available for this player. Decklists appear here when shared through a multiplayer lobby.</p>
          ) : <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,0.72fr)]">
            <DeckSection title="Deck" cards={deck} />
            <div className="grid content-start gap-4">
              {commanders.length > 0 ? <DeckSection title="Commanders" cards={commanders} /> : null}
              <DeckSection title="Sideboard" cards={sideboard} />
            </div>
          </div>}
        </div>
      </SheetContent>
    </Sheet>
  );
}
