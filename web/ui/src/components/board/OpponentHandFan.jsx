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

// An opponent's hand, fanned like ours but upside down as if held across the
// table, peeking from the top of their panel. Known cards show their faces
// and open the inspector on hover.
// `mirrorHand` (one opponent): the fan sits over the same column as our own
// hand, as wide as our hand row, with cards scaled from ours.
export default function OpponentHandFan({ player, mirrorHand = false }) {
  const ui = useUiText();
  const ref = useRef(null);
  const [width, setWidth] = useState(0);
  const [mirror, setMirror] = useState(null);
  useLayoutEffect(() => {
    if (!mirrorHand) return undefined;
    const node = ref.current;
    const panel = node?.offsetParent;
    if (!node || !panel) return undefined;
    let frame;
    const measure = () => {
      const row = document.querySelector("[data-my-zone] .hand-zone-row, .hand-zone-row");
      const card = row?.querySelector(".game-card.hand-card");
      if (!row || !card) { setMirror(null); return; }
      const rowRect = row.getBoundingClientRect();
      const panelRect = panel.getBoundingClientRect();
      const next = {
        center: Math.round(rowRect.left + rowRect.width / 2 - panelRect.left),
        width: Math.round(rowRect.width),
        cardWidth: Math.round(Math.max(CARD_W, card.offsetWidth * 0.6)),
      };
      setMirror((current) => current && current.center === next.center && current.width === next.width
        && current.cardWidth === next.cardWidth ? current : next);
    };
    const schedule = () => { cancelAnimationFrame(frame); frame = requestAnimationFrame(measure); };
    measure();
    const observer = new ResizeObserver(schedule);
    observer.observe(panel);
    const row = document.querySelector(".hand-zone-row");
    if (row) observer.observe(row);
    window.addEventListener("resize", schedule);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); window.removeEventListener("resize", schedule); };
  }, [mirrorHand, player?.hand_size]);
  useLayoutEffect(() => {
    if (mirrorHand) return undefined;
    const node = ref.current;
    if (!node) return undefined;
    // The table's top bar (logo, menu) floats over the right end of the top
    // row; the fan stops short of it rather than tucking cards underneath.
    const measure = () => {
      const own = node.getBoundingClientRect();
      // The bar itself may span the row; its visible controls are what count.
      const barLeft = [...document.querySelectorAll(".topbar-shell :is(button, a, img, svg, h1, [role='button'])")]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.bottom > own.top && rect.top < own.bottom && rect.left > own.left)
        .reduce((left, rect) => Math.min(left, rect.left), Infinity);
      setWidth(Math.max(0, Math.min(node.clientWidth, barLeft - own.left - 8)));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, [mirrorHand]);
  const known = knownHandCards(player);
  const total = Math.max(Number(player?.hand_size) || 0, known.length);
  const slots = Array.from({ length: total }, (_, index) => known[index] || null);
  const cardWidth = mirrorHand && mirror ? mirror.cardWidth : CARD_W;
  const room = mirrorHand && mirror ? mirror.width : width;
  const spread = total > 1 ? Math.max(1, Math.min(cardWidth * 0.72, (room - cardWidth) / (total - 1))) : 0;
  // Mirrored fans center on our hand; header fans start right after the name.
  const offset = mirrorHand ? -((total - 1) * spread + cardWidth) / 2 : 0;
  const arcDeg = Math.min(MANABREW_HAND_FAN_PARAMS.maxArcDeg * 0.7, total * 2);
  return (
    <div ref={ref} className={`opponent-hand-fan${mirrorHand ? " opponent-hand-fan--mirror" : ""}`}
      data-opponent-hand={String(player?.id ?? player?.index ?? "")}
      role="img" aria-label={ui("{0}'s hand: {1} cards", { 0: player?.name || ui("Opponent"), 1: total })}
      style={{
        "--opponent-hand-card-width": `${cardWidth}px`,
        ...(mirrorHand ? { left: `${mirror?.center ?? 0}px`, visibility: mirror ? undefined : "hidden" } : null),
      }}>
      {slots.map((card, index) => {
        // Seen from across the table: each card upside down, the fan
        // pivoting from the top edge so its ends rise toward the opponent.
        const t = total > 1 ? (index / (total - 1)) * 2 - 1 : 0;
        const rotate = 180 - t * (arcDeg / 2);
        const lift = (1 - Math.cos((t * Math.PI) / 2)) * 5;
        return <FanCard key={card ? `known-${card.id}` : `back-${index}`} card={card} style={{
          left: `${(offset + index * spread).toFixed(1)}px`,
          zIndex: total - index,
          transform: `translateY(${(-lift).toFixed(1)}px) rotate(${rotate.toFixed(2)}deg)`,
        }} />;
      })}
    </div>
  );
}
