import { MANA_SYMBOL_SVGS as SVG_DATA } from './mana-symbol-svg.js';
import { createContext, useContext } from "react";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import { splitTextWithMtgKeywordRules } from "@/lib/mtg-keywords";

/**
 * Inline Scryfall mana symbol SVGs — extracted once, no external fetches.
 * Each entry stores the viewBox and inner SVG markup for dangerouslySetInnerHTML.
 */



// For numbers > 10 and other unknowns, fall back to a simple circle with text
function fallbackCircle(label, size) {
  return (
    <svg width={size} height={size} viewBox="0 0 100 100" style={{ display: "inline-block", verticalAlign: "-0.15em" }}>
      <circle cx="50" cy="50" r="50" fill="#CAC5C0" />
      <text x="50" y="50" dy="0.36em" textAnchor="middle" fill="#0D0F0F"
        fontSize={label.length > 2 ? 38 : label.length > 1 ? 44 : 55}
        fontWeight="bold" fontFamily="serif">{label}</text>
    </svg>
  );
}

// Scryfall CDN symbol URL: {W/U} → "WU", {2/W} → "2W", {W/P} → "WP"
function scryfallSymbolUrl(code) {
  return `https://svgs.scryfall.io/card-symbols/${code}.svg`;
}

function scryfallImg(code, size) {
  return (
    <img
      src={scryfallSymbolUrl(code)}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      style={{ display: "inline-block", verticalAlign: "-0.15em", borderRadius: "50%" }}
      loading="lazy"
      referrerPolicy="no-referrer"
    />
  );
}

function symbolGraphicStyle() {
  return { display: "inline-block", verticalAlign: "-0.15em" };
}

function wrapSymbolWithTooltip(sym, rendered) {
  const tooltip = describeGameSymbol(sym);
  const label = tooltip?.title || `{${String(sym || "").toUpperCase()}}`;

  const wrapped = (
    <span
      className="inline-flex shrink-0 items-center justify-center align-middle"
      aria-label={label}
    >
      {rendered}
    </span>
  );

  if (!tooltip) return wrapped;

  return (
    <ComicTooltip
      title={tooltip.title}
      description={tooltip.description}
      sideOffset={6}
    >
      {wrapped}
    </ComicTooltip>
  );
}

function describeHybridSymbol(left, right) {
  if (left === "2") {
    const rightInfo = describeGameSymbol(right);
    return {
      title: `${rightInfo?.title || right} / Two Hybrid`,
      description: `This symbol can be paid with either two generic mana or one ${rightInfo?.title?.toLowerCase() || right} mana.`,
    };
  }

  const leftInfo = describeGameSymbol(left);
  const rightInfo = describeGameSymbol(right);
  return {
    title: `${leftInfo?.title || left} / ${rightInfo?.title || right} Hybrid`,
    description: `This hybrid symbol can be paid with either ${leftInfo?.title?.toLowerCase() || left} mana or ${rightInfo?.title?.toLowerCase() || right} mana.`,
  };
}

function describeGameSymbol(sym) {
  const key = String(sym || "").trim().toUpperCase();
  if (!key) return null;

  const exact = {
    W: {
      title: "White Mana",
      description: "Represents one white mana. White magic focuses on order, protection, healing, and teamwork.",
    },
    U: {
      title: "Blue Mana",
      description: "Represents one blue mana. Blue magic focuses on knowledge, control, tempo, and card draw.",
    },
    B: {
      title: "Black Mana",
      description: "Represents one black mana. Black magic trades resources for ambition, death, sacrifice, and reanimation.",
    },
    R: {
      title: "Red Mana",
      description: "Represents one red mana. Red magic is fast, emotional, and explosive, with damage and reckless aggression.",
    },
    G: {
      title: "Green Mana",
      description: "Represents one green mana. Green magic is about growth, creatures, lands, and raw natural strength.",
    },
    C: {
      title: "Colorless Mana",
      description: "Represents one colorless mana. It pays only costs that accept generic or specifically colorless mana.",
    },
    T: {
      title: "Tap",
      description: "Tap this permanent by turning it sideways. A tapped permanent is usually spent until it untaps.",
    },
    Q: {
      title: "Untap",
      description: "Untap this permanent by turning it upright. The untap symbol usually appears in activated abilities.",
    },
    S: {
      title: "Snow Mana",
      description: "This cost requires one mana produced by a snow source, not a specific color.",
    },
    E: {
      title: "Energy",
      description: "Represents one energy counter. Energy is a player resource kept separately from mana.",
    },
    X: {
      title: "Variable Cost",
      description: "X is a value chosen as the spell or ability is used. The total cost changes based on that chosen value.",
    },
    Y: {
      title: "Variable Cost",
      description: "Y is a variable value chosen by the effect or spell text when it is used.",
    },
    Z: {
      title: "Variable Cost",
      description: "Z is a variable value chosen by the effect or spell text when it is used.",
    },
  }[key];

  if (exact) return exact;

  if (/^\d+$/.test(key)) {
    const amount = Number(key);
    return {
      title: `${key} Generic Mana`,
      description: amount === 1
        ? "This cost requires one mana of any type."
        : `This cost requires ${key} mana of any type.`,
    };
  }

  const hybridMatch = key.match(/^([WUBRGC2])\/([WUBRG])$/);
  if (hybridMatch) {
    return describeHybridSymbol(hybridMatch[1], hybridMatch[2]);
  }

  const phyrexianMatch = key.match(/^([WUBRG])\/P$/);
  if (phyrexianMatch) {
    const baseInfo = describeGameSymbol(phyrexianMatch[1]);
    return {
      title: `${baseInfo?.title || phyrexianMatch[1]} Phyrexian`,
      description: `This symbol can be paid with either one ${baseInfo?.title?.toLowerCase() || phyrexianMatch[1]} mana or 2 life.`,
    };
  }

  return {
    title: `{${key}}`,
    description: "Represents a game symbol used in card text or mana costs.",
  };
}


export function ManaSymbol({ sym, size = 14 }) {
  if (!sym) return null;
  const key = sym.toUpperCase();

  // Check for exact match in SVG_DATA
  const data = SVG_DATA[key];
  if (data) {
    return wrapSymbolWithTooltip(sym, (
      <svg
        width={size}
        height={size}
        viewBox={data.vb}
        style={symbolGraphicStyle()}
        aria-hidden="true"
        dangerouslySetInnerHTML={{ __html: data.html }}
      />
    ));
  }

  // Generic mana numbers (11+)
  if (/^\d+$/.test(key)) return wrapSymbolWithTooltip(sym, fallbackCircle(key, size));

  // Y, Z variables
  if (/^[YZ]$/.test(key)) return wrapSymbolWithTooltip(sym, fallbackCircle(key, size));

  // Hybrid: W/U, U/B, 2/W, etc. → Scryfall CDN
  const hybridMatch = key.match(/^([WUBRGC2])\/([WUBRG])$/);
  if (hybridMatch) {
    return wrapSymbolWithTooltip(sym, scryfallImg(`${hybridMatch[1]}${hybridMatch[2]}`, size));
  }

  // Phyrexian: W/P, U/P, etc. → Scryfall CDN
  const phyMatch = key.match(/^([WUBRG])\/P$/);
  if (phyMatch) return wrapSymbolWithTooltip(sym, scryfallImg(`${phyMatch[1]}P`, size));

  // Fallback
  return wrapSymbolWithTooltip(sym, fallbackCircle(sym, size));
}

const SYMBOL_RE = /\{([^}]+)\}/g;
const KeywordHelpersEnabledContext = createContext(true);

export function KeywordHelpersProvider({ enabled = true, children }) {
  return (
    <KeywordHelpersEnabledContext.Provider value={enabled}>
      {children}
    </KeywordHelpersEnabledContext.Provider>
  );
}

function KeywordHelperText({ text, rule }) {
  if (!rule) return text;
  const title = `${rule.title} (${rule.rule})`;

  return (
    <ComicTooltip
      title={title}
      description={rule.summary}
      sideOffset={7}
      contentClassName="max-w-[300px]"
    >
      <span
        className="mtg-keyword-helper"
        role="button"
        tabIndex={0}
        aria-label={`${rule.title} rules helper`}
        data-keyword-helper={rule.id}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.stopPropagation()}
      >
        <span className="mtg-keyword-helper__text">{text}</span>
      </span>
    </ComicTooltip>
  );
}

function keywordTextParts(text, nextKey, keywordHelpersEnabled) {
  if (!keywordHelpersEnabled) return [text];

  return splitTextWithMtgKeywordRules(text).map((segment) => {
    if (segment.type !== "keyword") return segment.text;
    return (
      <KeywordHelperText
        key={nextKey()}
        text={segment.text}
        rule={segment.rule}
      />
    );
  });
}

export function ManaCostIcons({ cost, size = 12 }) {
  if (!cost) return null;
  const matches = cost.match(SYMBOL_RE);
  if (!matches) return null;
  return (
    <span className="inline-flex items-center gap-px flex-wrap">
      {matches.map((m, i) => {
        const s = m.slice(1, -1);
        return <ManaSymbol key={i} sym={s} size={size} />;
      })}
    </span>
  );
}

export function SymbolText({
  text,
  className,
  style,
  symbolSize = 14,
  noWrap = false,
  keywordHelpers = null,
}) {
  const contextKeywordHelpersEnabled = useContext(KeywordHelpersEnabledContext);
  if (!text) return null;
  const keywordHelpersEnabled = keywordHelpers ?? contextKeywordHelpersEnabled;
  const parts = [];
  let last = 0;
  let key = 0;
  const nextKey = () => `keyword-${key++}`;
  const appendText = (value) => {
    if (!value) return;
    parts.push(...keywordTextParts(value, nextKey, keywordHelpersEnabled));
  };

  for (const match of text.matchAll(SYMBOL_RE)) {
    if (match.index > last) appendText(text.slice(last, match.index));
    parts.push(<ManaSymbol key={`symbol-${key++}`} sym={match[1]} size={symbolSize} />);
    last = match.index + match[0].length;
  }
  if (last < text.length) appendText(text.slice(last));
  return (
    <span
      className={className}
      style={{ whiteSpace: noWrap ? "nowrap" : "pre-wrap", ...style }}
    >
      {parts}
    </span>
  );
}
