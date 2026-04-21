import { useDragState } from "@/context/DragContext";
import { scryfallImageUrl } from "@/lib/scryfall";

const GLOW_COLORS = {
  land: { border: "rgba(88, 214, 166, 0.6)", shadow: "rgba(88, 214, 166, VAL)" },
  spell: { border: "rgba(100, 169, 255, 0.6)", shadow: "rgba(100, 169, 255, VAL)" },
  ability: { border: "rgba(224, 232, 240, 0.5)", shadow: "rgba(224, 232, 240, VAL)" },
  instant: { border: "rgba(80, 220, 240, 0.6)", shadow: "rgba(80, 220, 240, VAL)" },
  sorcery: { border: "rgba(180, 100, 255, 0.6)", shadow: "rgba(180, 100, 255, VAL)" },
  creature: { border: "rgba(240, 190, 80, 0.6)", shadow: "rgba(240, 190, 80, VAL)" },
  enchantment: { border: "rgba(240, 100, 180, 0.6)", shadow: "rgba(240, 100, 180, VAL)" },
  battle: { border: "rgba(240, 80, 60, 0.6)", shadow: "rgba(240, 80, 60, VAL)" },
  artifact: { border: "rgba(190, 210, 230, 0.6)", shadow: "rgba(190, 210, 230, VAL)" },
  planeswalker: { border: "rgba(247, 160, 64, 0.6)", shadow: "rgba(247, 160, 64, VAL)" },
  extra: { border: "rgba(174, 118, 255, 0.6)", shadow: "rgba(174, 118, 255, VAL)" },
};

function computeProximity(x, y) {
  const dropZone = document.querySelector("[data-drop-zone]");
  if (!dropZone) return { t: 0 };

  const rect = dropZone.getBoundingClientRect();
  const distBelow = Math.max(0, y - rect.bottom);
  const distAbove = Math.max(0, rect.top - y);
  const distVert = distBelow + distAbove;
  const insideZone = y >= rect.top && y <= rect.bottom && x >= rect.left && x <= rect.right;

  if (!insideZone) {
    // 0 = far away, 0.7 = touching the edge
    return { t: Math.max(0, Math.min(0.7, (1 - distVert / 300) * 0.7)) };
  }

  const el = document.elementFromPoint(x, y);
  const overCard = el && el.closest(".game-card");
  return { t: overCard ? 0.8 : 1.0 };
}

export default function DragOverlay() {
  const dragState = useDragState();
  if (!dragState) return null;

  const { cardName, glowKind, currentX, currentY, startX } = dragState;
  const dx = currentX - startX;
  const rotation = Math.max(-8, Math.min(8, dx * 0.05));
  const { t } = computeProximity(currentX, currentY);

  // t: 0 = at hand, 1 = over empty board space
  const scale = 0.7 + t * 0.7;          // 0.7 → 1.4
  const brightness = 1.0 + t * 0.35;    // 1.0 → 1.35
  const glowIntensity = 0.3 + t * 0.5;  // 0.3 → 0.8

  const colors = GLOW_COLORS[glowKind] || { border: "rgba(200,210,220,0.4)", shadow: "rgba(200,210,220,VAL)" };
  const shadowColor = colors.shadow.replace("VAL", glowIntensity.toFixed(2));
  const shadowSpread = Math.round(6 + t * 20);
  const shadowBlur = Math.round(12 + t * 30);

  const artUrl = scryfallImageUrl(cardName, "art_crop");

  return (
    // Outer: tracks cursor position instantly (no transition)
    <div
      className="fixed z-[100] pointer-events-none"
      style={{
        left: currentX,
        top: currentY,
        willChange: "left, top",
      }}
    >
      {/* Inner: scale/brightness/rotation animate smoothly */}
      <div
        style={{
          transform: `translate(-50%, -60%) rotate(${rotation}deg) scale(${scale})`,
          filter: `brightness(${brightness})`,
          transition: "transform 220ms cubic-bezier(0.25, 0.46, 0.45, 0.94)",
          willChange: "transform",
        }}
      >
        <div
          className="w-[180px] h-[140px] rounded-lg font-bold text-[15px] text-[#d8e8ff] overflow-hidden relative"
          style={{
            background: "linear-gradient(180deg, rgba(13,20,30,0.92), rgba(7,12,18,0.95))",
            border: `1.5px solid ${colors.border}`,
            boxShadow: `0 0 ${shadowBlur}px ${shadowSpread}px ${shadowColor}, 0 12px 32px rgba(0,0,0,0.5)`,
            transition: "transform 220ms ease-out, opacity 220ms ease-out",
          }}
        >
          {artUrl && (
            <img
              className="absolute inset-0 w-full h-full object-cover opacity-65"
              src={artUrl}
              alt=""
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          )}
          <span className="relative z-1 px-2.5 py-2 leading-tight block text-shadow-[0_1px_4px_rgba(0,0,0,0.95)]">
            {cardName}
          </span>
        </div>
      </div>
    </div>
  );
}
