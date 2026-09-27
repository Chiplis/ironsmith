/* Shared counter helpers are intentionally imported by both card renderers. */
/* eslint-disable react-refresh/only-export-components */
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

function counterPalette(rawKind) {
  const displayLabel = counterDisplayLabel(rawKind);
  if (displayLabel?.startsWith("+")) {
    return { accent: "#67d79a", holder: "#236e46", fill: "#164c2f", stroke: "#aef0ca" };
  }
  if (displayLabel?.startsWith("-")) {
    return { accent: "#df6d83", holder: "#7a2339", fill: "#551626", stroke: "#ffb0c1" };
  }
  switch (rawKind) {
    case "Plus One Plus One":
      return { accent: "#67d79a", holder: "#236e46", fill: "#164c2f", stroke: "#aef0ca" };
    case "Minus One Minus One":
      return { accent: "#df6d83", holder: "#7a2339", fill: "#551626", stroke: "#ffb0c1" };
    case "Lore":
      return { accent: "#e1bd73", holder: "#765015", fill: "#5c3e0f", stroke: "#f8dba2" };
    case "Loyalty":
      return { accent: "#f1b561", holder: "#85400e", fill: "#612e09", stroke: "#ffd7a2" };
    case "Charge":
      return { accent: "#6bc2ff", holder: "#1b5688", fill: "#12355b", stroke: "#bbebff" };
    case "Time":
      return { accent: "#6cb8ff", holder: "#1f5b91", fill: "#123b63", stroke: "#c2e7ff" };
    case "Shield":
      return { accent: "#84d6cf", holder: "#185e58", fill: "#123f3b", stroke: "#c5f7ef" };
    case "Stun":
      return { accent: "#f2a464", holder: "#7d3c11", fill: "#5d280b", stroke: "#ffd2a1" };
    case "Vigilance":
      return { accent: "#b7df9f", holder: "#3d5f1e", fill: "#293e14", stroke: "#ebffd6" };
    case "Finality":
      return { accent: "#b48fff", holder: "#4e3785", fill: "#302052", stroke: "#ddd0ff" };
    default:
      return { accent: "#a7c3e7", holder: "#2a466c", fill: "#1b2d49", stroke: "#dcecff" };
  }
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
  const amountLabel = badge.amount > 99 ? "99+" : String(badge.amount);
  const chargeFrame = badge.shapeClassName === "charge" ? (
    <svg
      className="battlefield-counter-charge-frame"
      viewBox="0 0 76 40"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="charge-frame-base" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#0d4a7c" />
          <stop offset="0.48" stopColor="#0b3158" />
          <stop offset="1" stopColor="#061629" />
        </linearGradient>
        <radialGradient id="charge-frame-light" cx="0.28" cy="0.5" r="0.72">
          <stop offset="0" stopColor="#2e91dc" stopOpacity="0.96" />
          <stop offset="0.36" stopColor="#1765a8" stopOpacity="0.82" />
          <stop offset="0.72" stopColor="#0d3e6e" stopOpacity="0.22" />
          <stop offset="1" stopColor="#061629" stopOpacity="0" />
        </radialGradient>
        <filter id="charge-frame-glow" x="-20%" y="-30%" width="140%" height="160%">
          <feGaussianBlur stdDeviation="0.8" result="charge-blur" />
          <feFlood floodColor="#55bfff" floodOpacity="0.34" />
          <feComposite in2="charge-blur" operator="in" />
          <feMerge>
            <feMergeNode />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      <path
        className="battlefield-counter-charge-frame__body"
        d="M20 1.5C9.5 1.5 1.5 9.6 1.5 20S9.5 38.5 20 38.5h43.5c6.3 0 11-4.7 11-11V12.5c0-6.3-4.7-11-11-11Z"
        fill="url(#charge-frame-base)"
        stroke="#06111d"
        strokeWidth="3"
        strokeLinejoin="round"
        filter="url(#charge-frame-glow)"
      />
      <path
        d="M20 2.5C10.1 2.5 2.5 10.2 2.5 20S10.1 37.5 20 37.5h43.5c5.6 0 10-4.4 10-10V12.5c0-5.6-4.4-10-10-10Z"
        fill="url(#charge-frame-light)"
        opacity="0.9"
      />
      <path
        d="M20 2.5C10.1 2.5 2.5 10.2 2.5 20S10.1 37.5 20 37.5h43.5c5.6 0 10-4.4 10-10V12.5c0-5.6-4.4-10-10-10Z"
        fill="none"
        stroke="var(--counter-stroke, #bbebff)"
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
  ) : null;
  const icon = badge.icon ? (
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
      {chargeFrame}
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
