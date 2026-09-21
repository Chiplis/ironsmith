import { useI18n } from "@/i18n/I18nContext";
import { localeCatalogs } from "@/i18n/catalog";
import useUiText from "@/i18n/useUiText";
import { translateUiText as ui } from "@/i18n/catalog";
import { manaSymbolUrl } from './mana-assets.js';
import { createContext, useContext } from "react";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import { splitTextWithMtgKeywordRules } from "@/lib/mtg-keywords";

// Preserve unsupported symbols and arbitrary generic costs as readable text.
function fallbackCircle(label, size) {
  return (
    <svg width={size} height={size} viewBox="0 0 100 100" style={{ display: "inline-block", verticalAlign: "-0.15em" }}>
      <circle cx="50" cy="50" r="50" fill="#CAC5C0" />
      <text x="50" y="50" dy="0.36em" textAnchor="middle" fill="#0D0F0F"
        fontSize={label.length > 2 ? 38 : label.length > 1 ? 44 : 55}
        fontWeight="bold" fontFamily="serif">{ui(label)}</text>
    </svg>
  );
}

function wrapSymbolWithTooltip(sym, rendered, ui) {
  const tooltip = describeGameSymbol(sym, ui);
  const label = tooltip?.title || `{${String(sym || "").toUpperCase()}}`;

  const wrapped = (
    <span
      className="inline-flex shrink-0 items-center justify-center align-middle"
      aria-label={ui(label)}
    >
      {rendered}
    </span>
  );

  if (!tooltip) return wrapped;

  return (
    <ComicTooltip
      title={ui(tooltip.title)}
      description={ui(tooltip.description)}
      sideOffset={6}
    >
      {wrapped}
    </ComicTooltip>
  );
}

function describeHybridSymbol(left, right, ui) {
  if (left === "2") {
    const rightInfo = describeGameSymbol(right, ui);
    return {
      title: ui("{0} / Two Hybrid", { 0: ui(rightInfo?.title || right).toLowerCase() }),
      description: ui("This symbol can be paid with either two generic mana or one {0}.", { 0: ui(rightInfo?.title || right).toLowerCase() }),
    };
  }

  const leftInfo = describeGameSymbol(left, ui);
  const rightInfo = describeGameSymbol(right, ui);
  return {
    title: ui("{0} / {1} Hybrid", { 0: ui(leftInfo?.title || left).toLowerCase(), 1: ui(rightInfo?.title || right).toLowerCase().toLowerCase() }),
    description: ui("This hybrid symbol can be paid with either {0} or {1}.", { 0: ui(leftInfo?.title || left).toLowerCase(), 1: ui(rightInfo?.title || right).toLowerCase().toLowerCase() }),
  };
}

function describeGameSymbol(sym, ui) {
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
    return describeHybridSymbol(hybridMatch[1], hybridMatch[2], ui);
  }

  const phyrexianMatch = key.match(/^([WUBRG])\/P$/);
  if (phyrexianMatch) {
    const baseInfo = describeGameSymbol(phyrexianMatch[1], ui);
    return {
      title: ui("{0} Phyrexian", { 0: ui(baseInfo?.title || phyrexianMatch[1]) }),
      description: ui("This symbol can be paid with either one {0} or 2 life.", { 0: ui(baseInfo?.title || phyrexianMatch[1]).toLowerCase() }),
    };
  }

  return {
    title: `{${key}}`,
    description: "Represents a game symbol used in card text or mana costs.",
  };
}


export function ManaSymbol({ sym, size = 14 }) {
  const ui = useUiText();
  const withTooltip = rendered => wrapSymbolWithTooltip(sym, rendered, ui);
  if (!sym) return null;
  const key = sym.toUpperCase();

  // Some localized printings group consecutive mana in one pair of braces
  // (e.g. Spanish Pyretic Ritual's {RRR}). Each letter is a separate symbol.
  // Only expand plain mana letters; hybrid, Phyrexian and numeric codes stay whole.
  if (/^[WUBRGC]{2,}$/.test(key)) {
    return <>{[...key].map((code, index) => <ManaSymbol key={index} sym={code} size={size} />)}</>;
  }

  const source = manaSymbolUrl(key);
  if (source) {
    return withTooltip(<img src={source} alt="" aria-hidden="true" width={size} height={size}
      style={{ display: "inline-block", verticalAlign: "-0.15em" }} />);
  }

  // Fallback
  return withTooltip(fallbackCircle(sym, size));
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
  const ui = useUiText();
  const { locale } = useI18n({ optional: true });
  if (!rule) return text;
  const localizedRule = localeCatalogs[locale]?.rules?.[rule.id] || rule;
  const title = `${localizedRule.title} (${rule.rule})`;

  return (
    <ComicTooltip
      title={ui(title)}
      description={localizedRule.summary}
      sideOffset={7}
      contentClassName="max-w-[300px]"
    >
      <span
        className="mtg-keyword-helper"
        role="button"
        tabIndex={0}
        aria-label={ui("{0} rules helper", { 0: localizedRule.title })}
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

function keywordTextParts(text, nextKey, keywordHelpersEnabled, locale) {
  if (!keywordHelpersEnabled) return [text];

  return splitTextWithMtgKeywordRules(text, locale).map((segment) => {
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
  const { locale } = useI18n({ optional: true });
  const contextKeywordHelpersEnabled = useContext(KeywordHelpersEnabledContext);
  if (!text) return null;
  const keywordHelpersEnabled = keywordHelpers ?? contextKeywordHelpersEnabled;
  const parts = [];
  let last = 0;
  let key = 0;
  const nextKey = () => `keyword-${key++}`;
  const appendText = (value) => {
    if (!value) return;
    parts.push(...keywordTextParts(value, nextKey, keywordHelpersEnabled, locale));
  };

  const appendSegment = (segment) => {
    last = 0;
    for (const match of segment.matchAll(SYMBOL_RE)) {
      if (match.index > last) appendText(segment.slice(last, match.index));
      parts.push(<ManaSymbol key={`symbol-${key++}`} sym={match[1]} size={symbolSize} />);
      last = match.index + match[0].length;
    }
    if (last < segment.length) appendText(segment.slice(last));
  };
  // Reminder text is printed in italics, which also keeps translated reminder
  // paragraphs close to the room the printing gave them.
  for (const segment of text.split(/(\([^()]*\))/)) {
    if (!segment) continue;
    if (segment.startsWith("(") && segment.endsWith(")")) {
      const start = parts.length;
      appendSegment(segment);
      parts.push(<em key={`reminder-${key++}`} className="rules-reminder-text">{parts.splice(start)}</em>);
    } else appendSegment(segment);
  }
  return (
    <span
      className={className}
      style={{ whiteSpace: noWrap ? "nowrap" : "pre-wrap", ...style }}
    >
      {parts}
    </span>
  );
}
