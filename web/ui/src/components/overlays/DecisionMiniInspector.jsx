import { useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { SymbolText } from "@/lib/mana-symbols";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { cn } from "@/lib/utils";

function stripInspectorAbilityPrefixes(text = "") {
  const prefixPatterns = [
    /^\s*(?:Triggered|Activated|Mana|Static)\s+ability(?:\s+\d+)?\s*:\s*/i,
    /^\s*Spell\s+effects?\s*:\s*/i,
    /^\s*Keyword\s+ability(?:\s+\{[^}]+\})*\s*:\s*/i,
  ];

  return String(text)
    .split("\n")
    .map((line) => {
      let cleaned = String(line || "");
      for (const pattern of prefixPatterns) {
        cleaned = cleaned.replace(pattern, "");
      }
      return cleaned.trim();
    })
    .join("\n");
}

function normalizeAbilityMatchText(text = "") {
  return stripInspectorAbilityPrefixes(text)
    .toLowerCase()
    .replace(/\{[^}]+\}/g, " ")
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function lineAbilityMatchScore(lineText, needleText) {
  const line = normalizeAbilityMatchText(lineText);
  const needle = normalizeAbilityMatchText(needleText);
  if (!line || !needle) return 0;
  if (line === needle) return 4;
  if (line.includes(needle) || needle.includes(line)) return 3;

  const words = needle.split(" ").filter((word) => word.length >= 4);
  if (words.length === 0) return 0;
  let matched = 0;
  for (const word of words) {
    if (line.includes(word)) matched += 1;
  }
  const ratio = matched / words.length;
  if (ratio >= 0.66) return 2;
  if (ratio >= 0.4) return 1;
  return 0;
}

function fallbackLines(...texts) {
  return texts
    .flatMap((text) => String(text || "").split(/\n|;\s+/))
    .map((line) => normalizeDecisionText(String(line || "").trim()))
    .filter(Boolean)
    .slice(0, 3);
}

function pickRelevantOracleLines(oracleText, compiledAbilities, needleText, abilityKind = "") {
  const oracleLines = String(oracleText || "")
    .split("\n")
    .map((line) => normalizeDecisionText(String(line || "").trim()))
    .filter(Boolean);
  const compiledLines = Array.isArray(compiledAbilities)
    ? compiledAbilities
      .map((line) => normalizeDecisionText(String(line || "").trim()))
      .filter(Boolean)
    : [];
  const lines = oracleLines.length > 0 ? oracleLines : compiledLines;
  if (lines.length === 0) return [];

  let bestScore = 0;
  const scored = lines.map((line, index) => {
    const score = lineAbilityMatchScore(line, needleText);
    bestScore = Math.max(bestScore, score);
    return { line, index, score };
  });

  if (bestScore >= 2) {
    return scored
      .filter((entry) => entry.score === bestScore)
      .map((entry) => entry.line)
      .slice(0, 3);
  }

  const normalizedKind = String(abilityKind || "").toLowerCase();
  if (normalizedKind.includes("trigger")) {
    const triggerLine = lines.find((line) => /^(when|whenever|at the beginning)\b/i.test(line));
    if (triggerLine) return [triggerLine];
  }
  if (normalizedKind.includes("activat") || normalizedKind.includes("mana")) {
    const activatedLine = lines.find((line) => line.includes(":"));
    if (activatedLine) return [activatedLine];
  }

  return [];
}

function pickDefaultOracleLines(oracleText, compiledAbilities, abilityKind = "") {
  const oracleLines = String(oracleText || "")
    .split("\n")
    .map((line) => normalizeDecisionText(String(line || "").trim()))
    .filter(Boolean);
  const compiledLines = Array.isArray(compiledAbilities)
    ? compiledAbilities
      .map((line) => normalizeDecisionText(String(line || "").trim()))
      .filter(Boolean)
    : [];
  const lines = oracleLines.length > 0 ? oracleLines : compiledLines;
  if (lines.length === 0) return [];

  const normalizedKind = String(abilityKind || "").toLowerCase();
  if (normalizedKind.includes("trigger")) {
    const triggerLine = lines.find((line) => /^(when|whenever|at the beginning)\b/i.test(line));
    if (triggerLine) return [triggerLine];
  }
  if (normalizedKind.includes("activat") || normalizedKind.includes("mana")) {
    const activatedLine = lines.find((line) => line.includes(":"));
    if (activatedLine) return [activatedLine];
  }

  return lines.slice(0, 3);
}

export default function DecisionMiniInspector({
  decision,
  stackObject = null,
  className = "",
}) {
  const { game } = useGame();
  const inspectorSourceId = Number(
    stackObject?.inspect_object_id ?? stackObject?.id ?? decision?.source_id
  );
  const inspectorSourceIdKey = Number.isFinite(inspectorSourceId) ? String(inspectorSourceId) : null;
  const [detailsCache, setDetailsCache] = useState({});
  const transitionTimeoutRef = useRef(null);
  const transitionFrameRef = useRef(null);
  const hasCachedDetails = inspectorSourceIdKey != null
    && Object.prototype.hasOwnProperty.call(detailsCache, inspectorSourceIdKey);

  useEffect(() => {
    if (!game || inspectorSourceIdKey == null) return undefined;
    if (hasCachedDetails) return undefined;

    let active = true;
    game.objectDetails(BigInt(inspectorSourceId))
      .then((nextDetails) => {
        if (!active) return;
        setDetailsCache((prev) => {
          if (Object.prototype.hasOwnProperty.call(prev, inspectorSourceIdKey)) return prev;
          return { ...prev, [inspectorSourceIdKey]: nextDetails || null };
        });
      })
      .catch(() => {
        if (!active) return;
        setDetailsCache((prev) => {
          if (Object.prototype.hasOwnProperty.call(prev, inspectorSourceIdKey)) return prev;
          return { ...prev, [inspectorSourceIdKey]: null };
        });
      });

    return () => {
      active = false;
    };
  }, [game, hasCachedDetails, inspectorSourceId, inspectorSourceIdKey]);
  const details = inspectorSourceIdKey ? (detailsCache[inspectorSourceIdKey] || null) : null;

  const sourceName = normalizeDecisionText(
    String(details?.name || stackObject?.name || decision?.source_name || "").trim()
  );
  const decisionNeedleText = String(
    decision?.context_text
    || decision?.consequence_text
    || decision?.description
    || ""
  ).trim();
  const stackNeedleText = String(stackObject?.ability_text || stackObject?.effect_text || "").trim();
  const needleText = stackNeedleText || decisionNeedleText;
  const oracleLines = useMemo(
    () => {
      const relevantLines = pickRelevantOracleLines(
        details?.oracle_text,
        details?.compiled_text || details?.abilities,
        needleText,
        stackObject?.ability_kind || ""
      );
      if (relevantLines.length > 0) return relevantLines;
      return pickDefaultOracleLines(
        details?.oracle_text,
        details?.compiled_text || details?.abilities,
        stackObject?.ability_kind || ""
      );
    },
    [
      details?.abilities,
      details?.compiled_text,
      details?.oracle_text,
      needleText,
      stackObject?.ability_kind,
    ]
  );
  const displayLines = useMemo(() => {
    if (oracleLines.length > 0) return oracleLines;
    return fallbackLines(
      stackNeedleText,
      decisionNeedleText,
      decision?.context_text,
      decision?.consequence_text
    );
  }, [
    decision?.consequence_text,
    decision?.context_text,
    decisionNeedleText,
    oracleLines,
    stackNeedleText,
  ]);

  const combinedText = displayLines.join(" ").trim();
  const panel = useMemo(
    () => ({
      key: [
        inspectorSourceIdKey || "none",
        stackObject?.id ?? "none",
        stackObject?.ability_kind || "",
        stackNeedleText,
        decision?.kind || "",
      ].join("|"),
      sourceName: sourceName || "Source",
      combinedText,
    }),
    [
      combinedText,
      decision?.kind,
      inspectorSourceIdKey,
      sourceName,
      stackNeedleText,
      stackObject?.ability_kind,
      stackObject?.id,
    ]
  );
  const [renderedPanel, setRenderedPanel] = useState(panel);
  const [transitionPhase, setTransitionPhase] = useState("entered");

  useEffect(() => {
    if (panel == null) {
      setRenderedPanel(null);
      setTransitionPhase("entered");
      return undefined;
    }

    if (renderedPanel == null) {
      setRenderedPanel(panel);
      setTransitionPhase("entered");
      return undefined;
    }

    if (renderedPanel.key === panel.key) {
      setRenderedPanel(panel);
      setTransitionPhase("entered");
      return undefined;
    }

    if (typeof window === "undefined") {
      setRenderedPanel(panel);
      setTransitionPhase("entered");
      return undefined;
    }

    if (transitionTimeoutRef.current) {
      window.clearTimeout(transitionTimeoutRef.current);
      transitionTimeoutRef.current = null;
    }
    if (transitionFrameRef.current) {
      window.cancelAnimationFrame(transitionFrameRef.current);
      transitionFrameRef.current = null;
    }

    setTransitionPhase("leaving");
    transitionTimeoutRef.current = window.setTimeout(() => {
      setRenderedPanel(panel);
      setTransitionPhase("entering");
      transitionFrameRef.current = window.requestAnimationFrame(() => {
        transitionFrameRef.current = window.requestAnimationFrame(() => {
          setTransitionPhase("entered");
          transitionFrameRef.current = null;
        });
      });
      transitionTimeoutRef.current = null;
    }, 110);

    return () => {
      if (transitionTimeoutRef.current) {
        window.clearTimeout(transitionTimeoutRef.current);
        transitionTimeoutRef.current = null;
      }
      if (transitionFrameRef.current) {
        window.cancelAnimationFrame(transitionFrameRef.current);
        transitionFrameRef.current = null;
      }
    };
  }, [panel, renderedPanel]);

  useEffect(() => () => {
    if (typeof window === "undefined") return;
    if (transitionTimeoutRef.current) {
      window.clearTimeout(transitionTimeoutRef.current);
      transitionTimeoutRef.current = null;
    }
    if (transitionFrameRef.current) {
      window.cancelAnimationFrame(transitionFrameRef.current);
      transitionFrameRef.current = null;
    }
  }, []);

  const contentMotionStyle = transitionPhase === "leaving"
    ? { opacity: 0, transform: "translateY(5px)" }
    : transitionPhase === "entering"
      ? { opacity: 0, transform: "translateY(-5px)" }
      : { opacity: 1, transform: "translateY(0)" };

  if (!sourceName && displayLines.length === 0) return null;

  return (
    <section
      data-decision-mini-inspector
      className={cn(
        "h-full min-w-0 w-full overflow-hidden bg-[linear-gradient(180deg,rgba(9,20,33,0.98),rgba(7,14,24,0.98))]",
        className
      )}
    >
      <div
        className="flex h-full min-w-0 items-start gap-3 overflow-y-auto overflow-x-hidden px-3 py-2"
        style={{
          ...contentMotionStyle,
          transition: "opacity 180ms cubic-bezier(0.22, 1, 0.36, 1), transform 180ms cubic-bezier(0.22, 1, 0.36, 1)",
          willChange: "opacity, transform",
        }}
      >
        <div className="shrink-0 whitespace-nowrap text-[13px] font-bold uppercase tracking-[0.08em] text-[#edf5ff]">
          {renderedPanel?.sourceName || "Source"}
        </div>
        {renderedPanel?.combinedText && (
          <SymbolText
            text={renderedPanel.combinedText}
            className="block min-w-0 flex-1 text-[11px] leading-snug text-[#cfe3fb]"
            style={{ overflowWrap: "anywhere", wordBreak: "break-word" }}
          />
        )}
      </div>
    </section>
  );
}
