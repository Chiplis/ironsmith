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
  lore: { accent: "#e1bd73", holder: "#765015", fill: "#5c3e0f", stroke: "#f8dba2" },
  loyalty: { accent: "#f1b561", holder: "#85400e", fill: "#612e09", stroke: "#ffd7a2" },
  charge: { accent: "#6bc2ff", holder: "#1b5688", fill: "#12355b", stroke: "#bbebff" },
  energy: { accent: "#6bc2ff", holder: "#1b5688", fill: "#12355b", stroke: "#bbebff" },
  time: { accent: "#6cb8ff", holder: "#1f5b91", fill: "#123b63", stroke: "#c2e7ff" },
  shield: { accent: "#84d6cf", holder: "#185e58", fill: "#123f3b", stroke: "#c5f7ef" },
  stun: { accent: "#f2a464", holder: "#7d3c11", fill: "#5d280b", stroke: "#ffd2a1" },
  poison: { accent: "#b7df9f", holder: "#3d5f1e", fill: "#293e14", stroke: "#ebffd6" },
  vigilance: { accent: "#b7df9f", holder: "#3d5f1e", fill: "#293e14", stroke: "#ebffd6" },
  finality: { accent: "#b48fff", holder: "#4e3785", fill: "#302052", stroke: "#ddd0ff" },
  doom: { accent: "#e77c80", holder: "#76292e", fill: "#4c171d", stroke: "#ffc5c8" },
  flame: { accent: "#ff9c70", holder: "#8d351e", fill: "#5c2014", stroke: "#ffd0bb" },
  flood: { accent: "#75c9ff", holder: "#1c5a89", fill: "#123c63", stroke: "#c4ebff" },
  fungus: { accent: "#a9da83", holder: "#3d6b28", fill: "#29451c", stroke: "#e1ffc8" },
  gold: { accent: "#f0cf70", holder: "#80651b", fill: "#55420f", stroke: "#ffefb0" },
  echo: { accent: "#c4a4ff", holder: "#533c8e", fill: "#33215c", stroke: "#e4d7ff" },
  ki: { accent: "#ffbc76", holder: "#88501c", fill: "#5d3310", stroke: "#ffe0b8" },
  mining: { accent: "#b7c8d8", holder: "#4b6174", fill: "#2d3f50", stroke: "#e4f0fb" },
  muster: { accent: "#b5df9b", holder: "#3f6726", fill: "#2a4519", stroke: "#e4ffd2" },
  paw: { accent: "#e8c190", holder: "#76522d", fill: "#503418", stroke: "#ffe4c1" },
  pin: { accent: "#e69ad2", holder: "#733f6c", fill: "#4e2850", stroke: "#ffd7f3" },
  scream: { accent: "#f28cc0", holder: "#7a315e", fill: "#531d3e", stroke: "#ffd1e8" },
  skeleton: { accent: "#c7c9d3", holder: "#5d5e6b", fill: "#3b3c48", stroke: "#f1f2f8" },
  skull: { accent: "#c5a5ff", holder: "#523b7b", fill: "#30214f", stroke: "#e4d8ff" },
  slime: { accent: "#8dda98", holder: "#2f702f", fill: "#1f4a23", stroke: "#d0ffd5" },
  verse: { accent: "#9ebcff", holder: "#3f568f", fill: "#293a68", stroke: "#dbe6ff" },
  vortex: { accent: "#8de3ef", holder: "#246b78", fill: "#164b5b", stroke: "#d2fbff" },
  brick: { accent: "#d69a74", holder: "#704127", fill: "#4a2919", stroke: "#ffdbc6" },
  arrow: { accent: "#c2d2e8", holder: "#4b6283", fill: "#2d405e", stroke: "#e3efff" },
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
    return { accent: "#e7c76d", holder: "#876a1e", fill: "#5b4510", stroke: "#ffe8a8" };
  }
  if (displayLabel?.startsWith("-")) {
    return { accent: "#e59a5a", holder: "#8a471c", fill: "#642f12", stroke: "#ffd0a0" };
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
