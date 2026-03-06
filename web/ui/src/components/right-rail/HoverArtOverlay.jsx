import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
import { Check, Copy } from "lucide-react";

const ORACLE_TEXT_STYLE = {
  textShadow: "0 0 1px rgba(0, 0, 0, 0.95), 0 1px 2px rgba(0, 0, 0, 0.88)",
};

const METADATA_TEXT_STYLE = {
  textShadow: "0 1px 2px rgba(0, 0, 0, 0.96), 0 2px 10px rgba(0, 0, 0, 0.84)",
};
const INSPECTOR_ART_SWAP_MS = 240;

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
      return cleaned;
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

function normalizeInspectorMeasureText(text = "") {
  return String(text)
    .replace(/\{[^}]+\}/g, " OO ")
    .replace(/\s+/g, " ")
    .trim();
}

function measureInspectorTextWidth(ctx, text = "") {
  const normalized = normalizeInspectorMeasureText(text);
  if (!normalized) return 0;
  return ctx.measureText(normalized).width;
}

function buildObjectNameMap(state) {
  const map = new Map();
  const players = state?.players || [];

  for (const player of players) {
    for (const card of player?.hand_cards || []) {
      map.set(Number(card.id), card.name);
    }
    for (const card of player?.graveyard_cards || []) {
      map.set(Number(card.id), card.name);
    }
    for (const card of player?.exile_cards || []) {
      map.set(Number(card.id), card.name);
    }
    for (const card of player?.battlefield || []) {
      const cardId = Number(card.id);
      if (Number.isFinite(cardId)) {
        map.set(cardId, card.name);
      }
      if (Array.isArray(card.member_ids)) {
        for (const memberId of card.member_ids) {
          const parsed = Number(memberId);
          if (Number.isFinite(parsed)) {
            map.set(parsed, card.name);
          }
        }
      }
    }
  }

  for (const stackObject of state?.stack_objects || []) {
    const cardId = Number(stackObject.id);
    if (Number.isFinite(cardId)) {
      map.set(cardId, stackObject.name);
    }
  }

  return map;
}

function parseBattleHealth(details, oracleText) {
  const counters = details?.counters || [];
  for (const counter of counters) {
    const kind = String(counter?.kind || "").toLowerCase();
    if (kind === "defense" || kind.includes("defense")) {
      const amount = Number(counter?.amount);
      if (Number.isFinite(amount)) return amount;
    }
  }

  const defenseMatch = String(oracleText || "").match(/\bDefense:\s*(\d+)\b/i);
  if (defenseMatch) {
    const parsed = Number(defenseMatch[1]);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

function buildObjectFamilyIds(state, objectIdNum) {
  const ids = new Set();
  if (!Number.isFinite(objectIdNum)) return ids;
  ids.add(objectIdNum);

  const players = state?.players || [];
  for (const player of players) {
    for (const card of player?.battlefield || []) {
      const rootId = Number(card?.id);
      const members = Array.isArray(card?.member_ids) ? card.member_ids : [];
      const familyIds = [rootId, ...members.map((memberId) => Number(memberId))]
        .filter((id) => Number.isFinite(id));
      if (!familyIds.includes(objectIdNum)) continue;
      for (const id of familyIds) ids.add(id);
      return ids;
    }
  }
  return ids;
}

function InspectorArtImageLayers({
  imageUrl,
  objectName,
  fullArt = false,
  onError,
}) {
  const [activeImageUrl, setActiveImageUrl] = useState(imageUrl || "");
  const [outgoingImageUrl, setOutgoingImageUrl] = useState(null);
  const activeImageUrlRef = useRef(imageUrl || "");
  const preloadRequestIdRef = useRef(0);
  const swapTimerRef = useRef(null);
  const activeLayerRef = useRef(null);
  const outgoingLayerRef = useRef(null);
  const activeMotionRef = useRef(null);
  const outgoingMotionRef = useRef(null);

  useEffect(() => {
    activeImageUrlRef.current = activeImageUrl;
  }, [activeImageUrl]);

  useEffect(() => {
    if (!imageUrl) {
      activeImageUrlRef.current = "";
      return undefined;
    }

    if (imageUrl === activeImageUrlRef.current) {
      return undefined;
    }

    const commitImageSwap = () => {
      const previousImageUrl = activeImageUrlRef.current;
      activeImageUrlRef.current = imageUrl;
      setOutgoingImageUrl(previousImageUrl && previousImageUrl !== imageUrl ? previousImageUrl : null);
      setActiveImageUrl(imageUrl);
    };

    if (typeof Image === "undefined") {
      queueMicrotask(commitImageSwap);
      return undefined;
    }

    const requestId = preloadRequestIdRef.current + 1;
    preloadRequestIdRef.current = requestId;
    let disposed = false;
    const preloader = new Image();
    preloader.decoding = "async";
    preloader.referrerPolicy = "no-referrer";
    preloader.onload = () => {
      if (disposed || preloadRequestIdRef.current !== requestId) return;
      commitImageSwap();
    };
    preloader.onerror = () => {
      if (disposed || preloadRequestIdRef.current !== requestId) return;
      if (typeof onError === "function") {
        onError(imageUrl);
      }
    };
    preloader.src = imageUrl;

    return () => {
      disposed = true;
      preloader.onload = null;
      preloader.onerror = null;
    };
  }, [imageUrl, onError]);

  useEffect(() => {
    if (!outgoingImageUrl) return undefined;
    if (swapTimerRef.current) {
      clearTimeout(swapTimerRef.current);
    }
    swapTimerRef.current = setTimeout(() => {
      setOutgoingImageUrl((currentImageUrl) => (
        currentImageUrl === outgoingImageUrl ? null : currentImageUrl
      ));
      swapTimerRef.current = null;
    }, INSPECTOR_ART_SWAP_MS + 60);

    return () => {
      if (swapTimerRef.current) {
        clearTimeout(swapTimerRef.current);
        swapTimerRef.current = null;
      }
    };
  }, [outgoingImageUrl]);

  useEffect(() => () => {
    if (swapTimerRef.current) {
      clearTimeout(swapTimerRef.current);
      swapTimerRef.current = null;
    }
  }, []);

  useLayoutEffect(() => {
    const node = activeLayerRef.current;
    if (!node) return undefined;

    cancelMotion(activeMotionRef.current);
    if (!outgoingImageUrl) {
      node.style.opacity = "1";
      node.style.transform = "translate3d(0,0,0) scale(1)";
      return undefined;
    }

    activeMotionRef.current = animate(node, {
      opacity: [0, 1],
      scale: [fullArt ? 1.012 : 1.028, 1],
      duration: INSPECTOR_ART_SWAP_MS,
      ease: uiSpring({ duration: INSPECTOR_ART_SWAP_MS, bounce: 0.04 }),
    });

    return () => {
      cancelMotion(activeMotionRef.current);
      activeMotionRef.current = null;
    };
  }, [activeImageUrl, fullArt, outgoingImageUrl]);

  useLayoutEffect(() => {
    const node = outgoingLayerRef.current;
    if (!node || !outgoingImageUrl) return undefined;

    cancelMotion(outgoingMotionRef.current);
    outgoingMotionRef.current = animate(node, {
      opacity: [1, 0],
      scale: [1, fullArt ? 1.02 : 1.036],
      duration: INSPECTOR_ART_SWAP_MS,
      ease: "out(3)",
    });

    return () => {
      cancelMotion(outgoingMotionRef.current);
      outgoingMotionRef.current = null;
    };
  }, [fullArt, outgoingImageUrl]);

  if (!activeImageUrl && !outgoingImageUrl) return null;

  const renderImageLayer = (src, ref, layerClassName) => {
    if (!src) return null;

    if (fullArt) {
      return (
        <div ref={ref} className={cn("absolute inset-[14px] flex items-center justify-center", layerClassName)}>
          <img
            src={src}
            alt={objectName || "Card art"}
            className="h-full w-full object-contain drop-shadow-[0_22px_24px_rgba(0,0,0,0.4)]"
            loading="eager"
            decoding="async"
            referrerPolicy="no-referrer"
            onError={() => {
              if (typeof onError === "function") {
                onError(src);
              }
            }}
          />
        </div>
      );
    }

    return (
      <div ref={ref} className={cn("hover-art-media absolute inset-0", layerClassName)}>
        <img
          src={src}
          alt={objectName || "Card art"}
          className="hover-art-pan h-full w-full object-cover"
          loading="eager"
          decoding="async"
          referrerPolicy="no-referrer"
          onError={() => {
            if (typeof onError === "function") {
              onError(src);
            }
          }}
        />
      </div>
    );
  };

  return (
    <>
      {renderImageLayer(outgoingImageUrl, outgoingLayerRef, "z-0 pointer-events-none")}
      {renderImageLayer(activeImageUrl, activeLayerRef, "z-[1] pointer-events-none")}
    </>
  );
}

export default function HoverArtOverlay({
  objectId,
  suppressStableId = null,
  stackTimelineHeight = 0,
  compact = false,
  displayMode = "inspector",
  onProtectedTopChange = null,
  onOracleTextHeightChange = null,
  onPreferredWidthChange = null,
}) {
  const { state, game, inspectorDebug } = useGame();
  const objectNameById = useMemo(() => buildObjectNameMap(state), [state]);
  const objectIdNum = objectId != null ? Number(objectId) : null;
  const objectIdKey = Number.isFinite(objectIdNum) ? String(objectIdNum) : null;
  const topHeaderRef = useRef(null);
  const oracleBodyRef = useRef(null);
  const oracleScrollRef = useRef(null);
  const ruleLineRefs = useRef(new Map());

  const [detailsCache, setDetailsCache] = useState({});
  const [failedImageUrl, setFailedImageUrl] = useState(null);
  const [copiedDebug, setCopiedDebug] = useState(false);

  useEffect(() => {
    if (!game || objectIdNum == null || !objectIdKey) return;
    if (Object.prototype.hasOwnProperty.call(detailsCache, objectIdKey)) return;

    let active = true;
    game.objectDetails(BigInt(objectIdNum))
      .then((details) => {
        if (!active) return;
        setDetailsCache((prev) => {
          if (Object.prototype.hasOwnProperty.call(prev, objectIdKey)) return prev;
          return { ...prev, [objectIdKey]: details || null };
        });
      })
      .catch(() => {
        if (!active) return;
        setDetailsCache((prev) => {
          if (Object.prototype.hasOwnProperty.call(prev, objectIdKey)) return prev;
          return { ...prev, [objectIdKey]: null };
        });
      });

    return () => {
      active = false;
    };
  }, [game, objectIdNum, objectIdKey, detailsCache]);

  const details = objectIdKey ? (detailsCache[objectIdKey] || null) : null;
  const hoveredStackObject = useMemo(
    () => (state?.stack_objects || []).find((entry) => String(entry.id) === String(objectIdNum)),
    [state?.stack_objects, objectIdNum]
  );
  const isFullArtMode = displayMode === "full-art";

  const objectName = details?.name
    || (Number.isFinite(objectIdNum) ? objectNameById.get(objectIdNum) : null)
    || hoveredStackObject?.name
    || null;
  const oracleText = details?.oracle_text
    || hoveredStackObject?.ability_text
    || hoveredStackObject?.effect_text
    || null;
  const manaCost = details?.mana_cost || hoveredStackObject?.mana_cost || null;
  const isBattle = String(details?.type_line || "").toLowerCase().includes("battle");
  const statsText = useMemo(() => {
    if (details?.power != null && details?.toughness != null) {
      return `${details.power}/${details.toughness}`;
    }
    if (details?.loyalty != null) {
      return `Loyalty ${details.loyalty}`;
    }
    if (isBattle) {
      const health = parseBattleHealth(details, oracleText);
      if (health != null) return `Health ${health}`;
    }
    return null;
  }, [details, oracleText, isBattle]);

  const countersText = useMemo(() => {
    const counters = details?.counters || [];
    if (counters.length === 0) return null;
    return counters
      .map((counter) => `${counter.amount} ${counter.kind}`)
      .join(" \u00b7 ");
  }, [details?.counters]);

  const metadataText = useMemo(() => {
    if (!details) return null;
    const parts = [];
    if (details.type_line) parts.push(details.type_line);
    if (details.zone) parts.push(details.zone);
    if (details.controller != null) parts.push(`P${details.controller}`);
    if (details.tapped) parts.push("Tapped");
    if (countersText) parts.push(countersText);
    return parts.length > 0 ? parts.join(" \u00b7 ") : null;
  }, [details, countersText]);
  const imageUrl = objectName ? scryfallImageUrl(objectName, "art_crop") : "";
  const imageErrored = !!imageUrl && failedImageUrl === imageUrl;
  const topStackObject = (state?.stack_objects || [])[0] || null;
  const detailAbilities = Array.isArray(details?.abilities) ? details.abilities : null;
  const detailStableId = details?.stable_id != null ? String(details.stable_id) : null;
  const topStackId = topStackObject?.id != null ? String(topStackObject.id) : null;
  const topStackStableId = topStackObject?.stable_id != null ? String(topStackObject.stable_id) : null;
  const topStackName = topStackObject?.name != null ? String(topStackObject.name) : "";
  const hoveredStackAbilityText = String(hoveredStackObject?.ability_text || "");
  const hoveredStackEffectText = String(hoveredStackObject?.effect_text || "");
  const objectFamilyIds = useMemo(
    () => buildObjectFamilyIds(state, objectIdNum),
    [state, objectIdNum]
  );
  const groupedCardCount = objectFamilyIds.size;

  const suppressObject =
    suppressStableId != null
    && details != null
    && Number(details.stable_id) === Number(suppressStableId);

  const semanticScore = Number(details?.semantic_score);
  const hasSemanticScore = Number.isFinite(semanticScore);
  const compiledText = detailAbilities && detailAbilities.length > 0
    ? stripInspectorAbilityPrefixes(detailAbilities.join("\n"))
    : stripInspectorAbilityPrefixes(oracleText || "");
  const displayRulesLines = useMemo(() => {
    if (detailAbilities && detailAbilities.length > 0) {
      return detailAbilities
        .map((line) => stripInspectorAbilityPrefixes(String(line || "")).trim())
        .filter(Boolean);
    }
    const fallback = (
      stripInspectorAbilityPrefixes(hoveredStackAbilityText).trim()
      || stripInspectorAbilityPrefixes(hoveredStackEffectText).trim()
      || stripInspectorAbilityPrefixes(String(oracleText || "")).trim()
    );
    if (!fallback) return [];
    return fallback
      .split(/\n+/)
      .map((line) => line.trim())
      .filter(Boolean);
  }, [detailAbilities, hoveredStackAbilityText, hoveredStackEffectText, oracleText]);
  const displayRulesText = displayRulesLines.join("\n");
  const preferredInlineWidth = useMemo(() => {
    if (!compact || typeof document === "undefined") return null;
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("2d");
    if (!context) return null;

    let maxTextWidth = 0;
    context.font = `700 ${compact ? 17 : 22}px Rajdhani, "Segoe UI", "Inter", sans-serif`;
    maxTextWidth = Math.max(maxTextWidth, measureInspectorTextWidth(context, objectName || ""));
    if (groupedCardCount > 1) {
      maxTextWidth += compact ? 18 : 22;
    }

    context.font = `700 ${compact ? 15 : 20}px Rajdhani, "Segoe UI", "Inter", sans-serif`;
    maxTextWidth = Math.max(maxTextWidth, measureInspectorTextWidth(context, statsText || ""));

    context.font = `600 ${compact ? 11 : 13}px Rajdhani, "Segoe UI", "Inter", sans-serif`;
    maxTextWidth = Math.max(maxTextWidth, measureInspectorTextWidth(context, metadataText || ""));

    const maxRuleLineMeasure = compact ? 320 : 460;
    context.font = `600 ${compact ? 15 : 18}px Rajdhani, "Segoe UI", "Inter", sans-serif`;
    for (const line of displayRulesLines) {
      maxTextWidth = Math.max(
        maxTextWidth,
        Math.min(maxRuleLineMeasure, measureInspectorTextWidth(context, line))
      );
    }

    const manaSymbols = String(manaCost || "").match(/\{[^}]+\}/g);
    if (manaSymbols && manaSymbols.length > 0) {
      maxTextWidth = Math.max(maxTextWidth, manaSymbols.length * (compact ? 16 : 20));
    }

    const horizontalPadding = compact ? 56 : 90;
    return Math.ceil(maxTextWidth + horizontalPadding);
  }, [compact, displayRulesLines, groupedCardCount, manaCost, metadataText, objectName, statsText]);
  const topStackMatchesInspectorObject = useMemo(() => {
    if (!topStackObject) return false;
    if (objectIdNum != null && topStackId === String(objectIdNum)) return true;
    if (detailStableId != null && topStackStableId != null) {
      if (topStackStableId === detailStableId) return true;
    }
    if (objectName && topStackName && topStackName === String(objectName)) return true;
    return false;
  }, [topStackObject, objectIdNum, topStackId, detailStableId, topStackStableId, objectName, topStackName]);
  const highlightedStackObject = useMemo(() => {
    if (hoveredStackObject) return hoveredStackObject;
    if (topStackMatchesInspectorObject) return topStackObject;
    return null;
  }, [hoveredStackObject, topStackMatchesInspectorObject, topStackObject]);
  const highlightedStackAbilityText = String(highlightedStackObject?.ability_text || "").trim();
  const highlightedStackEffectText = String(highlightedStackObject?.effect_text || "").trim();
  const highlightedStackAbilityKind = String(highlightedStackObject?.ability_kind || "").toLowerCase();
  const highlightedRuleLineIndices = useMemo(() => {
    const indices = new Set();
    if (!highlightedStackObject) return indices;
    if (!displayRulesLines.length) return indices;

    const stackAbilityText = (
      highlightedStackAbilityText
      || highlightedStackEffectText
    );
    if (stackAbilityText) {
      let bestScore = 0;
      const scored = [];
      displayRulesLines.forEach((line, index) => {
        const score = lineAbilityMatchScore(line, stackAbilityText);
        scored.push({ index, score });
        bestScore = Math.max(bestScore, score);
      });

      const minimumScore = bestScore >= 2 ? bestScore : 0;
      if (minimumScore > 0) {
        for (const entry of scored) {
          if (entry.score === bestScore && entry.score >= minimumScore) {
            indices.add(entry.index);
          }
        }
      }
    }

    if (indices.size === 0) {
      const kind = highlightedStackAbilityKind;
      if (kind.includes("trigger")) {
        const triggerIndex = displayRulesLines.findIndex((line) => (
          /^(when|whenever|at the beginning)\b/i.test(String(line).trim())
        ));
        if (triggerIndex >= 0) indices.add(triggerIndex);
      } else if (kind.includes("activat") || kind.includes("mana")) {
        const activatedIndex = displayRulesLines.findIndex((line) => String(line).includes(":"));
        if (activatedIndex >= 0) indices.add(activatedIndex);
      }
    }

    return indices;
  }, [
    highlightedStackObject,
    displayRulesLines,
    highlightedStackAbilityText,
    highlightedStackEffectText,
    highlightedStackAbilityKind,
  ]);
  const rawDefinition = details?.raw_compilation || "";
  const canCopyDebug = compiledText.trim().length > 0 || rawDefinition.trim().length > 0;
  const debugClipboardText = [
    objectName ? `Card: ${objectName}` : "",
    hasSemanticScore ? `Similarity score: ${(semanticScore * 100).toFixed(1)}%` : "",
    `Compiled text:\n${compiledText || "-"}`,
    `Raw CardDefinition:\n${rawDefinition || "-"}`,
  ]
    .filter(Boolean)
    .join("\n\n");

  const copyDebugPayload = useCallback(async () => {
    if (!canCopyDebug) return;
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(debugClipboardText);
        setCopiedDebug(true);
        return;
      }
    } catch {
      // Fall through to legacy clipboard path.
    }

    try {
      const textArea = document.createElement("textarea");
      textArea.value = debugClipboardText;
      textArea.setAttribute("readonly", "");
      textArea.style.position = "fixed";
      textArea.style.left = "-9999px";
      document.body.appendChild(textArea);
      textArea.select();
      const copied = document.execCommand("copy");
      document.body.removeChild(textArea);
      if (copied) {
        setCopiedDebug(true);
      }
    } catch {
      // ignore
    }
  }, [canCopyDebug, debugClipboardText]);

  useEffect(() => {
    if (!copiedDebug) return;
    const timer = setTimeout(() => setCopiedDebug(false), 1400);
    return () => clearTimeout(timer);
  }, [copiedDebug]);

  useLayoutEffect(() => {
    if (typeof onProtectedTopChange !== "function") return undefined;
    const node = topHeaderRef.current;
    if (!node) {
      onProtectedTopChange(null);
      return undefined;
    }

    let rafId = null;
    const publishProtectedTop = () => {
      const nodeRect = node.getBoundingClientRect();
      const overlayRect = node.parentElement?.getBoundingClientRect();
      if (!overlayRect) {
        onProtectedTopChange(null);
        return;
      }
      onProtectedTopChange(nodeRect.bottom - overlayRect.top);
    };

    publishProtectedTop();
    const observer = new ResizeObserver(() => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(publishProtectedTop);
    });
    observer.observe(node);
    window.addEventListener("resize", publishProtectedTop);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", publishProtectedTop);
      onProtectedTopChange(null);
    };
  }, [onProtectedTopChange, objectName, manaCost]);

  useLayoutEffect(() => {
    if (typeof onOracleTextHeightChange !== "function") return undefined;
    const node = oracleBodyRef.current;
    if (!node) {
      onOracleTextHeightChange(0);
      return undefined;
    }

    let rafId = null;
    const publishOracleHeight = () => {
      onOracleTextHeightChange(Math.ceil(node.scrollHeight));
    };

    publishOracleHeight();
    const observer = new ResizeObserver(() => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(publishOracleHeight);
    });
    observer.observe(node);
    window.addEventListener("resize", publishOracleHeight);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", publishOracleHeight);
      onOracleTextHeightChange(0);
    };
  }, [onOracleTextHeightChange, displayRulesText, highlightedRuleLineIndices, metadataText, statsText]);

  useLayoutEffect(() => {
    if (typeof onPreferredWidthChange !== "function") return;
    onPreferredWidthChange(preferredInlineWidth);
  }, [onPreferredWidthChange, preferredInlineWidth, objectIdKey]);

  useEffect(
    () => () => {
      if (typeof onPreferredWidthChange === "function") {
        onPreferredWidthChange(null);
      }
    },
    [onPreferredWidthChange]
  );

  useLayoutEffect(() => {
    const scroller = oracleScrollRef.current;
    if (!scroller) return;

    const highlightedIndices = Array.from(highlightedRuleLineIndices).sort((a, b) => a - b);
    if (highlightedIndices.length === 0) return;

    const firstNode = ruleLineRefs.current.get(highlightedIndices[0]);
    const lastNode = ruleLineRefs.current.get(highlightedIndices[highlightedIndices.length - 1]);
    if (!firstNode || !lastNode) return;

    const containerRect = scroller.getBoundingClientRect();
    const firstRect = firstNode.getBoundingClientRect();
    const lastRect = lastNode.getBoundingClientRect();

    const targetTop = firstRect.top - containerRect.top + scroller.scrollTop;
    const targetBottom = lastRect.bottom - containerRect.top + scroller.scrollTop;
    const viewTop = scroller.scrollTop;
    const viewBottom = viewTop + scroller.clientHeight;
    const margin = 8;

    if (targetTop < viewTop + margin) {
      scroller.scrollTop = Math.max(0, targetTop - margin);
      return;
    }
    if (targetBottom > viewBottom - margin) {
      scroller.scrollTop = Math.max(0, targetBottom - scroller.clientHeight + margin);
    }
  }, [objectIdKey, highlightedRuleLineIndices, displayRulesText]);

  const oracleTopPaddingClass = compact ? "pt-[56px]" : "pt-[164px]";
  const oracleContainerClass = compact
    ? `relative z-10 px-2.5 ${oracleTopPaddingClass} pb-1.5`
    : `relative z-10 min-h-full flex flex-col justify-end px-2.5 ${oracleTopPaddingClass} pb-2.5`;
  const topMetadataTextClassName = compact
    ? "text-[11px] leading-snug text-[#d1e2f6] text-right"
    : "text-[13px] leading-snug text-[#d1e2f6] text-right";
  const rulesTextClassName = compact
    ? "text-[13px] leading-[1.28] text-white font-semibold text-right"
    : "text-[17px] leading-[1.34] text-white font-semibold text-right";

  if (!imageUrl || imageErrored || suppressObject) return null;

  if (isFullArtMode) {
    return (
      <div
        className="hover-art-stage hover-art-drop-in absolute inset-0 z-30 overflow-hidden pointer-events-auto"
      >
        <div className="absolute inset-0 bg-[radial-gradient(92%_92%_at_50%_14%,rgba(80,145,232,0.32),rgba(8,13,20,0)_62%),linear-gradient(180deg,rgba(4,8,14,0.96),rgba(5,9,14,0.98))]" />
        <div className="absolute inset-[10px] overflow-hidden rounded-[18px] border border-[#5fa8ff]/35 bg-[rgba(4,8,14,0.92)] shadow-[0_0_0_1px_rgba(95,168,255,0.12),0_0_28px_rgba(68,149,246,0.2),0_28px_52px_rgba(0,0,0,0.48)]">
          <div className="absolute inset-0 bg-[radial-gradient(78%_62%_at_50%_24%,rgba(98,170,255,0.14),rgba(6,10,16,0)_62%)]" />
          <div className="absolute inset-[10px] rounded-[14px] border border-white/6 bg-[linear-gradient(180deg,rgba(255,255,255,0.04),rgba(255,255,255,0.01))]" />
          <InspectorArtImageLayers
            imageUrl={imageUrl}
            objectName={objectName}
            fullArt
            onError={setFailedImageUrl}
          />
        </div>
        <div className="pointer-events-none absolute inset-x-3 top-3 z-10 flex items-start justify-between gap-2">
          {objectName && (
            <div
              className="max-w-[72%] rounded-full border border-[#6eb4ff]/38 bg-[rgba(7,14,24,0.8)] px-3 py-1.5 text-[13px] font-extrabold leading-none tracking-[0.08em] text-[#edf6ff] shadow-[0_0_18px_rgba(58,140,245,0.18)] backdrop-blur-[10px]"
              style={METADATA_TEXT_STYLE}
            >
              <span className="inline-flex items-center gap-2">
                {groupedCardCount > 1 && (
                  <span className="inline-flex h-5 min-w-5 items-center justify-center rounded-full border border-[#f5d08b]/70 bg-[rgba(0,0,0,0.45)] px-1 text-[11px] font-bold leading-none tracking-wide text-[#f5d08b]">
                    x{groupedCardCount}
                  </span>
                )}
                <span className="truncate">{objectName}</span>
              </span>
            </div>
          )}
          {(manaCost || statsText) && (
            <div className="flex flex-col items-end gap-1">
              {manaCost && (
                <div className="rounded-full border border-[#6eb4ff]/28 bg-[rgba(7,14,24,0.78)] px-2.5 py-1 shadow-[0_0_16px_rgba(58,140,245,0.16)] backdrop-blur-[10px]">
                  <ManaCostIcons cost={manaCost} size={16} />
                </div>
              )}
              {statsText && (
                <div
                  className="rounded-full border border-[#f5d08b]/34 bg-[rgba(7,14,24,0.78)] px-2.5 py-1 text-[14px] font-extrabold leading-none tracking-[0.08em] text-[#f8d98e] shadow-[0_0_16px_rgba(245,208,139,0.12)] backdrop-blur-[10px]"
                  style={METADATA_TEXT_STYLE}
                >
                  {statsText}
                </div>
              )}
            </div>
          )}
        </div>
        {metadataText && (
          <div className="pointer-events-none absolute inset-x-3 bottom-3 z-10 flex justify-end">
            <div
              className="max-w-[84%] rounded-full border border-[#6eb4ff]/24 bg-[rgba(7,14,24,0.76)] px-3 py-1.5 text-right text-[12px] font-semibold leading-tight text-[#d3e8ff] shadow-[0_0_18px_rgba(58,140,245,0.14)] backdrop-blur-[10px]"
              style={METADATA_TEXT_STYLE}
            >
              {metadataText}
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div
      className={cn(
        "hover-art-stage hover-art-drop-in absolute inset-0 z-30 overflow-hidden",
        compact ? "pointer-events-auto" : "pointer-events-none"
      )}
    >
      <div className="hover-art-slice-in absolute inset-0">
        <InspectorArtImageLayers
          imageUrl={imageUrl}
          objectName={objectName}
          onError={setFailedImageUrl}
        />
      </div>
      <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,rgba(0,0,0,0.08)_0%,rgba(0,0,0,0.16)_48%,rgba(0,0,0,0.3)_100%)]" />
      <div className="absolute inset-0 overflow-hidden">
        <div className="pointer-events-none absolute inset-x-0 bottom-0 top-[34%] bg-[linear-gradient(180deg,rgba(0,0,0,0)_0%,rgba(0,0,0,0.52)_46%,rgba(0,0,0,0.74)_100%)] backdrop-blur-[2.4px]" />
        <div ref={topHeaderRef} className="absolute top-0 left-0 z-10 flex flex-col items-start gap-0">
          {objectName && (
            <div className={cn(
              "rounded-r-sm bg-[linear-gradient(90deg,rgba(0,0,0,0.66)_0%,rgba(0,0,0,0.44)_82%,rgba(0,0,0,0.12)_100%)] px-3 py-1.5 font-extrabold leading-[1.02] tracking-[0.02em] text-[#f3f8ff] backdrop-blur-[2px]",
              compact ? "text-[17px]" : "text-[22px]"
            )}>
              <span className="inline-flex items-center gap-2">
                {groupedCardCount > 1 && (
                  <span className="inline-flex h-5 min-w-5 items-center justify-center rounded-sm border border-[#f5d08b]/70 bg-[rgba(0,0,0,0.45)] px-1 text-[12px] font-bold leading-none tracking-wide text-[#f5d08b]">
                    x{groupedCardCount}
                  </span>
                )}
                <span>{objectName}</span>
              </span>
            </div>
          )}
        </div>
        {(manaCost || statsText || metadataText) && (
          <div className="absolute top-0 right-0 z-10 flex max-w-[78%] flex-col items-end gap-0">
            {(manaCost || statsText) && (
              <div className="flex items-center gap-1.5">
                {manaCost && (
                  <div className="rounded-sm bg-[rgba(0,0,0,0.52)] px-2 py-1 backdrop-blur-[1.8px]">
                    <ManaCostIcons cost={manaCost} size={compact ? 14 : 18} />
                  </div>
                )}
                {statsText && (
                  <div
                    className={cn(
                      "rounded-sm bg-[rgba(0,0,0,0.52)] px-2.5 py-1 text-[#f8d98e] tracking-wide text-right backdrop-blur-[1.8px]",
                      compact ? "text-[15px] font-extrabold leading-none" : "text-[20px] font-extrabold leading-none"
                    )}
                    style={METADATA_TEXT_STYLE}
                  >
                    {statsText}
                  </div>
                )}
              </div>
            )}
            {metadataText && (
              <div
                className={cn("rounded-sm bg-[rgba(0,0,0,0.48)] px-2.5 py-1 backdrop-blur-[1.8px]", topMetadataTextClassName)}
                style={METADATA_TEXT_STYLE}
              >
                {metadataText}
              </div>
            )}
          </div>
        )}
        {inspectorDebug && (
          <div className="absolute top-0 right-0 z-20 p-1 max-w-[66%] pointer-events-auto">
            <div className="rounded-sm border border-[#2f4662] bg-[rgba(5,11,20,0.84)] px-2 py-1 shadow-[0_8px_24px_rgba(0,0,0,0.5)]">
              <div className="flex items-start gap-2">
                <div className="min-w-0 text-[10px] leading-tight text-[#c7dbf2]">
                  <div className="font-bold uppercase tracking-wider text-[#8ec4ff]">Debug</div>
                  <div>
                    Similarity: {hasSemanticScore ? `${(semanticScore * 100).toFixed(1)}%` : "-"}
                  </div>
                </div>
                <button
                  type="button"
                  className={`shrink-0 mt-0.5 inline-flex h-5 w-5 items-center justify-center rounded border transition-colors ${
                    canCopyDebug
                      ? "border-[#436183] text-[#9dc9f8] hover:border-[#6e9ccc] hover:text-[#d9ecff]"
                      : "border-[#2a3d52] text-[#627d98] opacity-60"
                  }`}
                  disabled={!canCopyDebug}
                  title={canCopyDebug ? "Copy compiled + raw definition" : "No debug text available"}
                  onClick={copyDebugPayload}
                >
                  {copiedDebug ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
                </button>
              </div>
              <div className="mt-1 max-h-[180px] overflow-auto pr-0.5 text-[10px] leading-tight text-[#dbe9fb]">
                <div className="font-bold uppercase tracking-wider text-[#8ec4ff]">Compiled</div>
                <pre className="m-0 whitespace-pre-wrap break-words font-mono text-[10px]">{compiledText || "-"}</pre>
                <div className="mt-1 font-bold uppercase tracking-wider text-[#8ec4ff]">Raw</div>
                <pre className="m-0 whitespace-pre-wrap break-words font-mono text-[10px]">{rawDefinition || "-"}</pre>
              </div>
            </div>
          </div>
        )}

        <div
          ref={oracleScrollRef}
          className="inspector-oracle-scroll absolute inset-x-0 top-0 overflow-y-auto pointer-events-auto overscroll-contain touch-pan-y"
          style={{ bottom: `${Math.max(0, stackTimelineHeight - 4)}px` }}
        >
          <div className={oracleContainerClass}>
            <div ref={oracleBodyRef} className="space-y-1">
              {displayRulesLines.length > 0 && (
                <div className="space-y-0.5">
                  {displayRulesLines.map((line, lineIndex) => (
                    <div
                      key={`${lineIndex}-${line.slice(0, 32)}`}
                      ref={(node) => {
                        if (node) {
                          ruleLineRefs.current.set(lineIndex, node);
                        } else {
                          ruleLineRefs.current.delete(lineIndex);
                        }
                      }}
                      className="block w-full"
                    >
                      <SymbolText
                        text={line}
                        className={cn(
                          rulesTextClassName,
                          "inspector-oracle-line",
                          /^\s*[•*-]\s+/.test(String(line || "")) && "inspector-oracle-line-bullet",
                          highlightedRuleLineIndices.has(lineIndex)
                            ? "inspector-rule-highlight border-y"
                            : ""
                        )}
                        style={ORACLE_TEXT_STYLE}
                      />
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
