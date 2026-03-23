import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { animate, cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import { debounceClick, debouncePointerDown } from "@/lib/interactionDebounce";
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
          fontFamily="Optima, Avenir Next, Segoe UI, Candara, sans-serif"
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
          fontFamily="Optima, Avenir Next, Segoe UI, Candara, sans-serif"
        >
          {badge.shortLabel}
        </text>
      </svg>
    </span>
  );
}

const BATTLEFIELD_SYMBOL_DEFS = [
  { id: "flying", title: "Flying", aliases: ["flying"] },
  { id: "vigilance", title: "Vigilance", aliases: ["vigilance"] },
  { id: "trample", title: "Trample", aliases: ["trample"] },
  { id: "haste", title: "Haste", aliases: ["haste"] },
  { id: "deathtouch", title: "Deathtouch", aliases: ["deathtouch"] },
  { id: "lifelink", title: "Lifelink", aliases: ["lifelink"] },
  { id: "menace", title: "Menace", aliases: ["menace"] },
  { id: "ward", title: "Ward", aliases: ["ward"] },
  { id: "hexproof", title: "Hexproof", aliases: ["hexproof"] },
  { id: "indestructible", title: "Indestructible", aliases: ["indestructible"] },
  { id: "reach", title: "Reach", aliases: ["reach"] },
  { id: "flash", title: "Flash", aliases: ["flash"] },
  { id: "first-strike", title: "First Strike", aliases: ["first strike", "firststrike"] },
  { id: "double-strike", title: "Double Strike", aliases: ["double strike", "doublestrike"] },
];
const BATTLEFIELD_MANA_TEXT_FIELDS = ["oracle_text", "effect_text", "ability_text"];
const BATTLEFIELD_MANA_SYMBOL_RE = /\{([^}]+)\}/g;
const BATTLEFIELD_MANA_CODE_RE = /^(?:W|U|B|R|G|C|S|E|T|Q|X|Y|Z|\d+)$/i;

function manaBadgeSymbolId(code) {
  return `mana:${code}`;
}

function battlefieldManaCodeFromSymbolId(symbolId) {
  if (typeof symbolId !== "string" || !symbolId.startsWith("mana:")) return null;
  const code = symbolId.slice(5).trim().toUpperCase();
  return code || null;
}

function normalizeBattlefieldManaCode(rawCode) {
  const code = String(rawCode || "").trim().toUpperCase();
  if (!code || !BATTLEFIELD_MANA_CODE_RE.test(code)) return null;
  return code;
}

function buildBattlefieldManaSymbolDisplay(code) {
  const normalizedCode = normalizeBattlefieldManaCode(code);
  if (!normalizedCode) return null;
  return {
    kind: "symbol",
    symbol: manaBadgeSymbolId(normalizedCode),
    title: `Adds ${normalizedCode}`,
  };
}

function buildBattlefieldSplitManaSymbolDisplay(leftCode, rightCode) {
  const left = normalizeBattlefieldManaCode(leftCode);
  const right = normalizeBattlefieldManaCode(rightCode);
  if (!left || !right || left === right) {
    return buildBattlefieldManaSymbolDisplay(left || right);
  }
  return {
    kind: "split-symbol",
    left: manaBadgeSymbolId(left),
    right: manaBadgeSymbolId(right),
    title: `Adds ${left} or ${right}`,
  };
}

function escapeRegExp(text) {
  return String(text).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function normalizeBattlefieldBadgeDisplay(value) {
  if (!value) return null;
  if (value.kind === "number") return value.label ? value : null;
  if (value.kind === "symbol") return value.symbol ? value : null;
  if (value.kind === "split-symbol") {
    return value.left && value.right ? value : null;
  }
  return null;
}

function uniqueBattlefieldDisplays(displays) {
  const unique = [];
  const seen = new Set();
  for (const display of displays) {
    const normalized = normalizeBattlefieldBadgeDisplay(display);
    if (!normalized) continue;
    const key = normalized.kind === "number"
      ? `number:${normalized.label}`
      : normalized.kind === "symbol"
        ? `symbol:${normalized.symbol}`
        : `split:${normalized.left}|${normalized.right}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(normalized);
  }
  return unique;
}

function deriveBattlefieldManaDisplays(card) {
  const displays = [];
  const sections = BATTLEFIELD_MANA_TEXT_FIELDS
    .map((field) => card?.[field])
    .filter(Boolean)
    .map((value) => String(value));

  for (const section of sections) {
    const clauses = section
      .split(/\r?\n|(?<=[.;])\s+/)
      .map((clause) => clause.trim())
      .filter(Boolean);
    for (const clause of clauses) {
      if (!/\badd\b/i.test(clause)) continue;
      const codes = Array.from(clause.matchAll(BATTLEFIELD_MANA_SYMBOL_RE))
        .map((match) => normalizeBattlefieldManaCode(match[1]))
        .filter(Boolean);
      if (codes.length === 0) continue;

      const uniqueCodes = [...new Set(codes)];
      if (uniqueCodes.length >= 2 && /(?:\bor\b|\/)/i.test(clause)) {
        displays.push(buildBattlefieldSplitManaSymbolDisplay(uniqueCodes[0], uniqueCodes[1]));
      } else {
        for (const code of uniqueCodes.slice(0, 2)) {
          displays.push(buildBattlefieldManaSymbolDisplay(code));
        }
      }
    }
  }

  if (displays.length === 0 && Array.isArray(card?.produced_mana)) {
    const producedMana = card.produced_mana
      .map(normalizeBattlefieldManaCode)
      .filter(Boolean);
    const uniqueProducedMana = [...new Set(producedMana)];
    if (uniqueProducedMana.length >= 2) {
      displays.push(buildBattlefieldSplitManaSymbolDisplay(uniqueProducedMana[0], uniqueProducedMana[1]));
    } else if (uniqueProducedMana.length === 1) {
      displays.push(buildBattlefieldManaSymbolDisplay(uniqueProducedMana[0]));
    }
  }

  return uniqueBattlefieldDisplays(displays).slice(0, 2);
}

function deriveBattlefieldEffectSymbolDisplays(card) {
  const textParts = [];
  if (Array.isArray(card?.keywords)) {
    for (const keyword of card.keywords) {
      if (keyword) textParts.push(String(keyword));
    }
  }
  for (const value of [card?.oracle_text, card?.effect_text, card?.ability_text, card?.keyword_text]) {
    if (value) textParts.push(String(value));
  }
  const sourceText = textParts.join("\n").trim();
  if (!sourceText) return [];

  const matches = [];
  for (const def of BATTLEFIELD_SYMBOL_DEFS) {
    for (const alias of def.aliases) {
      const regex = new RegExp(`\\b${escapeRegExp(alias)}\\b`, "gi");
      let match = regex.exec(sourceText);
      while (match) {
        matches.push({
          id: def.id,
          title: def.title,
          index: match.index,
          end: match.index + match[0].length,
        });
        match = regex.exec(sourceText);
      }
    }
  }

  matches.sort((left, right) => left.index - right.index);
  const orderedUnique = [];
  const seenIds = new Set();
  for (const match of matches) {
    if (seenIds.has(match.id)) continue;
    seenIds.add(match.id);
    orderedUnique.push(match);
  }

  const displays = [];
  for (let index = 0; index < orderedUnique.length; index += 1) {
    const current = orderedUnique[index];
    const next = orderedUnique[index + 1] || null;
    if (next) {
      const between = sourceText.slice(current.end, next.index);
      if (/^[\s,;:()/-]*(?:or|\/)[\s,;:()/-]*$/i.test(between)) {
        displays.push({
          kind: "split-symbol",
          left: current.id,
          right: next.id,
          title: `${current.title} or ${next.title}`,
        });
        index += 1;
        continue;
      }
    }
    displays.push({
      kind: "symbol",
      symbol: current.id,
      title: current.title,
    });
  }

  return uniqueBattlefieldDisplays(displays).slice(0, 2);
}

function renderBattlefieldBadgeSymbolShape(symbolId) {
  switch (symbolId) {
    case "flying":
      return (
        <>
          <path d="M-8 2 Q-2 -7 7 -1" />
          <path d="M-7 4 Q-1 0 6 5" />
        </>
      );
    case "vigilance":
      return <path d="M0 -8 L7 -4 V2 C7 6 4 8 0 10 C-4 8 -7 6 -7 2 V-4 Z" />;
    case "trample":
      return (
        <>
          <path d="M-8 5 L-2 -3 L2 2 L8 -6" />
          <path d="M5 -6 H8 V-3" />
        </>
      );
    case "haste":
      return <path d="M-4 -8 L1 -8 L-2 -1 H4 L-5 10 L-1 2 H-6 Z" />;
    case "deathtouch":
      return (
        <>
          <path d="M0 -9 C4 -4 5 -1 5 2 C5 6 2 9 0 10 C-2 9 -5 6 -5 2 C-5 -1 -4 -4 0 -9 Z" />
          <path d="M-2 2 Q0 4 2 2" />
        </>
      );
    case "lifelink":
      return (
        <>
          <circle cx="0" cy="0" r="8" />
          <path d="M0 -4 V4" />
          <path d="M-4 0 H4" />
        </>
      );
    case "menace":
      return (
        <>
          <path d="M-8 -2 L-1 -8 L-1 8" />
          <path d="M8 -2 L1 -8 L1 8" />
        </>
      );
    case "ward":
    case "hexproof":
      return <path d="M0 -9 L7 -5 V4 L0 9 L-7 4 V-5 Z" />;
    case "indestructible":
      return <path d="M0 -9 L4 -1 L9 0 L4 4 L5 9 L0 6 L-5 9 L-4 4 L-9 0 L-4 -1 Z" />;
    case "reach":
      return (
        <>
          <circle cx="0" cy="0" r="6.5" />
          <path d="M0 -10 V10" />
          <path d="M-10 0 H10" />
        </>
      );
    case "flash":
      return (
        <>
          <path d="M-5 -8 L1 -8 L-1 -2 H5 L-4 9 L-1 1 H-7 Z" />
          <path d="M5 -7 L8 -10" />
        </>
      );
    case "first-strike":
      return <path d="M-6 8 L4 -8" />;
    case "double-strike":
      return (
        <>
          <path d="M-8 8 L1 -8" />
          <path d="M-1 8 L8 -8" />
        </>
      );
    default:
      return <circle cx="0" cy="0" r="7.5" />;
  }
}

function battlefieldManaSymbolPalette(code) {
  switch (code) {
    case "W":
      return { fill: "#e7dbc1", stroke: "#fff6df", text: "#2f281b" };
    case "U":
      return { fill: "#6ba9d8", stroke: "#dff4ff", text: "#081721" };
    case "B":
      return { fill: "#857b92", stroke: "#ece4f5", text: "#faf7ff" };
    case "R":
      return { fill: "#cf6a49", stroke: "#ffd8ca", text: "#2b0902" };
    case "G":
      return { fill: "#6a9f5b", stroke: "#ddf6d7", text: "#0d1a0c" };
    case "C":
      return { fill: "#8c949f", stroke: "#eef2f5", text: "#10161b" };
    default:
      return { fill: "#9aa5b1", stroke: "#eff4f8", text: "#11161b" };
  }
}

function renderBattlefieldManaSymbolShape(code, isMain) {
  const palette = battlefieldManaSymbolPalette(code);
  const radius = isMain ? 8 : 6.9;
  const fontSize = isMain ? 9.6 : 7.7;
  const y = isMain ? 3.7 : 3.1;

  return (
    <>
      <circle
        cx="0"
        cy="0"
        r={radius}
        fill={palette.fill}
        stroke={palette.stroke}
        strokeWidth={isMain ? 1.35 : 1.2}
      />
      <text
        x="0"
        y={y}
        textAnchor="middle"
        fill={palette.text}
        fontSize={fontSize}
        fontWeight="900"
        letterSpacing="-0.02em"
        fontFamily="Bahnschrift, Avenir Next Condensed, Segoe UI, sans-serif"
      >
        {code}
      </text>
    </>
  );
}

function renderBattlefieldSplitChoiceBackdrop(display, slot, ids) {
  const leftManaCode = battlefieldManaCodeFromSymbolId(display?.left);
  const rightManaCode = battlefieldManaCodeFromSymbolId(display?.right);
  if (!leftManaCode || !rightManaCode) return null;

  const isMain = slot === "main";
  const leftFill = battlefieldManaSymbolPalette(leftManaCode).fill;
  const rightFill = battlefieldManaSymbolPalette(rightManaCode).fill;
  const leftClipId = isMain ? ids.mainLeft : ids.sideLeft;
  const rightClipId = isMain ? ids.mainRight : ids.sideRight;
  const choiceDividerClassName = isMain
    ? "battlefield-token-main-divider battlefield-token-choice-divider"
    : "battlefield-token-side-divider battlefield-token-choice-divider";

  return (
    <>
      {isMain ? (
        <>
          <polygon
            points="60,78 71,84.5 71,97 60,103.5 49,97 49,84.5"
            className="battlefield-token-choice-half battlefield-token-choice-half--left"
            fill={leftFill}
            clipPath={`url(#${leftClipId})`}
          />
          <polygon
            points="60,78 71,84.5 71,97 60,103.5 49,97 49,84.5"
            className="battlefield-token-choice-half battlefield-token-choice-half--right"
            fill={rightFill}
            clipPath={`url(#${rightClipId})`}
          />
          <path d="M57 83.25 L60 80.9 L63 83.25" className="battlefield-token-choice-indicator" />
          <path d="M60 81 L60 101" className={choiceDividerClassName} />
        </>
      ) : (
        <>
          <rect
            x="77"
            y="83.5"
            width="26"
            height="19"
            rx="3.5"
            className="battlefield-token-choice-half battlefield-token-choice-half--left"
            fill={leftFill}
            clipPath={`url(#${leftClipId})`}
          />
          <rect
            x="77"
            y="83.5"
            width="26"
            height="19"
            rx="3.5"
            className="battlefield-token-choice-half battlefield-token-choice-half--right"
            fill={rightFill}
            clipPath={`url(#${rightClipId})`}
          />
          <path d="M87.7 87.2 L90 85.5 L92.3 87.2" className="battlefield-token-choice-indicator" />
          <path d="M90 84.5 L90 101.5" className={choiceDividerClassName} />
        </>
      )}
    </>
  );
}

function renderBattlefieldBadgeGraphic(symbolId, options) {
  const { centerX, centerY, symbolScale, clipPathId, isMain, offsetX = 0 } = options;
  const manaCode = battlefieldManaCodeFromSymbolId(symbolId);
  if (manaCode) {
    const content = (
      <g
        className="battlefield-token-mana-symbol"
        transform={`translate(${centerX + offsetX} ${centerY}) scale(${symbolScale})`}
      >
        {renderBattlefieldManaSymbolShape(manaCode, isMain)}
      </g>
    );
    return clipPathId ? <g clipPath={`url(#${clipPathId})`}>{content}</g> : content;
  }

  const shape = (
    <g
      className="battlefield-token-symbol"
      transform={`translate(${centerX + offsetX} ${centerY}) scale(${symbolScale})`}
    >
      {renderBattlefieldBadgeSymbolShape(symbolId)}
    </g>
  );
  return clipPathId ? <g clipPath={`url(#${clipPathId})`}>{shape}</g> : shape;
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

function battlefieldPrimaryInfo(card) {
  if (card?.power_toughness) {
    return {
      kind: "number",
      label: String(card.power_toughness),
      title: `Power/Toughness ${card.power_toughness}`,
    };
  }
  if (card?.loyalty != null) {
    return {
      kind: "number",
      label: String(card.loyalty),
      title: `Loyalty ${card.loyalty}`,
    };
  }
  if (card?.defense != null) {
    return {
      kind: "number",
      label: String(card.defense),
      title: `Defense ${card.defense}`,
    };
  }
  return null;
}

function renderBattlefieldTokenBadgeContent(display, slot, ids) {
  const normalized = normalizeBattlefieldBadgeDisplay(display);
  if (!normalized) return null;

  const isMain = slot === "main";
  const centerX = isMain ? 60 : 90;
  const centerY = isMain ? 91 : 93;
  const symbolScale = isMain ? 1 : 0.92;
  const textClassName = isMain ? "battlefield-token-main-text" : "battlefield-token-side-text";
  const dividerClassName = isMain ? "battlefield-token-main-divider" : "battlefield-token-side-divider";
  const leftClipId = isMain ? ids.mainLeft : ids.sideLeft;
  const rightClipId = isMain ? ids.mainRight : ids.sideRight;
  const isManaChoice = normalized.kind === "split-symbol"
    && battlefieldManaCodeFromSymbolId(normalized.left)
    && battlefieldManaCodeFromSymbolId(normalized.right);

  if (normalized.kind === "number") {
    return (
      <text
        x={centerX}
        y={isMain ? 97 : 97}
        textAnchor="middle"
        className={textClassName}
      >
        {normalized.label}
      </text>
    );
  }

  if (normalized.kind === "symbol") {
    return renderBattlefieldBadgeGraphic(normalized.symbol, {
      centerX,
      centerY,
      symbolScale,
      isMain,
    });
  }

  return (
    <>
      {isManaChoice ? renderBattlefieldSplitChoiceBackdrop(normalized, slot, ids) : null}
      {renderBattlefieldBadgeGraphic(normalized.left, {
        centerX,
        centerY,
        symbolScale: isManaChoice ? (symbolScale * (isMain ? 0.84 : 0.8)) : symbolScale,
        clipPathId: leftClipId,
        isMain,
        offsetX: isManaChoice ? (isMain ? -4.2 : -3.1) : 0,
      })}
      {renderBattlefieldBadgeGraphic(normalized.right, {
        centerX,
        centerY,
        symbolScale: isManaChoice ? (symbolScale * (isMain ? 0.84 : 0.8)) : symbolScale,
        clipPathId: rightClipId,
        isMain,
        offsetX: isManaChoice ? (isMain ? 4.2 : 3.1) : 0,
      })}
      {!isManaChoice ? (
        <path
          d={isMain ? "M60 81 L60 101" : "M90 84.5 L90 101.5"}
          className={dividerClassName}
        />
      ) : null}
    </>
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
  onPointerUp,
  onPointerCancel,
  onPointerLeave,
  onMouseEnter,
  onMouseLeave,
  style,
  className = "",
  centerOverlay = null,
  handCircuitMode = "full",
  hideDebugBadge = false,
  suppressTooltip = false,
  battlefieldVisualMode = "classic",
}) {
  const { game, inspectorDebug } = useGame();
  const name = card.name || "";
  const artVersion = variant === "hand" ? "normal" : "art_crop";
  const artUrl = scryfallImageUrl(name, artVersion);
  const useTokenBattlefield = variant === "battlefield" && battlefieldVisualMode === "mobile-token";
  const count = Number(card.count);
  const groupSize = Number.isFinite(count) && count > 1 ? count : 1;
  const battlefieldStackDepth = variant === "battlefield"
    ? Math.max(0, Math.min(groupSize, 4) - 1)
    : 0;
  const [fetchedBattlefieldMeta, setFetchedBattlefieldMeta] = useState(null);
  const glowPhase = glowPhaseFromSeed(`${card.id}:${name}`);
  const auraDelay1 = `-${((glowPhase % 4200) / 1000).toFixed(3)}s`;
  const auraDelay2 = `-${(((glowPhase * 17) % 5600) / 1000).toFixed(3)}s`;
  const rotationSign = glowPhase % 2 === 0 ? -1 : 1;
  const auraRot1Pos = `${0.85 * rotationSign}deg`;
  const auraRot1Neg = `${-0.85 * rotationSign}deg`;
  const auraRot2Pos = `${1.2 * rotationSign}deg`;
  const auraRot2Neg = `${-1.2 * rotationSign}deg`;
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
  const artTreatmentClass = variant === "battlefield"
    ? "opacity-100"
    : variant === "hand"
      ? "opacity-100"
      : "opacity-72";
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
  const totalBattlefieldCounters = counterBadges.reduce((sum, badge) => sum + badge.amount, 0);
  const activeFetchedBattlefieldMeta = fetchedBattlefieldMeta?.name === name
    ? fetchedBattlefieldMeta
    : null;
  const resolvedBattlefieldCard = variant === "battlefield"
    ? {
      ...card,
      mana_cost: card.mana_cost ?? activeFetchedBattlefieldMeta?.mana_cost ?? null,
      oracle_text: String(card?.oracle_text || activeFetchedBattlefieldMeta?.oracle_text || ""),
      produced_mana: Array.isArray(card?.produced_mana) && card.produced_mana.length > 0
        ? card.produced_mana
        : (Array.isArray(activeFetchedBattlefieldMeta?.produced_mana)
          ? activeFetchedBattlefieldMeta.produced_mana
          : []),
    }
    : card;
  const manaBattlefieldDisplays = variant === "battlefield"
    ? deriveBattlefieldManaDisplays(resolvedBattlefieldCard)
    : [];
  const symbolicBattlefieldDisplays = variant === "battlefield"
    ? deriveBattlefieldEffectSymbolDisplays(resolvedBattlefieldCard)
    : [];
  const numericPrimaryBattlefieldInfo = variant === "battlefield"
    ? battlefieldPrimaryInfo(resolvedBattlefieldCard)
    : null;
  const numericSecondaryBattlefieldInfo = variant !== "battlefield"
    ? null
    : groupSize > 1
      ? {
        kind: "number",
        label: groupSize > 99 ? "99+" : String(groupSize),
        title: `${groupSize} grouped permanents`,
      }
      : totalBattlefieldCounters > 0
        ? {
          kind: "number",
          label: totalBattlefieldCounters > 99 ? "99+" : String(totalBattlefieldCounters),
          title: `${totalBattlefieldCounters} counter${totalBattlefieldCounters === 1 ? "" : "s"}`,
        }
        : null;
  const primaryBattlefieldInfo = (
    numericPrimaryBattlefieldInfo
    || manaBattlefieldDisplays[0]
    || symbolicBattlefieldDisplays[0]
    || null
  );
  const secondaryBattlefieldInfo = numericSecondaryBattlefieldInfo
    || (!numericPrimaryBattlefieldInfo && (manaBattlefieldDisplays[1] || symbolicBattlefieldDisplays[0]))
    || null;
  const battlefieldSvgIdBase = `battlefield-${String(stableId || card?.id || name || "card")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "") || "card"}`;
  const battlefieldBaseGradientId = `${battlefieldSvgIdBase}-base-gradient`;
  const battlefieldRingGradientId = `${battlefieldSvgIdBase}-ring-gradient`;
  const battlefieldBadgeGradientId = `${battlefieldSvgIdBase}-badge-gradient`;
  const battlefieldSideBadgeGradientId = `${battlefieldSvgIdBase}-side-badge-gradient`;
  const battlefieldImageClipId = `${battlefieldSvgIdBase}-image-clip`;
  const battlefieldMainLeftClipId = `${battlefieldSvgIdBase}-main-left-clip`;
  const battlefieldMainRightClipId = `${battlefieldSvgIdBase}-main-right-clip`;
  const battlefieldSideLeftClipId = `${battlefieldSvgIdBase}-side-left-clip`;
  const battlefieldSideRightClipId = `${battlefieldSvgIdBase}-side-right-clip`;
  const handFooterStat = variant === "hand" ? handCardFooterStat(card) : null;
  const debugSimilarityLabel = semanticScore != null ? formatSemanticScore(semanticScore) : null;
  const showDebugSimilarityBadge = (
    inspectorDebug
    && !hideDebugBadge
    && variant !== "stack"
    && debugSimilarityLabel != null
  );
  const debouncedOnClick = debounceClick(onClick);
  const debouncedOnPointerDown = debouncePointerDown(onPointerDown);

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
    if (variant !== "battlefield" || !name) return undefined;
    const needsMeta = (
      card.mana_cost == null
      || !String(card?.oracle_text || "").trim()
      || !(Array.isArray(card?.produced_mana) && card.produced_mana.length > 0)
    );
    if (!needsMeta) return undefined;

    let cancelled = false;
    fetchScryfallCardMeta(name)
      .then((meta) => {
        if (cancelled) return;
        setFetchedBattlefieldMeta({
          name,
          mana_cost: meta?.mana_cost ?? null,
          oracle_text: String(meta?.oracle_text || ""),
          produced_mana: Array.isArray(meta?.produced_mana) ? meta.produced_mana : [],
        });
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, [card.mana_cost, card.oracle_text, card.produced_mana, name, variant]);

  const visibleBattlefieldManaCost = variant === "battlefield"
    ? (resolvedBattlefieldCard.mana_cost ?? null)
    : null;
  const memberStableIds = Array.isArray(card?.member_stable_ids) && card.member_stable_ids.length > 0
    ? card.member_stable_ids
    : [stableId].filter(Boolean);

  return (
    <div
      ref={rootRef}
      className={cn(
        "game-card grid content-start",
        useTokenBattlefield ? "p-0.5" : "p-1.5",
        variant === "battlefield" && "field-card",
        useTokenBattlefield && "battlefield-token-card",
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
        glowKind === "play-from" && "glow-extra glow-play-from",
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
      title={suppressTooltip ? undefined : (groupSize > 1 ? `${name} (${groupSize} grouped permanents)` : name)}
      onClick={debouncedOnClick}
      onContextMenu={onContextMenu}
      onPointerDown={debouncedOnPointerDown}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      onPointerLeave={onPointerLeave}
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
            </div>
          ))}
        </div>
      )}
      <div className="game-card-surface">
        {artUrl && (variant !== "battlefield" || !useTokenBattlefield) && (
          <img
            className={cn(
              "absolute inset-0 w-full h-full z-0 pointer-events-none",
              variant === "battlefield" ? "object-contain" : "object-cover",
              artTreatmentClass,
            )}
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        )}
        {variant === "battlefield" && !useTokenBattlefield && (
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
        {useTokenBattlefield ? (
          <div className="battlefield-token-shell">
            <svg
              className="battlefield-token-svg"
              viewBox="0 0 120 120"
              role="img"
              aria-label={name}
              preserveAspectRatio="xMidYMid meet"
            >
              <defs>
                <linearGradient id={battlefieldBaseGradientId} x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stopColor="#c8cdd3" />
                  <stop offset="38%" stopColor="#7d8691" />
                  <stop offset="72%" stopColor="#39424d" />
                  <stop offset="100%" stopColor="#191f26" />
                </linearGradient>
                <linearGradient id={battlefieldRingGradientId} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#edf1f5" />
                  <stop offset="28%" stopColor="#b1b8c0" />
                  <stop offset="62%" stopColor="#5a636e" />
                  <stop offset="100%" stopColor="#1b2129" />
                </linearGradient>
                <linearGradient id={battlefieldBadgeGradientId} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#5a616c" />
                  <stop offset="48%" stopColor="#2a313a" />
                  <stop offset="100%" stopColor="#10151b" />
                </linearGradient>
                <linearGradient id={battlefieldSideBadgeGradientId} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#646c77" />
                  <stop offset="52%" stopColor="#313843" />
                  <stop offset="100%" stopColor="#12171d" />
                </linearGradient>
                <clipPath id={battlefieldImageClipId}>
                  <circle cx="60" cy="45" r="38" />
                </clipPath>
                <clipPath id={battlefieldMainLeftClipId}>
                  <rect x="48" y="78" width="12" height="26" />
                </clipPath>
                <clipPath id={battlefieldMainRightClipId}>
                  <rect x="60" y="78" width="12" height="26" />
                </clipPath>
                <clipPath id={battlefieldSideLeftClipId}>
                  <rect x="77" y="83.5" width="13" height="19" />
                </clipPath>
                <clipPath id={battlefieldSideRightClipId}>
                  <rect x="90" y="83.5" width="13" height="19" />
                </clipPath>
              </defs>

              <path
                d="M24 72 Q60 82 96 72 L86 100 L60 114 L34 100 Z"
                className="battlefield-token-base"
                fill={`url(#${battlefieldBaseGradientId})`}
              />

              <path
                d="M28 75 Q60 84 92 75"
                className="battlefield-token-base-edge"
              />

              <circle
                cx="60"
                cy="45"
                r="42"
                fill={`url(#${battlefieldRingGradientId})`}
                className="battlefield-token-ring"
              />

              <circle
                cx="60"
                cy="45"
                r="38"
                className="battlefield-token-inner-ring"
              />

              <path
                d="M33 22 A34 34 0 0 1 87 22"
                className="battlefield-token-ring-glint"
              />

              {artUrl && (
                <image
                  href={artUrl}
                  x="22"
                  y="7"
                  width="76"
                  height="76"
                  clipPath={`url(#${battlefieldImageClipId})`}
                  preserveAspectRatio="xMidYMid slice"
                />
              )}

              <polygon
                points="60,78 71,84.5 71,97 60,103.5 49,97 49,84.5"
                className="battlefield-token-main-badge"
                fill={`url(#${battlefieldBadgeGradientId})`}
              />

              {renderBattlefieldTokenBadgeContent(primaryBattlefieldInfo, "main", {
                mainLeft: battlefieldMainLeftClipId,
                mainRight: battlefieldMainRightClipId,
                sideLeft: battlefieldSideLeftClipId,
                sideRight: battlefieldSideRightClipId,
              })}

              {secondaryBattlefieldInfo ? (
                <>
                  <rect
                    x="77"
                    y="83.5"
                    width="26"
                    height="19"
                    rx="3.5"
                    className="battlefield-token-side-badge"
                    fill={`url(#${battlefieldSideBadgeGradientId})`}
                  />
                  {renderBattlefieldTokenBadgeContent(secondaryBattlefieldInfo, "side", {
                    mainLeft: battlefieldMainLeftClipId,
                    mainRight: battlefieldMainRightClipId,
                    sideLeft: battlefieldSideLeftClipId,
                    sideRight: battlefieldSideRightClipId,
                  })}
                </>
              ) : null}
            </svg>

            {primaryBattlefieldInfo ? (
              <span className="sr-only">
                {primaryBattlefieldInfo.title}
              </span>
            ) : null}
            {secondaryBattlefieldInfo ? (
              <span className="sr-only">
                {secondaryBattlefieldInfo.title}
              </span>
            ) : null}
          </div>
        ) : variant === "hand" ? (
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
            <span className="battlefield-header-copy">
              <span className="battlefield-nameplate text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">
                {name}
              </span>
              {groupSize > 1 && (
                <span className="battlefield-group-badge">
                  x{groupSize}
                </span>
              )}
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
                  <ManaCostIcons cost={visibleBattlefieldManaCost} size={compact ? 10 : 11} />
                </span>
              )}
            </span>
          </div>
        )}

        {variant === "battlefield" && !useTokenBattlefield && counterBadges.length > 0 && (
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

        {variant === "battlefield" && !useTokenBattlefield && card.power_toughness && (
          <div className="battlefield-footer">
            <span className="battlefield-pt-badge">
              {card.power_toughness}
            </span>
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
