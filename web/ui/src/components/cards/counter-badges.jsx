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
  experience: { accent: "#8bb8ff", holder: "#31568f", fill: "#1d315b", stroke: "#d0e4ff" },
  age: { accent: "#d995ff", holder: "#633f8f", fill: "#3a215f", stroke: "#efd6ff" },
  depletion: { accent: "#a7b9c9", holder: "#465c70", fill: "#293847", stroke: "#e0edf5" },
  fate: { accent: "#ff9e78", holder: "#804735", fill: "#54271e", stroke: "#ffd7c8" },
  quest: { accent: "#ffc45d", holder: "#86601d", fill: "#52380d", stroke: "#ffecb4" },
  level: { accent: "#7ee4ff", holder: "#236b85", fill: "#123e51", stroke: "#c9f7ff" },
  corruption: { accent: "#ef72c4", holder: "#792b64", fill: "#4a1741", stroke: "#ffd0ef" },
  oil: { accent: "#b8a15c", holder: "#635525", fill: "#3d3414", stroke: "#eee0a2" },
  aim: { accent: "#ff8978", holder: "#7e382f", fill: "#4f201c", stroke: "#ffd1c9" },
  brain: { accent: "#c39bff", holder: "#5b4292", fill: "#34245f", stroke: "#ebddff" },
  net: { accent: "#67ddd2", holder: "#26746f", fill: "#164a49", stroke: "#c2fff8" },
};

const FALLBACK_COUNTER_PALETTES = [
  { accent: "#d2a5ff", holder: "#5b3e87", fill: "#382459", stroke: "#f0ddff" },
  { accent: "#ffac8b", holder: "#844527", fill: "#572b19", stroke: "#ffe0d3" },
  { accent: "#8ed7c8", holder: "#2e6a63", fill: "#1e4946", stroke: "#d3fff5" },
  { accent: "#c3dc83", holder: "#526820", fill: "#354414", stroke: "#f0ffd0" },
  { accent: "#e7a7c1", holder: "#763d59", fill: "#4f263c", stroke: "#ffddec" },
  { accent: "#78b8ff", holder: "#315d9b", fill: "#1e3566", stroke: "#d0e6ff" },
  { accent: "#f3c45f", holder: "#87621c", fill: "#523b0d", stroke: "#ffefb6" },
  { accent: "#6fe5e4", holder: "#216c70", fill: "#12454b", stroke: "#c6ffff" },
  { accent: "#ff7d8f", holder: "#813344", fill: "#511e2c", stroke: "#ffd0d8" },
  { accent: "#b9a0ff", holder: "#56428e", fill: "#33255d", stroke: "#e8ddff" },
  { accent: "#9be06f", holder: "#4b762a", fill: "#2c4818", stroke: "#e4ffc7" },
  { accent: "#eaa66b", holder: "#7a4927", fill: "#4a2918", stroke: "#ffdbc0" },
];

const POSITIVE_POWER_PALETTE = {
  // Positive P/T counters use a vivid aqua/cyan family so they read as
  // additive at a glance without blending into the old violet badges.
  accent: "#48f0dc",
  holder: "#168e86",
  fill: "#074b51",
  stroke: "#c5fff7",
};

const NEGATIVE_POWER_PALETTE = {
  accent: "#ff6f91",
  holder: "#9e2e4a",
  fill: "#5c162f",
  stroke: "#ffd4de",
};

function powerToughnessPalette(rawKind) {
  const label = counterDisplayLabel(rawKind);
  if (!label) return null;
  const match = label.match(/^([+-])(\d+)\/([+-])(\d+)$/);
  if (!match) return null;
  const [, powerSign, , toughnessSign] = match;
  return powerSign === "+" && toughnessSign === "+"
    ? POSITIVE_POWER_PALETTE
    : NEGATIVE_POWER_PALETTE;
}

function counterPalette(rawKind) {
  const powerToughnessPaletteValue = powerToughnessPalette(rawKind);
  if (powerToughnessPaletteValue) return powerToughnessPaletteValue;

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

function parsePowerToughnessContribution(counter) {
  const label = counterDisplayLabel(counter?.kind);
  if (!label) return null;
  const match = label.match(/^([+-])(\d+)\/([+-])(\d+)$/);
  if (!match) return null;
  const [, powerSign, powerMagnitude, toughnessSign, toughnessMagnitude] = match;
  const amount = Number(counter.amount) || 0;
  const power = Number(powerMagnitude) * (powerSign === "-" ? -1 : 1) * amount;
  const toughness = Number(toughnessMagnitude) * (toughnessSign === "-" ? -1 : 1) * amount;
  return { label, power, toughness, amount };
}

function formatSignedCounterValue(value) {
  const numeric = Number(value) || 0;
  return numeric > 0 ? `+${numeric}` : String(numeric);
}

function compactPowerToughnessLabel(label) {
  const match = String(label || "").match(/^([+-])(\d+)\/([+-])(\d+)$/);
  if (!match || match[1] !== match[3]) return label;
  return `${match[1]}${match[2]}/${match[4]}`;
}

function formatNetPowerToughness(power, toughness) {
  const powerValue = Number(power) || 0;
  const toughnessValue = Number(toughness) || 0;
  if (powerValue >= 0 && toughnessValue >= 0 && (powerValue > 0 || toughnessValue > 0)) {
    return `+${powerValue}/${toughnessValue}`;
  }
  if (powerValue <= 0 && toughnessValue <= 0 && (powerValue < 0 || toughnessValue < 0)) {
    return `-${Math.abs(powerValue)}/${Math.abs(toughnessValue)}`;
  }
  return `${formatSignedCounterValue(powerValue)}/${formatSignedCounterValue(toughnessValue)}`;
}

function formatCounterBreakdown(counters) {
  return counters
    .map((counter) => {
      const label = counterDisplayLabel(counter.kind) || counter.kind;
      return `${counter.amount} ${label}`;
    })
    .join("\n");
}

/**
 * Collapse all numeric P/T counter kinds into one net fraction while keeping
 * every source counter available for the hover description. Non-P/T counters
 * remain separate, so a card can show both its net stats and keyword counters.
 */
export function aggregateCounterEntries(counters) {
  if (!Array.isArray(counters) || counters.length === 0) return counters || [];

  const numericEntries = [];
  let power = 0;
  let toughness = 0;
  let sourceCount = 0;
  let firstNumericIndex = -1;

  counters.forEach((counter, index) => {
    const contribution = parsePowerToughnessContribution(counter);
    if (!contribution) return;
    if (firstNumericIndex < 0) firstNumericIndex = index;
    numericEntries.push(counter);
    power += contribution.power;
    toughness += contribution.toughness;
    sourceCount += contribution.amount;
  });

  if (numericEntries.length === 0) return counters;

  const netLabel = formatNetPowerToughness(power, toughness);
  const aggregate = {
    kind: "__power_toughness_total__",
    amount: 1,
    sourceCount,
    displayLabel: netLabel,
    paletteKind: power < 0 || toughness < 0 ? "-1/-1" : "+1/+1",
    isPowerToughness: true,
    breakdown: formatCounterBreakdown(numericEntries),
  };

  const result = [];
  let inserted = false;
  counters.forEach((counter, index) => {
    if (index === firstNumericIndex) {
      result.push(aggregate);
      inserted = true;
    }
    if (!parsePowerToughnessContribution(counter)) result.push(counter);
  });
  if (!inserted) result.push(aggregate);
  return result;
}

export function buildCounterBadge(counter) {
  const amount = Number(counter?.amount);
  const rawKind = String(counter?.kind || "").trim();
  if (!rawKind || !Number.isFinite(amount) || amount <= 0) return null;

  const displayLabelOverride = String(counter?.displayLabel || "").trim();
  const powerToughnessLabel = displayLabelOverride || counterDisplayLabel(rawKind);
  const normalizedKind = rawKind.toLowerCase().replaceAll("_", " ").replace(/\s+/g, " ");
  const variant = counter?.isPowerToughness === true || powerToughnessLabel
    ? "power-toughness"
    : normalizedKind === "loyalty"
    ? "loyalty"
    : "standard";
  const displayLabel = powerToughnessLabel
    ? compactPowerToughnessLabel(powerToughnessLabel)
    : rawKind;
  const isPowerToughness = variant === "power-toughness";
  return {
    amount,
    sourceCount: Number(counter?.sourceCount) > 0 ? Number(counter.sourceCount) : amount,
    fullLabel: counter?.breakdown
      || `${amount} ${(powerToughnessLabel || displayLabel).toLowerCase()} counter${amount === 1 ? "" : "s"}`,
    displayLabel,
    shortLabel: isPowerToughness ? displayLabel : abbreviateCounterKind(rawKind),
    variant,
    kind: rawKind,
    shapeClassName: counterShapeClass(rawKind, variant),
    iconClassName: counterIconClass(rawKind),
    palette: counterPalette(counter?.paletteKind || displayLabel),
    icon: counterSymbolUrl(rawKind),
  };
}

function counterGlyphFamily(rawKind) {
  const key = String(rawKind || "")
    .trim()
    .toLowerCase()
    .replaceAll("_", " ")
    .replace(/\s+/g, " ");
  const has = (...terms) => terms.some((term) => key.includes(term));

  // Keep the common keyword counters visually distinct before falling back to
  // broader families. These are intentionally tiny, single-purpose marks so
  // the showcase remains legible without shipping an icon library.
  if (has("double strike")) return "double-sword";
  if (has("first strike")) return "sword";
  if (has("deathtouch")) return "skull";
  if (has("decayed")) return "decay";
  if (has("haste", "hast")) return "comet";
  if (has("hexproof")) return "eye-slash";
  if (has("indestructible")) return "shield";
  if (has("lifelink")) return "heart";
  if (has("menace")) return "fangs";
  if (has("reach")) return "hand";
  if (has("trample", "hoofprint", "prey")) return "foot";
  if (has("vigilance")) return "watch";
  if (has("blood")) return "blood";
  if (has("aim")) return "crosshair";
  if (has("bounty")) return "target";
  if (has("credit", "currency", "wage")) return "coin";
  if (has("crystal", "gem")) return "crystal";
  if (has("cube", "matrix", "storage")) return "cube";
  if (has("depletion")) return "battery";
  if (has("devotion", "divinity")) return "halo";
  if (has("dream")) return "cloud";
  if (has("eon")) return "infinity";
  if (has("eyeball", "omen", "foreshadow")) return "eye";
  if (has("fate", "wish", "luck")) return "clover";
  if (has("feather", "flying")) return "wing";
  if (has("hunger")) return "fangs";
  if (has("ice")) return "snowflake";
  if (has("ki")) return "spark";
  if (has("magnet")) return "magnet";
  if (has("music")) return "note";
  if (has("oil")) return "droplet";
  if (has("ore")) return "pick";
  if (has("pain")) return "exclaim";
  if (has("petrification")) return "stone";
  if (has("finality")) return "lock";
  if (has("glyph")) return "rune";
  if (has("javelin")) return "arrow";
  if (has("pin")) return "pin";
  if (has("pressure")) return "gauge";
  if (has("rad")) return "radiation";
  if (has("soul", "unity")) return "halo";
  if (has("stun")) return "exclaim";
  if (has("theft")) return "hand";
  if (has("tower")) return "tower";
  if (has("training")) return "bars";
  if (has("quest")) return "flag";
  if (has("task")) return "gear";
  if (has("trap")) return "target";
  if (has("void")) return "vortex";
  if (has("volatile")) return "flame";

  if (has("charge", "energy", "velocity", "fuse", "paralyzation")) return "bolt";
  if (has("time", "hour", "eon", "age", "depletion", "fade")) return "clock";
  if (has("night", "sleep", "slumber")) return "moon";
  if (has("loyalty", "shield", "defense", "hexproof", "indestructible", "vigilance", "isolation")) return "shield";
  if (has("lore", "page", "knowledge", "study", "verse", "plot", "glyph", "keyword")) return "book";
  if (has("rad")) return "radiation";
  if (has("poison", "infection", "blood", "oil", "mire", "slime")) return "droplet";
  if (has("flame", "soot")) return "flame";
  if (has("flood", "tide", "ice", "music")) return "wave";
  if (has("fungus", "growth", "petal", "spore")) return "leaf";
  if (has("hatchling", "pupa", "egg", "polyp")) return "egg";
  if (has("gold", "treasure", "currency", "credit", "silver", "ore", "gem", "crystal", "wage", "luck")) return "coin";
  if (has("double strike", "first strike", "javelin", "strife", "hit")) return "sword";
  if (has("arrow")) return "arrow";
  if (has("flying", "feather", "wind", "voyage")) return "wing";
  if (has("lifelink", "healing", "vitality")) return "heart";
  if (has("menace", "eyeball")) return "eye";
  if (has("reach", "trample", "hoofprint", "paw", "prey")) return "foot";
  if (has("brain")) return "brain";
  if (has("mine", "mining")) return "pick";
  if (has("brick")) return "blocks";
  if (has("manifestation", "incarnation", "mannequin")) return "person";
  if (has("muster", "filibuster")) return "flag";
  if (has("music")) return "note";
  if (has("winch")) return "gear";
  if (has("awakening", "intervention")) return "sunrise";
  if (has("phylactery")) return "lock";
  if (has("scream", "despair", "pain")) return "exclaim";
  if (has("deathtouch", "decayed", "corpse", "death", "doom", "skeleton", "skull", "void", "plague", "pain")) return "skull";
  if (has("echo", "dream", "fate", "omen", "foreshadow", "enlightened", "devotion", "divinity", "soul", "unity")) return "spark";
  if (has("level", "experience", "quest", "tower", "storage", "cube", "matrix", "mannequin", "mine", "mining", "brick")) return "bars";
  if (has("pin", "net", "trap", "theft", "bounty", "pressure", "magnet")) return "target";
  return "generic";
}

function CounterKindGlyph({ kind }) {
  const family = counterGlyphFamily(kind);
  const glyphs = {
    bolt: <path d="M13.5 1.5 5 13h5.2l-1 9.5L18.8 10h-5.1l-.2-8.5Z" />,
    comet: <><path d="m14 4 6 6-8.5 8.5-6-6Z" /><path d="M5 19 3 21M7 16l-3 1M9 13l-1-3" /></>,
    "double-sword": <><path d="m5 19 11-11M9 21 20 10" /><path d="m13 6 4-1 .9.9-1 4M17 8l3-1 .9.9-1 3" /></>,
    clock: <><circle cx="12" cy="12" r="8.5" /><path d="M12 7v5l3.5 2" /></>,
    watch: <><circle cx="12" cy="12" r="7.3" /><path d="M12 8v4l2.7 1.6M9 2h6M9 22h6" /></>,
    moon: <path d="M18.8 15.8A7.8 7.8 0 0 1 8.2 5.2 8.5 8.5 0 1 0 18.8 15.8Z" />,
    shield: <path d="m12 2.5 7 3v5.3c0 4.7-2.8 8.5-7 10.7-4.2-2.2-7-6-7-10.7V5.5l7-3Z" />,
    fangs: <><path d="M5 7c2.6 1.4 4.8 1.4 7 0 2.2 1.4 4.4 1.4 7 0" /><path d="m7 8 1 8 3-4 1 8 1-8 3 4 1-8" /></>,
    hand: <path d="M8 20v-7.5M8 13V7a1.4 1.4 0 0 1 2.8 0v5M10.8 12V5.5a1.4 1.4 0 0 1 2.8 0V12M13.6 12V7a1.4 1.4 0 0 1 2.8 0v6M16.4 13v-2a1.4 1.4 0 0 1 2.8 0v4.5c0 3.2-2 5.5-5.3 5.5H11c-1.8 0-3-1-3-3Z" />,
    book: <><path d="M4.5 4.5h5.2c1.3 0 2.3 1 2.3 2.3v12.7c-.6-.7-1.4-1-2.3-1H4.5Z" /><path d="M19.5 4.5h-5.2c-1.3 0-2.3 1-2.3 2.3v12.7c.6-.7 1.4-1 2.3-1h5.2Z" /></>,
    radiation: <><circle cx="12" cy="12" r="2.2" /><path d="M12 2.5a9.5 9.5 0 0 1 8.2 4.7l-5.1 2.9A3.6 3.6 0 0 0 12 8.4Z" /><path d="M20.2 16.8a9.5 9.5 0 0 1-8.2 4.7v-5.9a3.6 3.6 0 0 0 3.1-1.8Z" /><path d="M3.8 16.8a9.5 9.5 0 0 1 0-9.5l5.1 2.9A3.6 3.6 0 0 0 8.9 12c0 .7.2 1.3.5 1.8Z" /></>,
    droplet: <path d="M12 2.2S5.5 9.3 5.5 14.2a6.5 6.5 0 0 0 13 0C18.5 9.3 12 2.2 12 2.2Z" />,
    blood: <path d="M12 3.2S6.4 9.8 6.4 14.2a5.6 5.6 0 0 0 11.2 0C17.6 9.8 12 3.2 12 3.2ZM9.5 15.2a2.8 2.8 0 0 0 2.5 2" />,
    flame: <path d="M13.6 2.2c.7 3.1-.9 4.8-2.5 6.2-.8-1.4-1.8-2.4-1.5-4.5C6.8 6.8 5 9.3 5 13a7 7 0 0 0 14 0c0-3.7-2-6.1-5.4-10.8Z" />,
    wave: <path d="M3 9c2.4-3.2 4.8-3.2 7.2 0s4.8 3.2 7.2 0M3 14c2.4-3.2 4.8-3.2 7.2 0s4.8 3.2 7.2 0" />,
    leaf: <><path d="M20 4C11 4 5 7.5 5 13.5A6.5 6.5 0 0 0 11.5 20C17.5 20 20 12 20 4Z" /><path d="M5.5 18.5 14 10" /></>,
    decay: <><path d="M20 4C11 4 5 7.5 5 13.5A6.5 6.5 0 0 0 11.5 20C17.5 20 20 12 20 4Z" /><path d="m7 17 4-4 2 2 4-5" /></>,
    egg: <path d="M12 3.2c-3.6 0-6.5 5-6.5 9.8a6.5 6.5 0 0 0 13 0c0-4.8-2.9-9.8-6.5-9.8Z" />,
    coin: <><circle cx="12" cy="12" r="8.5" /><path d="M12 7v10M9.5 9.5h4a1.7 1.7 0 0 1 0 3.4h-3a1.7 1.7 0 0 0 0 3.4h4" /></>,
    crystal: <path d="m12 2.5 7.3 5.4-2.8 10.1H7.5L4.7 7.9 12 2.5Z" />,
    cube: <><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z" /><path d="M4.5 7.7 12 12l7.5-4.3M12 12v9" /></>,
    battery: <><rect x="4" y="7" width="15" height="10" rx="2" /><path d="M19 10h2v4h-2M8 10v4M11 10v4M14 10v4" /></>,
    halo: <><circle cx="12" cy="12" r="4" /><ellipse cx="12" cy="12" rx="9.5" ry="4.2" /><path d="M12 2.5v3M12 18.5v3" /></>,
    cloud: <path d="M6.5 18h10.8a3.8 3.8 0 0 0 .5-7.6A6.3 6.3 0 0 0 6 9.2 4.4 4.4 0 0 0 6.5 18Z" />,
    infinity: <path d="M4 12c0-3.3 4.1-4.2 6.4-1.7l3.2 3.4C16 16.2 20 15.3 20 12s-4.1-4.2-6.4-1.7l-3.2 3.4C8.1 16.2 4 15.3 4 12Z" />,
    clover: <><circle cx="9" cy="9" r="3.2" /><circle cx="15" cy="9" r="3.2" /><circle cx="9" cy="15" r="3.2" /><circle cx="15" cy="15" r="3.2" /><path d="m12 12 5 8" /></>,
    sword: <><path d="m6 18 11.8-11.8" /><path d="m14.5 5.5 4-1 .9.9-1 4" /><path d="m5 19 2-2M4 20l2-2" /></>,
    arrow: <><path d="M4 12h14" /><path d="m13 6 6 6-6 6" /></>,
    wing: <path d="M3 18c3.5-6.5 8-10.5 14.5-12.5-2.2 3.2-2.5 6.3-1.5 9.5-3.9-1.2-7.2-.2-10 3Z" />,
    heart: <path d="M12 20S4 15.6 4 9.7A4.2 4.2 0 0 1 12 8a4.2 4.2 0 0 1 8 1.7C20 15.6 12 20 12 20Z" />,
    eye: <><path d="M2.5 12s3.3-5 9.5-5 9.5 5 9.5 5-3.3 5-9.5 5-9.5-5-9.5-5Z" /><circle cx="12" cy="12" r="2.2" /></>,
    "eye-slash": <><path d="M2.5 12s3.3-5 9.5-5c2.1 0 4 .6 5.6 1.5M21.5 12s-3.3 5-9.5 5c-2.1 0-4-.6-5.6-1.5" /><circle cx="12" cy="12" r="2.2" /><path d="M4 4 20 20" /></>,
    foot: <path d="M9.6 4.5c1.6 3.7 2.7 6.2 4.5 8.2 1.7 1.9 4 2.1 4.8 3.7.7 1.4-.3 3.1-2.1 3.1-3.8 0-7.3-3.9-8.9-7.1C6.5 9.5 5.8 6 7.1 4.7c.6-.7 1.7-.8 2.5-.2Z" />,
    brain: <><path d="M9 5.1A3.1 3.1 0 0 0 5.2 8c-1.8.7-2.7 3.1-1.5 4.7-1 2.1.5 4.5 2.7 4.6A3.1 3.1 0 0 0 12 19V6.9a3.1 3.1 0 0 0-3-1.8Z" /><path d="M15 5.1A3.1 3.1 0 0 1 18.8 8c1.8.7 2.7 3.1 1.5 4.7 1 2.1-.5 4.5-2.7 4.6A3.1 3.1 0 0 1 12 19V6.9a3.1 3.1 0 0 1 3-1.8Z" /></>,
    pick: <><path d="m4 20 10.5-10.5" /><path d="m13 5 6 6M15 3l6 6" /></>,
    blocks: <><path d="M4 5h7v7H4zM13 5h7v7h-7zM4 14h7v6H4zM13 14h7v6h-7z" /></>,
    snowflake: <><path d="M12 3v18M4.2 7.5l15.6 9M4.2 16.5l15.6-9M12 3l2 2M12 3l-2 2M12 21l2-2M12 21l-2-2" /></>,
    magnet: <path d="M5 5v7a7 7 0 0 0 14 0V5h-4v7a3 3 0 0 1-6 0V5H5Z" />,
    stone: <path d="m7 4 9-1 4 5-2 11-10 1-4-6 1-7Z" />,
    gauge: <><path d="M4 17a8 8 0 1 1 16 0" /><path d="m12 13 4-4M6 17h2M16 17h2" /></>,
    tower: <><path d="M7 21V8h10v13M5 8h14M9 4h6M10 2h4" /><path d="M10 12h4M10 16h4" /></>,
    person: <><circle cx="12" cy="7" r="3" /><path d="M5.5 20c.4-4 2.5-6 6.5-6s6.1 2 6.5 6" /></>,
    flag: <><path d="M6 21V3" /><path d="M6 4c4-3 7 3 12 0v9c-5 3-8-3-12 0" /></>,
    note: <><path d="M9 18V6l10-2v12" /><circle cx="6" cy="18" r="3" /><circle cx="16" cy="16" r="3" /></>,
    gear: <><circle cx="12" cy="12" r="3" /><path d="m12 2 1 2.1 2.2.6 1.9-1.2 1.4 1.4-1.2 1.9.6 2.2L20 10v4l-2.1 1-.6 2.2 1.2 1.9-1.4 1.4-1.9-1.2-2.2.6L12 22l-1-2.1-2.2-.6-1.9 1.2-1.4-1.4 1.2-1.9-.6-2.2L4 14v-4l2.1-1 .6-2.2-1.2-1.9 1.4-1.4 1.9 1.2 2.2-.6Z" /></>,
    sunrise: <><path d="M4 17h16M6 14a6 6 0 0 1 12 0M12 3v3M5.5 6.5l2 2M18.5 6.5l-2 2" /></>,
    lock: <><rect x="5" y="10" width="14" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /></>,
    exclaim: <><path d="M12 4v9" /><circle cx="12" cy="18" r="1" /></>,
    skull: <><path d="M6 11a6 6 0 1 1 12 0v3.4c0 .9-.7 1.6-1.6 1.6H15v3H9v-3H7.6A1.6 1.6 0 0 1 6 14.4Z" /><circle cx="9.5" cy="11.5" r="1" /><circle cx="14.5" cy="11.5" r="1" /><path d="M10 15h4" /></>,
    spark: <path d="m12 2 1.7 6.3L20 10l-6.3 1.7L12 18l-1.7-6.3L4 10l6.3-1.7L12 2ZM19 16l.7 2.3L22 19l-2.3.7L19 22l-.7-2.3L16 19l2.3-.7L19 16Z" />,
    bars: <><path d="M5 19V13M10 19V9M15 19V5M20 19V11" /><path d="M3 19h19" /></>,
    vortex: <path d="M12 4a8 8 0 1 1-6.4 3.2c1.5-2 3.8-3.2 6.4-3.2Zm0 4a4 4 0 1 1-3.2 1.6c.8-1 1.9-1.6 3.2-1.6Z" />,
    target: <><circle cx="12" cy="12" r="8.5" /><circle cx="12" cy="12" r="4.5" /><circle cx="12" cy="12" r="1" /></>,
    crosshair: <><circle cx="12" cy="12" r="5" /><path d="M12 2v5M12 17v5M2 12h5M17 12h5" /><circle cx="12" cy="12" r="1.3" /></>,
    pin: <><path d="M12 3a5 5 0 0 1 5 5c0 4-5 8.5-5 8.5S7 12 7 8a5 5 0 0 1 5-5Z" /><circle cx="12" cy="8" r="1.7" /><path d="M12 16.5V22" /></>,
    rune: <><path d="M6 4h12l-2 4 2 4-2 4 2 4H6l2-4-2-4 2-4-2-4Z" /><path d="M10 8h4M10 16h4" /></>,
    generic: <path d="m12 3 7 5v8l-7 5-7-5V8l7-5Z" />,
  };

  return (
    <svg
      className="battlefield-counter-icon battlefield-counter-icon--generated"
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {glyphs[family] || glyphs.generic}
    </svg>
  );
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
        strokeOpacity={isPowerToughnessCounter ? "0.98" : "0.82"}
        strokeWidth={isPowerToughnessCounter ? "1.15" : "1"}
      />
      <path
        d="M10 9.5c3.2-4 7.2-5.8 12.5-5.8h40"
        fill="none"
        stroke="#e8f9ff"
        strokeOpacity={isPowerToughnessCounter ? "0.52" : "0.3"}
        strokeWidth={isPowerToughnessCounter ? "1.15" : "1"}
        strokeLinecap="round"
      />
    </svg>
  );
  const icon = isPowerToughnessCounter ? (
    <span
      className="battlefield-counter-symbol battlefield-counter-symbol--power"
      aria-hidden="true"
    >
      {badge.displayLabel}
    </span>
  ) : (
    <CounterKindGlyph kind={badge.kind || badge.displayLabel} />
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
