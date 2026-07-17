import GameCard from "@/components/cards/GameCard";
import { playerDisplayName } from "@/lib/player-display";

export default function PlanarZone({ state, selectedObjectId = null, onInspect }) {
  const planechase = state?.planechase;
  const cards = Array.isArray(planechase?.face_up) ? planechase.face_up : [];
  if (!planechase || cards.length === 0) return null;

  const controller = (state?.players || []).find(
    (player) => Number(player?.id) === Number(planechase.planar_controller)
  );
  const totalDeckCards = (planechase.deck_sizes || []).reduce(
    (total, deck) => total + Number(deck?.size || 0),
    0
  );

  return (
    <section
      className="pointer-events-auto absolute left-1/2 top-1/2 z-[68] flex max-w-[42vw] -translate-x-1/2 -translate-y-1/2 items-center gap-2 rounded-sm border border-cyan-300/35 bg-slate-950/90 p-1.5 shadow-[0_0_24px_rgba(34,211,238,0.18)] backdrop-blur"
      aria-label="Planechase planar zone"
      data-planar-zone
    >
      <div className="flex gap-1.5">
        {cards.map((card) => (
          <button
            key={card.id}
            type="button"
            className="h-[104px] w-[148px] shrink-0 overflow-hidden rounded-sm text-left ring-offset-1 ring-offset-slate-950 hover:ring-2 hover:ring-cyan-300/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-200"
            onClick={() => onInspect?.(card.id)}
            aria-label={`Inspect ${card.name}`}
          >
            <GameCard
              card={{
                ...card,
                type_line: card.kind === "phenomenon" ? "Phenomenon" : "Plane",
                card_types: [],
              }}
              compact
              selected={Number(selectedObjectId) === Number(card.id)}
              hideDebugBadge
              className="h-full min-h-full w-full min-w-full"
            />
          </button>
        ))}
      </div>
      <div className="min-w-[116px] pr-1 text-[10px] uppercase tracking-[0.12em] text-slate-300">
        <div className="font-semibold text-cyan-200">Planar zone</div>
        <div className="mt-1 normal-case tracking-normal text-slate-100">
          {controller ? playerDisplayName(state.players || [], controller) : "Planar controller"}
        </div>
        <div className="mt-1">Next roll {Number(planechase.die_roll_cost || 0) === 0 ? "free" : `{${planechase.die_roll_cost}}`}</div>
        <div>{planechase.communal_deck ? "Communal" : "Planar"} deck {totalDeckCards}</div>
      </div>
    </section>
  );
}
