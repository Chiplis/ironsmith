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
  if (counterDisplayLabel(rawKind)?.startsWith("+")) {
    return { accent: "#67d79a", fill: "#164c2f", stroke: "#aef0ca" };
  }
  if (counterDisplayLabel(rawKind)?.startsWith("-")) {
    return { accent: "#df6d83", fill: "#551626", stroke: "#ffb0c1" };
  }
  switch (rawKind) {
    case "Plus One Plus One":
      return { accent: "#67d79a", fill: "#164c2f", stroke: "#aef0ca" };
    case "Minus One Minus One":
      return { accent: "#df6d83", fill: "#551626", stroke: "#ffb0c1" };
    case "Lore":
      return { accent: "#e1bd73", fill: "#5c3e0f", stroke: "#f8dba2" };
    case "Loyalty":
      return { accent: "#f1b561", fill: "#612e09", stroke: "#ffd7a2" };
    case "Charge":
      return { accent: "#6bc2ff", fill: "#12355b", stroke: "#bbebff" };
    case "Shield":
      return { accent: "#84d6cf", fill: "#123f3b", stroke: "#c5f7ef" };
    case "Stun":
      return { accent: "#f2a464", fill: "#5d280b", stroke: "#ffd2a1" };
    case "Vigilance":
      return { accent: "#b7df9f", fill: "#293e14", stroke: "#ebffd6" };
    case "Finality":
      return { accent: "#b48fff", fill: "#302052", stroke: "#ddd0ff" };
    default:
      return { accent: "#a7c3e7", fill: "#1b2d49", stroke: "#dcecff" };
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
    palette: counterPalette(displayLabel),
    icon: counterSymbolUrl(rawKind),
  };
}

export function BattlefieldCounterBadge({ badge }) {
  const ui = useUiText();
  const amountLabel = badge.amount > 99 ? "99+" : String(badge.amount);
  const icon = badge.icon ? (
    <img className="battlefield-counter-icon" src={badge.icon} alt="" aria-hidden="true" />
  ) : (
    <span className="battlefield-counter-symbol" aria-hidden="true">◇</span>
  );

  return (
    <span
      className={`battlefield-counter-chip battlefield-counter-chip--${badge.variant || "standard"}`}
      title={ui(badge.fullLabel)}
      role="img"
      aria-label={ui(badge.fullLabel)}
      style={{
        "--counter-accent": badge.palette.accent,
        "--counter-fill": badge.palette.fill,
        "--counter-stroke": badge.palette.stroke,
      }}
    >
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
      ) : (
        <>
          <span className="battlefield-counter-amount">{ui(amountLabel)}</span>
          {icon}
        </>
      )}
    </span>
  );
}
