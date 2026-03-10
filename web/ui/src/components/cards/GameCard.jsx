import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { animate, cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import { cn } from "@/lib/utils";
import { fetchScryfallCardMeta, scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons } from "@/lib/mana-symbols";

const semanticScoreCache = new Map();

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function normalizeSemanticScore(rawScore) {
  const score = Number(rawScore);
  if (!Number.isFinite(score) || score < 0) return null;
  return Math.min(1, Math.max(0, score));
}

function formatSemanticScore(score) {
  return `${(score * 100).toFixed(1)}%`;
}

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

function normalizeCounterEntry(rawCounter, fallbackKind = "") {
  const kind = String(
    rawCounter?.kind
    ?? rawCounter?.name
    ?? rawCounter?.counter_type
    ?? fallbackKind
    ?? ""
  ).trim();
  const amount = Number(
    rawCounter?.amount
    ?? rawCounter?.count
    ?? rawCounter?.value
  );
  if (!kind || !Number.isFinite(amount) || amount <= 0) return null;
  return { kind, amount };
}

function parseCounterSignature(counterSignature) {
  const signature = String(counterSignature || "").trim();
  if (!signature || signature === "-") return [];

  return signature
    .split("|")
    .map((entry) => {
      const divider = entry.lastIndexOf(":");
      if (divider <= 0) return null;
      const kind = entry.slice(0, divider).trim();
      const amount = Number(entry.slice(divider + 1).trim());
      return normalizeCounterEntry({ amount }, kind);
    })
    .filter(Boolean);
}

function resolveBattlefieldCounters(rawCounters, counterSignature) {
  if (Array.isArray(rawCounters)) {
    const normalized = rawCounters
      .map((counter) => normalizeCounterEntry(counter))
      .filter(Boolean);
    if (normalized.length > 0) return normalized;
  }

  if (rawCounters && typeof rawCounters === "object") {
    const normalized = Object.entries(rawCounters)
      .map(([kind, amount]) => normalizeCounterEntry({ amount }, kind))
      .filter(Boolean);
    if (normalized.length > 0) return normalized;
  }

  return parseCounterSignature(counterSignature);
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

function handCardFooterStat(card) {
  if (card?.power_toughness) {
    return {
      label: card.power_toughness,
      className: "text-[#f5d08b]",
      title: `Power/Toughness ${card.power_toughness}`,
    };
  }
  if (card?.loyalty != null) {
    return {
      label: `L${card.loyalty}`,
      className: "text-[#f2be6b]",
      title: `Loyalty ${card.loyalty}`,
    };
  }
  if (card?.defense != null) {
    return {
      label: `D${card.defense}`,
      className: "text-[#8fd8ff]",
      title: `Defense ${card.defense}`,
    };
  }
  return null;
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
  handCircuitMode = "full",
  hideDebugBadge = false,
}) {
  const { game, inspectorDebug } = useGame();
  const name = card.name || "";
  const artVersion = "art_crop";
  const artUrl = scryfallImageUrl(name, artVersion);
  const count = Number(card.count);
  const groupSize = Number.isFinite(count) && count > 1 ? count : 1;
  const battlefieldStackDepth = variant === "battlefield"
    ? Math.max(0, Math.min(groupSize, 4) - 1)
    : 0;
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
  const stableId = card?.stable_id ?? card?.id ?? "";
  const directSemanticScore = normalizeSemanticScore(card?.semantic_score);
  if (directSemanticScore != null) {
    semanticScoreCache.set(name, directSemanticScore);
  }
  const [fetchedSemanticState, setFetchedSemanticState] = useState(() => ({
    name,
    score: semanticScoreCache.get(name) ?? null,
  }));
  const fetchedSemanticScore = fetchedSemanticState.name === name
    ? fetchedSemanticState.score
    : null;
  const semanticScore = directSemanticScore
    ?? fetchedSemanticScore
    ?? (semanticScoreCache.get(name) ?? null);
  const battlefieldCircuitActive = variant === "battlefield" && (
    isInspected
    || glowKind === "action-link"
    || glowKind === "attack-selected"
    || glowKind === "spell"
  );
  const showBattlefieldCircuit = battlefieldCircuitActive;
  const showHandCircuit = variant === "hand" && (Boolean(glowKind) || isPlayable || isInspected);
  const showCircuitAnimation = showBattlefieldCircuit || showHandCircuit;
  const replaceGlowWithCircuit = (
    (variant === "hand" && showHandCircuit)
    || (variant === "battlefield" && battlefieldCircuitActive)
  );
  const usesTopOnlyHandCircuit = variant === "hand" && handCircuitMode === "top";
  const circuitViewBox = usesTopOnlyHandCircuit ? "0 0 100 46" : "0 0 100 140";
  const circuitPath = usesTopOnlyHandCircuit
    ? "M2.5 1.5H97.5"
    : "M5.5 2.5H94.5C97.26 2.5 99.5 4.74 99.5 7.5V132.5C99.5 135.26 97.26 137.5 94.5 137.5H5.5C2.74 137.5 0.5 135.26 0.5 132.5V7.5C0.5 4.74 2.74 2.5 5.5 2.5Z";
  const rootRef = useRef(null);
  const entryMotionRef = useRef(null);
  const bumpMotionRef = useRef(null);
  const stackMotionRef = useRef(null);
  const circuitMotionRefs = useRef([]);
  const circuitGlowRef = useRef(null);
  const circuitCoreRef = useRef(null);
  const circuitAccentRef = useRef(null);
  const stackCleanupTimersRef = useRef([]);
  const previousGroupSizeRef = useRef(groupSize);
  const counterBadges = variant === "battlefield"
    ? resolveBattlefieldCounters(card?.counters, card?.counter_signature)
      .map(buildCounterBadge)
      .filter(Boolean)
    : [];
  const handFooterStat = variant === "hand" ? handCardFooterStat(card) : null;
  const debugSimilarityLabel = semanticScore != null ? formatSemanticScore(semanticScore) : null;
  const showDebugSimilarityBadge = (
    inspectorDebug
    && !hideDebugBadge
    && variant !== "stack"
    && debugSimilarityLabel != null
  );

  useEffect(() => {
    if (!inspectorDebug || !game || !name || semanticScoreCache.has(name)) return undefined;

    let cancelled = false;
    game.getCardSemanticScore(name)
      .then((rawScore) => {
        if (cancelled) return;
        const nextScore = normalizeSemanticScore(rawScore);
        if (nextScore == null) return;
        semanticScoreCache.set(name, nextScore);
        setFetchedSemanticState({ name, score: nextScore });
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, [game, inspectorDebug, name]);

  const clearStackAnimation = () => {
    const node = rootRef.current;
    if (!node) return;
    for (const timeoutId of stackCleanupTimersRef.current) {
      window.clearTimeout(timeoutId);
    }
    stackCleanupTimersRef.current = [];
    node.style.removeProperty("--card-jolt-x");
    node.style.removeProperty("--card-jolt-y");
    node.style.removeProperty("--card-jolt-rotate");
    node.style.removeProperty("--card-jolt-scale");
    node.style.removeProperty("--card-flash-brightness");
    const badge = node.querySelector(".battlefield-group-badge");
    if (badge) {
      badge.style.removeProperty("transform");
      badge.style.removeProperty("opacity");
    }
    const layers = node.querySelectorAll(".battlefield-group-stack-layer");
    for (const layer of layers) {
      layer.style.removeProperty("transform");
      layer.style.removeProperty("opacity");
      layer.style.removeProperty("visibility");
    }
    const enteringLayers = node.querySelectorAll(".battlefield-group-stack-layer-entering");
    for (const enteringLayer of enteringLayers) {
      enteringLayer.remove();
    }
  };

  useLayoutEffect(() => {
    circuitMotionRefs.current.forEach(cancelMotion);
    circuitMotionRefs.current = [];

    if (!showCircuitAnimation) return undefined;

    const startOffset = -((glowPhase % 1000) + 100);
    const primaryNodes = [circuitGlowRef.current, circuitCoreRef.current].filter(Boolean);

    if (primaryNodes.length > 0) {
      circuitMotionRefs.current.push(
        animate(primaryNodes, {
          strokeDashoffset: [startOffset, startOffset - 1000],
          ease: "linear",
          duration: 2400 + (glowPhase % 900),
          loop: true,
        })
      );
    }

    if (circuitAccentRef.current) {
      circuitMotionRefs.current.push(
        animate(circuitAccentRef.current, {
          strokeDashoffset: [startOffset - 460, startOffset - 1460],
          ease: "linear",
          duration: 4200 + (glowPhase % 1400),
          loop: true,
        })
      );
    }

    return () => {
      circuitMotionRefs.current.forEach(cancelMotion);
      circuitMotionRefs.current = [];
    };
  }, [glowPhase, showCircuitAnimation]);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node || !isNew) return undefined;

    cancelMotion(entryMotionRef.current);
    node.style.removeProperty("transform");
    entryMotionRef.current = createTimeline({ autoplay: true }).add(node, {
      keyframes: [
        {
          opacity: 0,
          "--card-jolt-scale": 0.74,
          "--card-jolt-rotate": `${rotationSign * -6}deg`,
          duration: 0,
        },
        {
          opacity: 1,
          "--card-jolt-scale": 1,
          "--card-jolt-rotate": "0deg",
          duration: 420,
        },
      ],
      ease: uiSpring({ duration: 420, bounce: 0.28 }),
      onComplete: () => {
        node.style.removeProperty("opacity");
        node.style.removeProperty("--card-jolt-scale");
        node.style.removeProperty("--card-jolt-rotate");
      },
    });
    return () => {
      cancelMotion(entryMotionRef.current);
      entryMotionRef.current = null;
      node.style.removeProperty("opacity");
      node.style.removeProperty("--card-jolt-scale");
      node.style.removeProperty("--card-jolt-rotate");
    };
  }, [isNew, rotationSign]);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node || !isBumped || isNew) return undefined;

    cancelMotion(bumpMotionRef.current);
    bumpMotionRef.current = animate(node, {
      keyframes: [
        {
          "--card-jolt-scale": 0.94,
          "--card-jolt-x": `${bumpDirection * 4}px`,
          duration: 110,
        },
        {
          "--card-jolt-scale": 1.025,
          "--card-jolt-x": "0px",
          duration: 120,
        },
        {
          "--card-jolt-scale": 1,
          "--card-jolt-x": "0px",
          duration: 120,
        },
      ],
      ease: "out(3)",
      onComplete: () => {
        node.style.removeProperty("--card-jolt-scale");
        node.style.removeProperty("--card-jolt-x");
      },
    });
    return () => {
      cancelMotion(bumpMotionRef.current);
      bumpMotionRef.current = null;
      node.style.removeProperty("--card-jolt-scale");
      node.style.removeProperty("--card-jolt-x");
    };
  }, [bumpDirection, isBumped, isNew]);

  useLayoutEffect(() => {
    const node = rootRef.current;
    const previousGroupSize = previousGroupSizeRef.current;
    previousGroupSizeRef.current = groupSize;
    if (!node || variant !== "battlefield" || groupSize <= previousGroupSize || groupSize <= 1) {
      return undefined;
    }

    cancelMotion(stackMotionRef.current);
    stackMotionRef.current = null;
    clearStackAnimation();

    const badge = node.querySelector(".battlefield-group-badge");
    const layers = Array.from(node.querySelectorAll(".battlefield-group-stack-layer"));
    const width = node.getBoundingClientRect().width || 124;
    const height = node.getBoundingClientRect().height || 96;
    const currentVisibleDepth = Math.max(0, Math.min(groupSize, 4) - 1);
    const previousVisibleDepth = Math.max(0, Math.min(previousGroupSize, 4) - 1);
    const timeline = createTimeline({ autoplay: true });

    timeline.add(node, {
      keyframes: [
        {
          "--card-jolt-y": "-1px",
          "--card-jolt-scale": 1.018,
          "--card-flash-brightness": 1.08,
          duration: 110,
        },
        {
          "--card-jolt-y": "0px",
          "--card-jolt-scale": 1,
          "--card-flash-brightness": 1,
          duration: 210,
        },
      ],
      ease: uiSpring({ duration: 320, bounce: 0.18 }),
    }, 0);

    if (badge) {
      timeline.add(badge, {
        keyframes: [
          { scale: 0.86, opacity: 0.76, duration: 0 },
          { scale: 1.18, opacity: 1, duration: 150 },
          { scale: 1, opacity: 1, duration: 170 },
        ],
        ease: uiSpring({ duration: 320, bounce: 0.24 }),
        onComplete: () => {
          badge.style.removeProperty("transform");
          badge.style.removeProperty("opacity");
        },
      }, 0);
    }

    if (currentVisibleDepth > previousVisibleDepth) {
      for (let depth = previousVisibleDepth + 1; depth <= currentVisibleDepth; depth += 1) {
        const layer = layers[depth - 1];
        if (!layer) continue;
        const targetX = depth * clamp(width * 0.03, 3, 5);
        const targetY = depth * clamp(height * -0.032, -3, -2);
        const startX = targetX - clamp(width * 0.96, 184, 244);
        const startY = targetY + clamp(height * 0.09, 12, 22);
        const visibleX = targetX - clamp(width * 0.58, 104, 148);
        const visibleY = targetY + clamp(height * 0.16, 16, 30);
        const midX = targetX - clamp(width * 0.2, 32, 54);
        const midY = targetY + clamp(height * 0.08, 10, 18);
        const entryDuration = 560;
        const targetOpacity = Math.max(0.4, 0.92 - (depth * 0.11));
        const delay = 12 + ((depth - previousVisibleDepth - 1) * 42);
        const enteringLayer = layer.cloneNode(true);
        enteringLayer.classList.add("battlefield-group-stack-layer-entering");
        enteringLayer.style.zIndex = "5";
        enteringLayer.style.setProperty("--stack-enter-start-x", `${startX}px`);
        enteringLayer.style.setProperty("--stack-enter-target-x", `${targetX}px`);
        enteringLayer.style.setProperty("--stack-enter-visible-x", `${visibleX}px`);
        enteringLayer.style.setProperty("--stack-enter-mid-x", `${midX}px`);
        enteringLayer.style.setProperty("--stack-enter-target-y", `${targetY}px`);
        enteringLayer.style.setProperty("--stack-enter-start-y", `${startY}px`);
        enteringLayer.style.setProperty("--stack-enter-visible-y", `${visibleY}px`);
        enteringLayer.style.setProperty("--stack-enter-mid-y", `${midY}px`);
        enteringLayer.style.setProperty("--stack-enter-start-rotate", "-24deg");
        enteringLayer.style.setProperty("--stack-enter-visible-rotate", "-15deg");
        enteringLayer.style.setProperty("--stack-enter-mid-rotate", "-6deg");
        enteringLayer.style.setProperty("--stack-enter-target-opacity", String(targetOpacity));
        enteringLayer.style.animationDelay = `${delay}ms`;
        layer.style.visibility = "hidden";
        node.appendChild(enteringLayer);

        const cleanupTimer = window.setTimeout(() => {
          if (!node.isConnected) return;
          if (layer.isConnected) {
            layer.style.removeProperty("visibility");
          }
          if (enteringLayer.isConnected) {
            enteringLayer.remove();
          }
        }, delay + entryDuration + 50);
        stackCleanupTimersRef.current.push(cleanupTimer);
      }
    }

    stackMotionRef.current = timeline;

    return () => {
      cancelMotion(stackMotionRef.current);
      stackMotionRef.current = null;
      clearStackAnimation();
    };
  }, [groupSize, variant]);

  useEffect(() => () => {
    cancelMotion(entryMotionRef.current);
    entryMotionRef.current = null;
    cancelMotion(bumpMotionRef.current);
    bumpMotionRef.current = null;
    cancelMotion(stackMotionRef.current);
    stackMotionRef.current = null;
    circuitMotionRefs.current.forEach(cancelMotion);
    circuitMotionRefs.current = [];
    clearStackAnimation();
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
        battlefieldStackDepth > 0 && "battlefield-grouped-card",
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
        showCircuitAnimation && "card-circuit-active",
        replaceGlowWithCircuit && "card-circuit-replaces-glow",
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
      {variant === "battlefield" && battlefieldStackDepth > 0 && (
        <div className="battlefield-group-stack" aria-hidden="true">
          {Array.from({ length: battlefieldStackDepth }, (_, index) => (
            <div
              key={`stack-layer-${index + 1}`}
              className="battlefield-group-stack-layer"
              style={{
                "--group-stack-depth": index + 1,
                zIndex: battlefieldStackDepth - index,
              }}
            >
              {artUrl && (
                <img
                  className="battlefield-group-stack-art"
                  src={artUrl}
                  alt=""
                  loading="lazy"
                  referrerPolicy="no-referrer"
                />
              )}
              <span className="battlefield-frame" aria-hidden="true" />
              <span className="game-card-shade battlefield-card-shade" aria-hidden="true" />
            </div>
          ))}
        </div>
      )}
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
        {showCircuitAnimation && (
          <div className="card-circuit-overlay" aria-hidden="true">
            <svg
              className={cn(
                "card-circuit-svg",
                usesTopOnlyHandCircuit && "card-circuit-svg-top",
              )}
              viewBox={circuitViewBox}
              preserveAspectRatio="none"
            >
              <path
                className="card-circuit-track"
                d={circuitPath}
                pathLength="1000"
              />
              <path
                ref={circuitGlowRef}
                className="card-circuit-glow"
                d={circuitPath}
                pathLength="1000"
              />
              <path
                ref={circuitCoreRef}
                className="card-circuit-core"
                d={circuitPath}
                pathLength="1000"
              />
              <path
                ref={circuitAccentRef}
                className="card-circuit-accent"
                d={circuitPath}
                pathLength="1000"
              />
            </svg>
          </div>
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
            {showDebugSimilarityBadge && (
              <span
                className="absolute right-1.5 top-1 rounded border border-[#6aa6d5]/50 bg-[rgba(7,13,20,0.88)] px-1 py-0.5 text-[10px] font-semibold leading-none tracking-wide text-[#bfe5ff] shadow-[0_2px_6px_rgba(0,0,0,0.32)]"
                title={`Similarity score: ${debugSimilarityLabel}`}
              >
                {debugSimilarityLabel}
              </span>
            )}
          </div>
        ) : (
          <div className="battlefield-header">
            <span className="battlefield-nameplate text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">
              {name}
            </span>
            <span className="flex items-center gap-1">
              {showDebugSimilarityBadge && (
                <span
                  className="rounded border border-[#6aa6d5]/50 bg-[rgba(7,13,20,0.88)] px-1 py-0.5 text-[10px] font-semibold leading-none tracking-wide text-[#bfe5ff] shadow-[0_2px_6px_rgba(0,0,0,0.32)]"
                  title={`Similarity score: ${debugSimilarityLabel}`}
                >
                  {debugSimilarityLabel}
                </span>
              )}
              {visibleBattlefieldManaCost && (
                <span className="battlefield-mana-rack">
                  <ManaCostIcons cost={visibleBattlefieldManaCost} size={battlefieldManaIconSize} />
                </span>
              )}
            </span>
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
        {variant === "hand" && (card.mana_cost || handFooterStat) && (
          <div className="hand-card-bottom-bar absolute bottom-0 left-0 right-0 z-2 flex items-center justify-between px-1 py-0.5 bg-[rgba(6,10,16,0.92)]">
            {card.mana_cost ? (
              <span className="inline-flex items-center gap-px">
                <ManaCostIcons cost={card.mana_cost} size={14} />
              </span>
            ) : <span />}
            {handFooterStat && (
              <span
                className={cn(
                  "text-[12px] font-bold leading-none tracking-wide",
                  handFooterStat.className,
                )}
                title={handFooterStat.title}
              >
                {handFooterStat.label}
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
