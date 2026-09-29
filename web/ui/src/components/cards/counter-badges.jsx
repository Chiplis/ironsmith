/* Shared counter helpers are intentionally imported by both card renderers. */
/* eslint-disable react-refresh/only-export-components */
import { useId } from "react";
import useUiText from "@/i18n/useUiText";
import { counterDisplayLabel, counterSymbolUrl } from "@/lib/mana-assets";

function abbreviateCounterKind(rawKind) {
  const powerToughnessLabel = counterDisplayLabel(rawKind);
  if (powerToughnessLabel) return powerToughnessLabel;

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

const COUNTER_PALETTES = {
  lore: { accent: "#e7bd67", holder: "#765015", fill: "#53350b", stroke: "#ffe4a4" },
  loyalty: { accent: "#ff8f3d", holder: "#853b0e", fill: "#5e2208", stroke: "#ffd0a1" },
  charge: { accent: "#35a8ff", holder: "#14558e", fill: "#0d2e56", stroke: "#a9e4ff" },
  energy: { accent: "#29d4c1", holder: "#126c6c", fill: "#0a3e4a", stroke: "#a8fff4" },
  time: { accent: "#8b7cff", holder: "#3e3989", fill: "#24245c", stroke: "#d2ccff" },
  shield: { accent: "#40d59a", holder: "#126d55", fill: "#0b423b", stroke: "#b7ffe5" },
  stun: { accent: "#ff655c", holder: "#84252a", fill: "#53151d", stroke: "#ffc5c3" },
  poison: { accent: "#8bdd3f", holder: "#47721d", fill: "#29440d", stroke: "#ddffad" },
  vigilance: { accent: "#d5e6f2", holder: "#4d677a", fill: "#293d4d", stroke: "#f5fbff" },
  finality: { accent: "#bd70ff", holder: "#603082", fill: "#37164f", stroke: "#f0ccff" },
  doom: { accent: "#d94862", holder: "#762034", fill: "#470f24", stroke: "#ffb4c2" },
  flame: { accent: "#ff733b", holder: "#8e2f1e", fill: "#57170f", stroke: "#ffc0a6" },
  flood: { accent: "#4e80ff", holder: "#274d9a", fill: "#182b62", stroke: "#c4d5ff" },
  fungus: { accent: "#c1d94b", holder: "#65751b", fill: "#3d470d", stroke: "#f1ffad" },
  gold: { accent: "#ffd34f", holder: "#87651a", fill: "#543d0c", stroke: "#fff1a8" },
  echo: { accent: "#a789ff", holder: "#57419d", fill: "#302260", stroke: "#e1d6ff" },
  ki: { accent: "#ff79b9", holder: "#82345d", fill: "#531a3e", stroke: "#ffc8e4" },
  mining: { accent: "#aebfd4", holder: "#4c5c73", fill: "#293545", stroke: "#e7f1ff" },
  muster: { accent: "#27bc67", holder: "#17673b", fill: "#0d422a", stroke: "#b8ffd6" },
  paw: { accent: "#e8ae6f", holder: "#76502d", fill: "#4d2b17", stroke: "#ffdbb0" },
  pin: { accent: "#f086d0", holder: "#78316a", fill: "#501b4b", stroke: "#ffd0f1" },
  scream: { accent: "#ff4fb5", holder: "#7d1d57", fill: "#4d1039", stroke: "#ffbde5" },
  skeleton: { accent: "#d0d7e2", holder: "#5c6675", fill: "#343b48", stroke: "#f6f9ff" },
  skull: { accent: "#d07cff", holder: "#553074", fill: "#321847", stroke: "#f1caff" },
  slime: { accent: "#d1ee35", holder: "#628016", fill: "#37480b", stroke: "#f3ff9b" },
  verse: { accent: "#62a8ff", holder: "#315c9d", fill: "#1c3567", stroke: "#d3e7ff" },
  vortex: { accent: "#4ce0ed", holder: "#1c7581", fill: "#0d4c57", stroke: "#c1fbff" },
  brick: { accent: "#d68858", holder: "#713b24", fill: "#482215", stroke: "#ffd0b4" },
  arrow: { accent: "#b1c5db", holder: "#405b78", fill: "#243a56", stroke: "#e0efff" },
};

const FALLBACK_COUNTER_PALETTES = [
  { accent: "#d2a5ff", holder: "#5b3e87", fill: "#382459", stroke: "#f0ddff" },
  { accent: "#ffac8b", holder: "#844527", fill: "#572b19", stroke: "#ffe0d3" },
  { accent: "#8ed7c8", holder: "#2e6a63", fill: "#1e4946", stroke: "#d3fff5" },
  { accent: "#c3dc83", holder: "#526820", fill: "#354414", stroke: "#f0ffd0" },
  { accent: "#e7a7c1", holder: "#763d59", fill: "#4f263c", stroke: "#ffddec" },
];

function counterPalette(rawKind) {
  const displayLabel = counterDisplayLabel(rawKind);
  if (displayLabel?.startsWith("+")) {
    return { accent: "#59d7b9", holder: "#1e7565", fill: "#123f3a", stroke: "#bcfff0" };
  }
  if (displayLabel?.startsWith("-")) {
    return { accent: "#c69ab7", holder: "#6b405a", fill: "#45293b", stroke: "#f3d1df" };
  }
  const normalizedKind = String(rawKind || "")
    .trim()
    .toLowerCase()
    .replaceAll("_", " ")
    .replace(/\s+/g, " ");
  if (COUNTER_PALETTES[normalizedKind]) return COUNTER_PALETTES[normalizedKind];

  let hash = 0;
  for (const character of normalizedKind) {
    hash = (hash * 31 + character.charCodeAt(0)) | 0;
  }
  return FALLBACK_COUNTER_PALETTES[Math.abs(hash) % FALLBACK_COUNTER_PALETTES.length];
}

function counterShapeClass(rawKind, variant) {
  const key = String(rawKind || "")
    .trim()
    .toLowerCase()
    .replaceAll("_", " ")
    .replace(/\s+/g, " ");
  if (["charge", "time", "loyalty", "lore"].includes(key)) return key;
  return variant === "power-toughness" ? "power-toughness" : "standard";
}

function counterIconClass(rawKind) {
  const key = String(rawKind || "")
    .trim()
    .toLowerCase()
    .replaceAll("_", " ")
    .replace(/\s+/g, " ");
  return ["lore", "time", "charge"].includes(key)
    ? `battlefield-counter-icon--${key}`
    : "";
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

export function resolveCardCounters(rawCounters, counterSignature) {
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

export function buildCounterBadge(counter) {
  const amount = Number(counter?.amount);
  const rawKind = String(counter?.kind || "").trim();
  if (!rawKind || !Number.isFinite(amount) || amount <= 0) return null;

  const powerToughnessLabel = counterDisplayLabel(rawKind);
  const normalizedKind = rawKind.toLowerCase().replaceAll("_", " ").replace(/\s+/g, " ");
  const variant = normalizedKind === "loyalty"
    ? "loyalty"
    : powerToughnessLabel
      ? "power-toughness"
      : "standard";
  if (powerToughnessLabel === "+1/+1") {
    return {
      amount,
      fullLabel: `${amount} +1/+1 counter${amount === 1 ? "" : "s"}`,
      displayLabel: "+1/+1",
      shortLabel: "+1",
      variant,
      shapeClassName: counterShapeClass(rawKind, variant),
      iconClassName: counterIconClass(rawKind),
      palette: counterPalette(rawKind),
      icon: counterSymbolUrl(rawKind),
    };
  }
  if (powerToughnessLabel === "-1/-1") {
    return {
      amount,
      fullLabel: `${amount} -1/-1 counter${amount === 1 ? "" : "s"}`,
      displayLabel: "-1/-1",
      shortLabel: "-1",
      variant,
      shapeClassName: counterShapeClass(rawKind, variant),
      iconClassName: counterIconClass(rawKind),
      palette: counterPalette(rawKind),
      icon: counterSymbolUrl(rawKind),
    };
  }

  const displayLabel = powerToughnessLabel || rawKind;
  return {
    amount,
    fullLabel: `${amount} ${displayLabel.toLowerCase()} counter${amount === 1 ? "" : "s"}`,
    displayLabel,
    shortLabel: powerToughnessLabel || abbreviateCounterKind(rawKind),
    variant,
    shapeClassName: counterShapeClass(rawKind, variant),
    iconClassName: counterIconClass(rawKind),
    palette: counterPalette(displayLabel),
    icon: counterSymbolUrl(rawKind),
  };
}

export function BattlefieldCounterBadge({ badge }) {
  const ui = useUiText();
  const frameId = useId().replaceAll(":", "");
  const palette = badge.palette || {
    accent: "#a7c3e7",
    holder: "#2a466c",
    fill: "#1b2d49",
    stroke: "#dcecff",
  };
  const amountLabel = badge.amount > 99 ? "99+" : String(badge.amount);
  const isPowerToughnessCounter = badge.variant === "power-toughness";
  const counterFrame = (
    <svg
      className="battlefield-counter-frame"
      viewBox="0 0 76 40"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <defs>
        <linearGradient id={`${frameId}-base`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor={palette.holder || palette.fill} />
          <stop offset="0.48" stopColor={palette.fill} />
          <stop offset="1" stopColor="#080f1b" />
        </linearGradient>
        <radialGradient id={`${frameId}-light`} cx="0.28" cy="0.5" r="0.72">
          <stop offset="0" stopColor={palette.accent} stopOpacity="0.9" />
          <stop offset="0.36" stopColor={palette.holder || palette.fill} stopOpacity="0.72" />
          <stop offset="0.72" stopColor={palette.fill} stopOpacity="0.18" />
          <stop offset="1" stopColor="#080f1b" stopOpacity="0" />
        </radialGradient>
        <filter id={`${frameId}-glow`} x="-20%" y="-30%" width="140%" height="160%">
          <feGaussianBlur stdDeviation="0.65" result="counter-blur" />
          <feFlood floodColor={palette.accent} floodOpacity="0.24" />
          <feComposite in2="counter-blur" operator="in" />
          <feMerge>
            <feMergeNode />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      <path
        className="battlefield-counter-frame__body"
        d="M20 1.5C9.5 1.5 1.5 9.6 1.5 20S9.5 38.5 20 38.5h43.5c6.3 0 11-4.7 11-11V12.5c0-6.3-4.7-11-11-11Z"
        fill={`url(#${frameId}-base)`}
        stroke={palette.fill}
        strokeWidth="3"
        strokeLinejoin="round"
        filter={`url(#${frameId}-glow)`}
      />
      <path
        d="M20 2.5C10.1 2.5 2.5 10.2 2.5 20S10.1 37.5 20 37.5h43.5c5.6 0 10-4.4 10-10V12.5c0-5.6-4.4-10-10-10Z"
        fill={`url(#${frameId}-light)`}
        opacity="0.9"
      />
      <path
        d="M20 2.5C10.1 2.5 2.5 10.2 2.5 20S10.1 37.5 20 37.5h43.5c5.6 0 10-4.4 10-10V12.5c0-5.6-4.4-10-10-10Z"
        fill="none"
        stroke={palette.stroke}
        strokeOpacity="0.82"
        strokeWidth="1"
      />
      <path
        d="M10 9.5c3.2-4 7.2-5.8 12.5-5.8h40"
        fill="none"
        stroke="#e8f9ff"
        strokeOpacity="0.3"
        strokeWidth="1"
        strokeLinecap="round"
      />
    </svg>
  );
  const icon = isPowerToughnessCounter ? (
    <span
      className="battlefield-counter-symbol battlefield-counter-symbol--power"
      aria-hidden="true"
    >
      {badge.displayLabel?.startsWith("-") ? "-" : "+"}
    </span>
  ) : badge.icon ? (
    <img
      className={`battlefield-counter-icon ${badge.iconClassName || ""}`.trim()}
      src={badge.icon}
      alt=""
      aria-hidden="true"
    />
  ) : (
    <span className="battlefield-counter-symbol" aria-hidden="true">◇</span>
  );

  return (
    <span
      className={`battlefield-counter-chip battlefield-counter-chip--${badge.variant || "standard"} battlefield-counter-chip--shape-${badge.shapeClassName || "standard"}`}
      title={ui(badge.fullLabel)}
      role="img"
      aria-label={ui(badge.fullLabel)}
      style={{
        "--counter-accent": badge.palette.accent,
        "--counter-holder": badge.palette.holder || badge.palette.fill,
        "--counter-fill": badge.palette.fill,
        "--counter-stroke": badge.palette.stroke,
      }}
    >
      {counterFrame}
      {badge.variant === "loyalty" ? (
        <>
          {icon}
          <span className="battlefield-counter-amount">{ui(amountLabel)}</span>
        </>
      ) : badge.variant === "power-toughness" ? (
        <>
          {icon}
          <span className="battlefield-counter-amount">{ui(amountLabel)}</span>
        </>
      ) : badge.shapeClassName === "charge" || badge.shapeClassName === "time" ? (
        <>
          {icon}
          <span className="battlefield-counter-amount">{ui(amountLabel)}</span>
        </>
      ) : (
        <>
          {icon}
          <span className="battlefield-counter-amount">{ui(amountLabel)}</span>
        </>
      )}
    </span>
  );
}
