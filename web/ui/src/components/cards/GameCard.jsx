import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { animate, cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import { cn } from "@/lib/utils";
import { fetchScryfallCardMeta, scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons } from "@/lib/mana-symbols";

function glowPhaseFromSeed(seed) {
  let hash = 0;
  const text = String(seed || "");
  for (let i = 0; i < text.length; i++) {
    hash = ((hash * 31) + text.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

function abbreviateCounterKind(rawKind) {
  const directMap = {
    "Plus One Plus One": "+1",
    "Minus One Minus One": "-1",
    Lore: "LR",
    Loyalty: "LY",
    Charge: "CH",
    Shield: "SH",
    Stun: "ST",
    Vigilance: "VG",
    Flying: "FL",
    Trample: "TR",
    Reach: "RE",
    Deathtouch: "DT",
    Menace: "MN",
    Hexproof: "HX",
    Indestructible: "IN",
    FirstStrike: "FS",
    "First Strike": "FS",
    DoubleStrike: "DS",
    "Double Strike": "DS",
    Finality: "FN",
    Brain: "BR",
    Aim: "AM",
    Arrow: "AR",
    Blaze: "BZ",
  };
  if (directMap[rawKind]) return directMap[rawKind];

  const words = String(rawKind || "")
    .split(/[\s/-]+/)
    .map((word) => word.trim())
    .filter(Boolean);
  if (words.length >= 2) {
    return `${words[0][0] || ""}${words[1][0] || ""}`.toUpperCase().slice(0, 2);
  }
  return String(rawKind || "").slice(0, 2).toUpperCase();
}

function counterPalette(rawKind) {
  switch (rawKind) {
    case "Plus One Plus One":
      return { accent: "#70d8a1", fill: "rgba(77, 168, 111, 0.28)", stroke: "#aef0ca" };
    case "Minus One Minus One":
      return { accent: "#df6d83", fill: "rgba(160, 64, 82, 0.28)", stroke: "#ffb0c1" };
    case "Lore":
      return { accent: "#e1bd73", fill: "rgba(171, 124, 43, 0.3)", stroke: "#f8dba2" };
    case "Loyalty":
      return { accent: "#f1b561", fill: "rgba(181, 104, 34, 0.3)", stroke: "#ffd7a2" };
    case "Charge":
      return { accent: "#6bc2ff", fill: "rgba(49, 103, 164, 0.28)", stroke: "#bbebff" };
    case "Shield":
      return { accent: "#84d6cf", fill: "rgba(55, 123, 118, 0.3)", stroke: "#c5f7ef" };
    case "Stun":
      return { accent: "#f2a464", fill: "rgba(170, 88, 29, 0.3)", stroke: "#ffd2a1" };
    case "Vigilance":
      return { accent: "#b7df9f", fill: "rgba(87, 120, 55, 0.28)", stroke: "#ebffd6" };
    case "Finality":
      return { accent: "#b48fff", fill: "rgba(95, 67, 150, 0.28)", stroke: "#ddd0ff" };
    default:
      return { accent: "#a7c3e7", fill: "rgba(59, 86, 122, 0.28)", stroke: "#dcecff" };
  }
}

function buildCounterBadge(counter) {
  const amount = Number(counter?.amount);
  const rawKind = String(counter?.kind || "").trim();
  if (!rawKind || !Number.isFinite(amount) || amount <= 0) return null;

  if (rawKind === "Plus One Plus One") {
    return {
      amount,
      fullLabel: `${amount} +1/+1 counter${amount === 1 ? "" : "s"}`,
      shortLabel: "+1",
      palette: counterPalette(rawKind),
    };
  }
  if (rawKind === "Minus One Minus One") {
    return {
      amount,
      fullLabel: `${amount} -1/-1 counter${amount === 1 ? "" : "s"}`,
      shortLabel: "-1",
      palette: counterPalette(rawKind),
    };
  }

  return {
    amount,
    fullLabel: `${amount} ${rawKind.toLowerCase()} counter${amount === 1 ? "" : "s"}`,
    shortLabel: abbreviateCounterKind(rawKind),
    palette: counterPalette(rawKind),
  };
}

function BattlefieldCounterBadge({ badge }) {
  const amountLabel = badge.amount > 99 ? "99+" : String(badge.amount);
  const labelFontSize = badge.shortLabel.length >= 3 ? 9 : 10;
  const amountFontSize = amountLabel.length >= 3 ? 10 : 12;

  return (
    <span className="battlefield-counter-chip" title={badge.fullLabel}>
      <svg viewBox="0 0 84 28" role="img" aria-label={badge.fullLabel} preserveAspectRatio="none">
        <path
          d="M10 1H69L83 14L69 27H10L1 14Z"
          fill="rgba(6, 11, 18, 0.96)"
        />
        <path
          d="M11 3H64L73.5 14L64 25H11L4 14Z"
          fill={badge.palette.fill}
        />
        <path
          d="M10 1H69L83 14L69 27H10L1 14Z"
          fill="none"
          stroke={badge.palette.stroke}
          strokeWidth="1.4"
        />
        <path
          d="M10 1H26L29 14L26 27H10L1 14Z"
          fill={badge.palette.accent}
        />
        <path
          d="M31 5H66"
          stroke={badge.palette.stroke}
          strokeWidth="0.9"
          strokeLinecap="round"
          opacity="0.45"
        />
        <text
          x="16"
          y="18"
          textAnchor="middle"
          fill="#061019"
          fontSize={amountFontSize}
          fontWeight="800"
          fontFamily="Rajdhani, sans-serif"
        >
          {amountLabel}
        </text>
        <text
          x="50"
          y="18"
          textAnchor="middle"
          fill="#ebf5ff"
          fontSize={labelFontSize}
          fontWeight="800"
          letterSpacing="1.1"
          fontFamily="Rajdhani, sans-serif"
        >
          {badge.shortLabel}
        </text>
      </svg>
    </span>
  );
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
  className = "",
  centerOverlay = null,
}) {
  const name = card.name || "";
  const artVersion = "art_crop";
  const artUrl = scryfallImageUrl(name, artVersion);
  const count = Number(card.count);
  const groupSize = Number.isFinite(count) && count > 1 ? count : 1;
  const [battlefieldManaCost, setBattlefieldManaCost] = useState(card.mana_cost ?? null);
  const glowPhase = glowPhaseFromSeed(`${card.id}:${name}`);
  const auraDelay1 = `-${((glowPhase % 4200) / 1000).toFixed(3)}s`;
  const auraDelay2 = `-${(((glowPhase * 17) % 5600) / 1000).toFixed(3)}s`;
  const rotationSign = glowPhase % 2 === 0 ? -1 : 1;
  const auraRot1Pos = `${0.85 * rotationSign}deg`;
  const auraRot1Neg = `${-0.85 * rotationSign}deg`;
  const auraRot2Pos = `${1.2 * rotationSign}deg`;
  const auraRot2Neg = `${-1.2 * rotationSign}deg`;
  const battlefieldManaIconSize = compact ? 10 : 11;
  const artTreatmentClass = variant === "battlefield"
    ? "opacity-100 saturate-[1.12] contrast-[1.08] brightness-[1.08]"
    : "opacity-72 saturate-[1.05] contrast-[1.04]";
  const rootRef = useRef(null);
  const entryMotionRef = useRef(null);
  const bumpMotionRef = useRef(null);
  const counterBadges = variant === "battlefield" && Array.isArray(card.counters)
    ? card.counters.map(buildCounterBadge).filter(Boolean)
    : [];

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

  useEffect(() => {
    if (variant !== "battlefield" || card.mana_cost != null || !name) return undefined;

    let cancelled = false;
    fetchScryfallCardMeta(name)
      .then((meta) => {
        if (cancelled) return;
        setBattlefieldManaCost(meta?.mana_cost ?? null);
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, [card.mana_cost, name, variant]);

  const visibleBattlefieldManaCost = variant === "battlefield"
    ? (card.mana_cost ?? battlefieldManaCost)
    : null;
  const stableId = card?.stable_id ?? card?.id ?? "";
  const memberStableIds = Array.isArray(card?.member_stable_ids) && card.member_stable_ids.length > 0
    ? card.member_stable_ids
    : [stableId].filter(Boolean);

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
        className,
      )}
      data-object-id={card.id}
      data-stable-id={stableId}
      data-member-stable-ids={memberStableIds.join(",")}
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
        {variant === "battlefield" && (
          <span className="battlefield-frame" aria-hidden="true" />
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
          <div className="battlefield-header">
            <span className="battlefield-nameplate text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">
              {name}
            </span>
            {visibleBattlefieldManaCost && (
              <span className="battlefield-mana-rack">
                <ManaCostIcons cost={visibleBattlefieldManaCost} size={battlefieldManaIconSize} />
              </span>
            )}
          </div>
        )}

        {variant === "battlefield" && counterBadges.length > 0 && (
          <div className="battlefield-counter-rail">
            {counterBadges.map((badge, index) => (
              <BattlefieldCounterBadge
                key={`${badge.fullLabel}-${index}`}
                badge={badge}
              />
            ))}
          </div>
        )}

        {variant === "battlefield" && centerOverlay && (
          <div className="pointer-events-none absolute inset-0 z-[4] flex items-center justify-center">
            <div className="pointer-events-auto">
              {centerOverlay}
            </div>
          </div>
        )}

        {variant === "battlefield" && (groupSize > 1 || card.power_toughness) && (
          <div className="battlefield-footer">
            <div className="battlefield-footer-left">
              {groupSize > 1 && (
                <span className="battlefield-group-badge">
                  x{groupSize}
                </span>
              )}
            </div>
            {card.power_toughness && (
              <span className="battlefield-pt-badge">
                {card.power_toughness}
              </span>
            )}
          </div>
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

        {variant !== "hand" && variant !== "battlefield" && card.power_toughness && (
          <span className="absolute bottom-1 right-1 bg-[rgba(16,24,35,0.92)] text-[#f5d08b] text-[13px] font-bold leading-none px-1 py-0.5 rounded-sm z-2 tracking-wide">
            {card.power_toughness}
          </span>
        )}

      </div>
    </div>
  );
}
