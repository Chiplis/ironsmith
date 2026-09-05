import { useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { ArrowLeftRight } from "lucide-react";

function countCards(cards) {
  const counts = new Map();
  for (const card of cards || []) {
    const name = String(card || "").trim();
    if (!name) continue;
    counts.set(name, (counts.get(name) || 0) + 1);
  }
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

function moveCard(cards, name, direction) {
  const source = [...(cards?.source || [])];
  const target = [...(cards?.target || [])];
  const index = source.findIndex((card) => card === name);
  if (index < 0) return cards;
  const [card] = source.splice(index, 1);
  if (direction === "to-main") {
    return { source, target: [...target, card] };
  }
  return { source, target: [...target, card] };
}

function CardColumn({ title, cards, emptyText, actionLabel, onMove, disabled = false }) {
  const countedCards = useMemo(() => countCards(cards), [cards]);
  return (
    <section className="setup-editor flex min-h-0 flex-1 flex-col border border-[rgba(128,107,78,0.42)] bg-[rgba(11,13,15,0.74)]">
      <div className="flex shrink-0 items-center justify-between border-b border-[rgba(128,107,78,0.28)] px-3 py-2">
        <h2 className="text-[12px] font-bold uppercase tracking-wider text-[#f2d9a3]">
          {title}
        </h2>
        <span className="text-[12px] font-semibold text-muted-foreground">
          {cards.length}
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {countedCards.length === 0 ? (
          <div className="px-2 py-8 text-center text-[13px] italic text-muted-foreground">
            {emptyText}
          </div>
        ) : (
          <div className="grid gap-1">
            {countedCards.map((entry) => (
              <button
                key={entry.name}
                type="button"
                className="group flex min-h-9 w-full items-center gap-2 border border-transparent bg-[rgba(255,255,255,0.035)] px-2 py-1.5 text-left transition-colors hover:border-primary/45 hover:bg-secondary"
                onClick={() => {
                  if (!disabled) onMove(entry.name);
                }}
                disabled={disabled}
                title={actionLabel}
                aria-label={`${entry.name}: ${actionLabel}`}
              >
                <span className="w-8 shrink-0 text-[12px] font-bold text-primary">
                  {entry.count}x
                </span>
                <span className="min-w-0 flex-1 break-words text-[13px] font-semibold text-foreground">
                  {entry.name}
                </span>
                <ArrowLeftRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground transition-colors group-hover:text-primary" />
              </button>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

export default function RematchSideboardingView() {
  const { multiplayer, updateRematchDecks } = useGame();
  const rematch = multiplayer?.rematch || {};
  const localDeck = rematch.localDeck || [];
  const localSideboard = rematch.localSideboard || [];
  const localReady = Boolean(rematch.localReady);
  const readyPlayers = (rematch.players || []).filter((player) => player.ready).length;
  const totalPlayers = (rematch.players || []).length;

  const moveToSideboard = (name) => {
    const moved = moveCard({ source: localDeck, target: localSideboard }, name, "to-sideboard");
    updateRematchDecks?.({ deck: moved.source, sideboard: moved.target });
  };

  const moveToMain = (name) => {
    const moved = moveCard({ source: localSideboard, target: localDeck }, name, "to-main");
    updateRematchDecks?.({ deck: moved.target, sideboard: moved.source });
  };

  return (
    <div className="setup-screen sideboarding-screen flex h-full min-h-0 w-full flex-col overflow-hidden bg-[linear-gradient(180deg,rgba(17,16,14,0.98),rgba(8,10,12,0.98))] px-3 py-3">
      <div className="mb-3 flex shrink-0 items-end justify-between gap-3">
        <div>
          <h1 className="text-[18px] font-bold uppercase tracking-wide text-[#f2d9a3]">
            Sideboard
          </h1>
          <div className="mt-1 text-[12px] font-semibold text-muted-foreground">
            {readyPlayers}/{totalPlayers} ready
          </div>
        </div>
        <div className="text-right text-[12px] font-semibold text-muted-foreground">
          Main {localDeck.length} · Sideboard {localSideboard.length}
        </div>
      </div>
      <div className="grid min-h-0 flex-1 gap-3 md:grid-cols-2">
        <CardColumn
          title="Main Deck"
          cards={localDeck}
          emptyText="Main deck is empty"
          actionLabel="Move one copy to sideboard"
          onMove={moveToSideboard}
          disabled={localReady}
        />
        <CardColumn
          title="Sideboard"
          cards={localSideboard}
          emptyText="Sideboard is empty"
          actionLabel="Move one copy to main deck"
          onMove={moveToMain}
          disabled={localReady}
        />
      </div>
      {localReady ? (
        <div className="mt-3 shrink-0 border border-[#8ec4ff]/35 bg-[#102033] px-3 py-2 text-[12px] font-bold uppercase tracking-wide text-[#c6ddff]">
          Waiting for the other players.
        </div>
      ) : (
        <div className="mt-3 shrink-0 text-[12px] text-muted-foreground">
          Use the main decision button when ready.
        </div>
      )}
    </div>
  );
}
