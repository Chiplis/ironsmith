import { useEffect, useLayoutEffect, useRef } from "react";
import { animate, cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import { cn } from "@/lib/utils";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons } from "@/lib/mana-symbols";

function glowPhaseFromSeed(seed) {
  let hash = 0;
  const text = String(seed || "");
  for (let i = 0; i < text.length; i++) {
    hash = ((hash * 31) + text.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

export default function GameCard({
  card,
  compact = false,
  isPlayable = false,
  isInspected = false,
  glowKind = null,
  isHovered = false,
  isDragging = false,
  isNew = false,
  isBumped = false,
  bumpDirection = 0,
  variant = "battlefield",
  onClick,
  onContextMenu,
  onPointerDown,
  onMouseEnter,
  onMouseLeave,
  style,
}) {
  const name = card.name || "";
  const artVersion = "art_crop";
  const artUrl = scryfallImageUrl(name, artVersion);
  const count = Number(card.count);
  const groupSize = Number.isFinite(count) && count > 1 ? count : 1;
  const glowPhase = glowPhaseFromSeed(`${card.id}:${name}`);
  const auraDelay1 = `-${((glowPhase % 4200) / 1000).toFixed(3)}s`;
  const auraDelay2 = `-${(((glowPhase * 17) % 5600) / 1000).toFixed(3)}s`;
  const rotationSign = glowPhase % 2 === 0 ? -1 : 1;
  const auraRot1Pos = `${0.85 * rotationSign}deg`;
  const auraRot1Neg = `${-0.85 * rotationSign}deg`;
  const auraRot2Pos = `${1.2 * rotationSign}deg`;
  const auraRot2Neg = `${-1.2 * rotationSign}deg`;
  const artTreatmentClass = variant === "battlefield"
    ? "opacity-100 saturate-[1.12] contrast-[1.08] brightness-[1.08]"
    : "opacity-72 saturate-[1.05] contrast-[1.04]";
  const rootRef = useRef(null);
  const entryMotionRef = useRef(null);
  const bumpMotionRef = useRef(null);
  const battlefieldNameplateStyle = !compact && variant === "battlefield"
    ? {
      fontSize: "clamp(10px, calc(var(--bf-card-width, 124px) * 0.118), 16px)",
      lineHeight: 1.08,
      paddingInline: "clamp(4px, calc(var(--bf-card-width, 124px) * 0.05), 8px)",
      paddingBlock: "clamp(2px, calc(var(--bf-card-height, 96px) * 0.024), 4px)",
    }
    : undefined;
  const battlefieldCountBadgeStyle = !compact && variant === "battlefield"
    ? {
      fontSize: "clamp(10px, calc(var(--bf-card-width, 124px) * 0.105), 13px)",
      paddingInline: "clamp(3px, calc(var(--bf-card-width, 124px) * 0.028), 5px)",
      paddingBlock: "clamp(1px, calc(var(--bf-card-height, 96px) * 0.01), 3px)",
    }
    : undefined;

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node || !isNew) return undefined;

    cancelMotion(entryMotionRef.current);
    entryMotionRef.current = createTimeline({ autoplay: true }).add(node, {
      opacity: [0, 1],
      scale: [0.74, 1],
      rotateZ: [rotationSign * -6, 0],
      duration: 420,
      ease: uiSpring({ duration: 420, bounce: 0.28 }),
    });
  }, [isNew, rotationSign]);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node || !isBumped || isNew) return undefined;

    cancelMotion(bumpMotionRef.current);
    bumpMotionRef.current = animate(node, {
      keyframes: [
        { scale: 0.94, x: bumpDirection * 4, duration: 110 },
        { scale: 1.025, x: 0, duration: 120 },
        { scale: 1, x: 0, duration: 120 },
      ],
      ease: "out(3)",
    });
  }, [bumpDirection, isBumped, isNew]);

  useEffect(() => () => {
    cancelMotion(entryMotionRef.current);
    entryMotionRef.current = null;
    cancelMotion(bumpMotionRef.current);
    bumpMotionRef.current = null;
  }, []);

  return (
    <div
      ref={rootRef}
      className={cn(
        "game-card p-1.5 grid content-start",
        variant === "battlefield" && "field-card",
        variant === "hand" && "hand-card",
        compact && "w-[96px] min-w-[96px] min-h-[134px] p-1 text-[14px]",
        !compact && variant === "hand" && "flex-1 basis-0 min-w-0 max-w-[124px] min-h-[100px]",
        !compact && variant !== "hand" && "w-[124px] min-w-[124px] min-h-[172px]",
        card.tapped && "tapped",
        isPlayable && !glowKind && "playable",
        glowKind === "land" && "glow-land",
        glowKind === "spell" && "glow-spell",
        glowKind === "ability" && "glow-ability",
        glowKind === "mana" && "glow-mana",
        glowKind === "extra" && "glow-extra",
        glowKind === "instant" && "glow-instant",
        glowKind === "sorcery" && "glow-sorcery",
        glowKind === "creature" && "glow-creature",
        glowKind === "enchantment" && "glow-enchantment",
        glowKind === "battle" && "glow-battle",
        glowKind === "artifact" && "glow-artifact",
        glowKind === "planeswalker" && "glow-planeswalker",
        glowKind === "action-link" && "action-link",
        glowKind === "attack-candidate" && "attack-candidate",
        glowKind === "attack-selected" && "attack-selected",
        glowKind === "blocker-candidate" && "blocker-candidate",
        isHovered && "hovered",
        isDragging && "dragging",
        isInspected && "inspected",
      )}
      data-object-id={card.id}
      data-card-name={name}
      title={groupSize > 1 ? `${name} (${groupSize} grouped permanents)` : name}
      onClick={onClick}
      onContextMenu={onContextMenu}
      onPointerDown={onPointerDown}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={{
        ...style,
        "--aura-delay-1": auraDelay1,
        "--aura-delay-2": auraDelay2,
        "--aura-rot-1-pos": auraRot1Pos,
        "--aura-rot-1-neg": auraRot1Neg,
        "--aura-rot-2-pos": auraRot2Pos,
        "--aura-rot-2-neg": auraRot2Neg,
        ...(isBumped ? { "--bump-x": `${bumpDirection * 4}px` } : undefined),
      }}
    >
      <div className="game-card-surface">
        {artUrl && (
          <img
            className={cn(
              "absolute inset-0 w-full h-full object-cover z-0 pointer-events-none",
              artTreatmentClass,
            )}
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        )}
        <span
          className={cn(
            "game-card-shade",
            variant === "battlefield" && "battlefield-card-shade",
          )}
          aria-hidden="true"
        />

        {variant === "hand" ? (
          <div className="hand-card-header absolute top-0 left-0 right-0 z-2 px-1.5 py-1">
            <div className="hand-card-title whitespace-nowrap overflow-hidden text-ellipsis text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">
              {name}
            </div>
            {(card.mana_cost || card.power_toughness) && (
              <div className="hand-card-peek-meta mt-0.5 flex items-center justify-between gap-1.5">
                {card.mana_cost ? (
                  <span className="inline-flex min-w-0 items-center gap-px overflow-hidden">
                    <ManaCostIcons cost={card.mana_cost} size={12} />
                  </span>
                ) : <span />}
                {card.power_toughness && (
                  <span className="shrink-0 text-[#f5d08b] text-[11px] font-bold leading-none tracking-wide">
                    {card.power_toughness}
                  </span>
                )}
              </div>
            )}
          </div>
        ) : (
          <span
            className="absolute top-0 left-0 right-0 mt-0 bg-[rgba(16,24,35,0.85)] px-1.5 py-0.5 z-2 whitespace-nowrap overflow-hidden text-ellipsis text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]"
            style={battlefieldNameplateStyle}
          >
            {name}
          </span>
        )}

        {/* Grouped count (battlefield) */}
        {variant === "battlefield" && groupSize > 1 && (
          <span
            className="absolute bottom-1 left-1 bg-[rgba(16,24,35,0.92)] text-[#f5d08b] font-bold leading-none rounded-sm z-2 tracking-wide"
            style={battlefieldCountBadgeStyle}
          >
            x{groupSize}
          </span>
        )}

        {/* P/T badge (battlefield) */}
        {variant !== "hand" && card.power_toughness && (
          <span className="absolute bottom-1 right-1 bg-[rgba(16,24,35,0.92)] text-[#f5d08b] text-[13px] font-bold leading-none px-1 py-0.5 rounded-sm z-2 tracking-wide">
            {card.power_toughness}
          </span>
        )}

        {/* Mana cost + P/T bar (hand cards) */}
        {variant === "hand" && (card.mana_cost || card.power_toughness) && (
          <div className="hand-card-bottom-bar absolute bottom-0 left-0 right-0 z-2 flex items-center justify-between px-1 py-0.5 bg-[rgba(6,10,16,0.92)]">
            {card.mana_cost ? (
              <span className="inline-flex items-center gap-px">
                <ManaCostIcons cost={card.mana_cost} size={14} />
              </span>
            ) : <span />}
            {card.power_toughness && (
              <span className="text-[#f5d08b] text-[12px] font-bold leading-none tracking-wide">
                {card.power_toughness}
              </span>
            )}
          </div>
        )}

      </div>
    </div>
  );
}
