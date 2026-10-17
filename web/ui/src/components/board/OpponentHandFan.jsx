import { useLayoutEffect, useRef, useState } from "react";
import useUiText from "@/i18n/useUiText";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { useHoverActions } from "@/context/HoverContext";
import { HIDDEN_CARD_BACK_IMAGE_URL } from "@/lib/scryfall";
import { knownHandCards } from "@/lib/look-pile";
import { MANABREW_HAND_FAN_PARAMS } from "@/hooks/useManabrewHandScale";

const CARD_W = 40;
// Neighbours overlap like the local fan; past this the fan packs tighter so
// any hand fits the same width.
const MAX_SPREAD = CARD_W * 0.72;

function FanCard({ card, style }) {
  const ui = useUiText();
  const { hoverCard, clearHover } = useHoverActions();
  const url = useScryfallImageUrl(card?.name || null, "normal");
  const known = Boolean(card);
  const preview = () => { if (known) hoverCard(card.id); };
  return (
    <span
      className="opponent-hand-card"
      data-known={known ? "true" : undefined}
      data-object-id={known ? card.id : undefined}
      style={style}
      tabIndex={known ? 0 : undefined}
      aria-label={known ? card.name : undefined}
      title={known ? card.name : undefined}
      onPointerEnter={(event) => { if (event.pointerType !== "touch") preview(); }}
      onPointerLeave={() => { if (known) clearHover(); }}
      onFocus={preview}
      onBlur={() => { if (known) clearHover(); }}
    >
      <img src={known && url ? url : HIDDEN_CARD_BACK_IMAGE_URL} alt="" draggable={false}
        loading="lazy" referrerPolicy="no-referrer" />
      {known && !url ? <span className="opponent-hand-card-name">{card.name || ui("Card")}</span> : null}
    </span>
  );
}

// An opponent's hand, fanned like ours and seen from the backs, hanging from
// the top of their panel. Known cards show their faces.
export default function OpponentHandFan({ player }) {
  const ui = useUiText();
  const ref = useRef(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return undefined;
    const measure = () => setWidth(node.clientWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  const known = knownHandCards(player);
  const total = Math.max(Number(player?.hand_size) || 0, known.length);
  const slots = Array.from({ length: total }, (_, index) => known[index] || null);
  const spread = total > 1 ? Math.max(1, Math.min(MAX_SPREAD, (width - CARD_W) / (total - 1))) : 0;
  const span = (total - 1) * spread;
  const arcDeg = Math.min(MANABREW_HAND_FAN_PARAMS.maxArcDeg * 0.7, total * 2);
  return (
    <div ref={ref} className="opponent-hand-fan" data-opponent-hand={String(player?.id ?? player?.index ?? "")}
      role="img" aria-label={ui("{0}'s hand: {1} cards", { 0: player?.name || ui("Opponent"), 1: total })}
      style={{ "--opponent-hand-card-width": `${CARD_W}px` }}>
      {slots.map((card, index) => {
        const t = total > 1 ? (index / (total - 1)) * 2 - 1 : 0;
        const x = -span / 2 + index * spread;
        const rotate = t * (arcDeg / 2);
        const drop = (1 - Math.cos((t * Math.PI) / 2)) * 5;
        return <FanCard key={card ? `known-${card.id}` : `back-${index}`} card={card} style={{
          left: `calc(50% + ${(x - CARD_W / 2).toFixed(1)}px)`,
          zIndex: index + 1,
          transform: `translateY(${drop.toFixed(1)}px) rotate(${rotate.toFixed(2)}deg)`,
        }} />;
      })}
    </div>
  );
}
