import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";
import { getPlayerAccent } from "@/lib/player-colors";
import { getVisibleStackObjects, getVisibleTopStackObject } from "@/lib/stack-targets";
import { cn } from "@/lib/utils";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
import { Check, ChevronLeft, ChevronRight, Copy } from "lucide-react";

const ORACLE_TEXT_STYLE = {
  textShadow: "0 0 1px rgba(0, 0, 0, 0.95), 0 1px 2px rgba(0, 0, 0, 0.88)",
};

const METADATA_TEXT_STYLE = {
  textShadow: "0 1px 2px rgba(0, 0, 0, 0.96), 0 2px 10px rgba(0, 0, 0, 0.84)",
};
const INSPECTOR_ART_SWAP_MS = 240;
const MIN_INSPECTOR_TEXT_SCALE = 0.74;
const MIN_INSPECTOR_TITLE_SCALE = 0.16;
const INSPECTOR_TITLE_FONT_SIZE = 22;
const INSPECTOR_STATS_FONT_SIZE = 20;
const INSPECTOR_METADATA_FONT_SIZE = 13;
const INSPECTOR_RULES_FONT_SIZE = 17;
const INSPECTOR_RULES_LINE_HEIGHT = INSPECTOR_RULES_FONT_SIZE * 1.34;
const INSPECTOR_RULES_ROW_GAP = 2;
const INSPECTOR_DEFAULT_HEIGHT = 248;
const INSPECTOR_RULES_MIN_WIDTH = 220;
const INSPECTOR_RULES_MAX_LINE_WIDTH = 920;
const INSPECTOR_HEADER_HORIZONTAL_PADDING = 136;
const INSPECTOR_ORACLE_TOP_PADDING = 92;
const INSPECTOR_ORACLE_BOTTOM_PADDING = 10;
const INSPECTOR_ORACLE_HORIZONTAL_PADDING = 28;
const INSPECTOR_MEASURE_FONT = `"Optima", "Avenir Next", "Segoe UI", "Candara", sans-serif`;

function clampNumber(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function estimateInspectorRulesHeight(lineWidths, innerWidth) {
  const width = Math.max(1, Number(innerWidth) || 0);
  let totalHeight = 0;

  for (const lineWidth of lineWidths) {
    const wraps = Math.max(1, Math.ceil(lineWidth / width));
    totalHeight += wraps * INSPECTOR_RULES_LINE_HEIGHT;
  }

  if (lineWidths.length > 1) {
    totalHeight += (lineWidths.length - 1) * INSPECTOR_RULES_ROW_GAP;
  }

  return totalHeight;
}

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

function buildPlayerNameMap(state) {
  const byId = new Map();
  for (const player of state?.players || []) {
    const id = Number(player?.id);
    if (!Number.isFinite(id)) continue;
    const name = String(player?.name || "").trim();
    byId.set(id, name || `P${id + 1}`);
  }
  return byId;
}

function formatInspectorPlayerLabel(playerId, playerNameById) {
  const id = Number(playerId);
  if (!Number.isFinite(id)) return null;
  return playerNameById.get(id) || `P${id + 1}`;
}

function normalizeInspectorCounters(rawCounters) {
  if (!Array.isArray(rawCounters)) return [];
  return rawCounters
    .map((counter) => {
      const kind = String(counter?.kind || "").trim();
      const amount = Number(counter?.amount);
      if (!kind || !Number.isFinite(amount) || amount <= 0) return null;
      return { kind, amount };
    })
    .filter(Boolean);
}

function formatInspectorCounterLine(counters) {
  if (!Array.isArray(counters) || counters.length === 0) return null;
  return counters
    .map((counter) => `${counter.amount} ${counter.kind}`)
    .join(" · ");
}

function InspectorMetadataBlock({
  lines,
  className = "",
  lineClassName = "",
  style,
}) {
  if (!Array.isArray(lines) || lines.length === 0) return null;

  return (
    <div className={className} style={style}>
      {lines.map((line, index) => (
        <div
          key={`${line}-${index}`}
          className={cn(index > 0 && "mt-0.5", lineClassName)}
        >
          {line}
        </div>
      ))}
    </div>
  );
}

function handleInspectorChevronPointerDown(event) {
  event.preventDefault();
  event.stopPropagation();
}

function handleInspectorChevronClick(callback, event) {
  event.preventDefault();
  event.stopPropagation();
  callback?.();
}

function setObjectName(map, key, name, options = {}) {
  const parsedKey = Number(key);
  if (!Number.isFinite(parsedKey)) return;
  if (!name) return;
  if (options.onlyIfMissing && map.has(parsedKey)) return;
  map.set(parsedKey, name);
}

function preferredStackStableId(stackObject) {
  const stableId = Number(stackObject?.stable_id);
  if (Number.isFinite(stableId)) return stableId;
  const sourceStableId = Number(stackObject?.source_stable_id);
  if (Number.isFinite(sourceStableId)) return sourceStableId;
  return null;
}

function stackStableIdCandidates(stackObject) {
  const candidates = [];
  const stableId = Number(stackObject?.stable_id);
  const sourceStableId = Number(stackObject?.source_stable_id);
  if (Number.isFinite(stableId)) {
    candidates.push(stableId);
  }
  if (Number.isFinite(sourceStableId) && sourceStableId !== stableId) {
    candidates.push(sourceStableId);
  }
  return candidates;
}

function buildObjectNameMaps(state) {
  const byId = new Map();
  const byStableId = new Map();
  const players = state?.players || [];

  for (const player of players) {
    for (const card of player?.hand_cards || []) {
      setObjectName(byId, card.id, card.name);
      setObjectName(byStableId, card.stable_id, card.name);
    }
    for (const card of player?.graveyard_cards || []) {
      setObjectName(byId, card.id, card.name);
      setObjectName(byStableId, card.stable_id, card.name);
    }
    for (const card of player?.exile_cards || []) {
      setObjectName(byId, card.id, card.name);
      setObjectName(byStableId, card.stable_id, card.name);
    }
    for (const card of player?.command_cards || []) {
      setObjectName(byId, card.id, card.name);
      setObjectName(byStableId, card.stable_id, card.name);
    }
    for (const card of player?.battlefield || []) {
      setObjectName(byId, card.id, card.name);
      setObjectName(byStableId, card.stable_id, card.name);
      if (Array.isArray(card.member_ids)) {
        for (const memberId of card.member_ids) {
          setObjectName(byId, memberId, card.name);
        }
      }
      if (Array.isArray(card.member_stable_ids)) {
        for (const memberStableId of card.member_stable_ids) {
          setObjectName(byStableId, memberStableId, card.name);
        }
      }
    }
  }

  for (const stackObject of getVisibleStackObjects(state)) {
    for (const candidateId of [stackObject.id, stackObject.inspect_object_id]) {
      setObjectName(byId, candidateId, stackObject.name);
    }
    setObjectName(byStableId, stackObject.stable_id, stackObject.name, { onlyIfMissing: true });
    setObjectName(byStableId, stackObject.source_stable_id, stackObject.name, { onlyIfMissing: true });
  }

  return { byId, byStableId };
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
  transientPreview = null,
  transientPreviewIndex = 0,
  transientPreviewCount = 0,
  onShowPreviousTransientPreview = null,
  onShowNextTransientPreview = null,
  stackTimelineHeight = 0,
  compact = false,
  displayMode = "inspector",
  inspectorVariant = "normal",
  availableInspectorWidth = null,
  availableInspectorHeight = null,
  hideOwnershipMetadata = false,
  minInspectorTextScale = MIN_INSPECTOR_TEXT_SCALE,
  minInspectorTitleScale = MIN_INSPECTOR_TITLE_SCALE,
  onProtectedTopChange = null,
  onOracleTextHeightChange = null,
  onPreferredWidthChange = null,
  onPreferredInspectorWidthChange = null,
  onInspectorAccentChange = null,
}) {
  const { state, game } = useGame();
  const debugInspector = inspectorVariant === "debug";
  const playerNameById = useMemo(() => buildPlayerNameMap(state), [state]);
  const { byId: objectNameById, byStableId: objectNameByStableId } = useMemo(
    () => buildObjectNameMaps(state),
    [state]
  );
  const previewCard = transientPreview?.card && typeof transientPreview.card === "object"
    ? transientPreview.card
    : null;
  const transitionTitle = String(transientPreview?.title || "").trim() || null;
  const hasTransitionNavigator = transitionTitle && transientPreviewCount > 1;
  const transitionSequenceLabel = hasTransitionNavigator
    ? `${Math.min(transientPreviewIndex + 1, transientPreviewCount)}/${transientPreviewCount}`
    : null;
  const objectIdNum = objectId != null ? Number(objectId) : null;
  const objectIdKey = Number.isFinite(objectIdNum) ? String(objectIdNum) : null;
  const topHeaderRef = useRef(null);
  const topMetadataRef = useRef(null);
  const inspectorTitleRef = useRef(null);
  const oracleBodyRef = useRef(null);
  const oracleContainerRef = useRef(null);
  const oracleScrollRef = useRef(null);
  const ruleLineRefs = useRef(new Map());

  const [detailsCache, setDetailsCache] = useState({});
  const [failedImageUrl, setFailedImageUrl] = useState(null);
  const [copiedDebug, setCopiedDebug] = useState(false);
  const [inspectorScaleSession, setInspectorScaleSession] = useState({ key: null, scale: 1 });
  const [inspectorTitleScaleSession, setInspectorTitleScaleSession] = useState({ key: null, scale: 1 });
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
    () => getVisibleStackObjects(state).find((entry) => (
      String(entry.id) === String(objectIdNum)
      || String(entry.inspect_object_id) === String(objectIdNum)
    )),
    [state, objectIdNum]
  );
  const isFullArtMode = displayMode === "full-art";
  const artStackObject = useMemo(() => {
    if (hoveredStackObject) return hoveredStackObject;
    return null;
  }, [hoveredStackObject]);
  const artStableId = useMemo(
    () => preferredStackStableId(artStackObject),
    [artStackObject]
  );
  const stableLinkedObjectName = useMemo(
    () => (Number.isFinite(artStableId) ? objectNameByStableId.get(artStableId) : null),
    [artStableId, objectNameByStableId]
  );

  const previewObjectName = String(previewCard?.name || "").trim() || null;
  const previewOracleText = String(previewCard?.oracle_text || previewCard?.effect_text || "").trim() || null;
  const previewManaCost = previewCard?.mana_cost || null;
  const previewTypeLine = String(previewCard?.type_line || "").trim() || null;
  const previewZoneLine = String(previewCard?.zone || "").trim() || null;

  const objectName = details?.name
    || previewObjectName
    || (Number.isFinite(objectIdNum) ? objectNameById.get(objectIdNum) : null)
    || hoveredStackObject?.name
    || null;
  const oracleText = details?.oracle_text
    || previewOracleText
    || hoveredStackObject?.ability_text
    || hoveredStackObject?.effect_text
    || null;
  const manaCost = details?.mana_cost || previewManaCost || hoveredStackObject?.mana_cost || null;
  const isBattle = String(details?.type_line || previewTypeLine || "").toLowerCase().includes("battle");
  const statsText = useMemo(() => {
    if (details?.power != null && details?.toughness != null) {
      return `${details.power}/${details.toughness}`;
    }
    if (previewCard?.power != null && previewCard?.toughness != null) {
      return `${previewCard.power}/${previewCard.toughness}`;
    }
    if (details?.loyalty != null) {
      return `Loyalty ${details.loyalty}`;
    }
    if (previewCard?.loyalty != null) {
      return `Loyalty ${previewCard.loyalty}`;
    }
    if (isBattle) {
      const health = parseBattleHealth(details || previewCard, oracleText);
      if (health != null) return `Health ${health}`;
    }
    return null;
  }, [details, isBattle, oracleText, previewCard]);

  const normalizedCounters = useMemo(
    () => normalizeInspectorCounters(details?.counters || previewCard?.counters),
    [details?.counters, previewCard?.counters]
  );

  const typeLine = String(details?.type_line || previewTypeLine || hoveredStackObject?.type_line || "").trim() || null;
  const typeLineDisplay = String(
    details?.type_line_display
    || previewTypeLine
    || hoveredStackObject?.type_line
    || typeLine
    || ""
  ).trim() || null;
  const typeLineBadges = Array.isArray(details?.type_line_badges)
    ? details.type_line_badges
      .map((badge) => String(badge || "").trim())
      .filter(Boolean)
    : [];
  const zoneLine = String(details?.zone || previewZoneLine || hoveredStackObject?.zone || "").trim() || null;
  const ownershipMetadataLines = useMemo(() => {
    if (hideOwnershipMetadata) return [];

    const ownerLabel = formatInspectorPlayerLabel(details?.owner ?? previewCard?.owner, playerNameById);
    const controllerLabel = formatInspectorPlayerLabel(details?.controller ?? previewCard?.controller, playerNameById);

    if (ownerLabel && controllerLabel && ownerLabel === controllerLabel) {
      return [`Controller/Owner: ${ownerLabel}`];
    }

    return [
      controllerLabel ? `Controller: ${controllerLabel}` : null,
      ownerLabel ? `Owner: ${ownerLabel}` : null,
    ].filter(Boolean);
  }, [details, hideOwnershipMetadata, playerNameById, previewCard]);
  const countersLine = useMemo(
    () => formatInspectorCounterLine(normalizedCounters),
    [normalizedCounters]
  );
  const inspectorAccent = useMemo(() => {
    const controllerId = details?.controller ?? previewCard?.controller ?? hoveredStackObject?.controller ?? null;
    return controllerId == null ? null : getPlayerAccent(state?.players || [], controllerId);
  }, [details?.controller, hoveredStackObject?.controller, previewCard?.controller, state?.players]);
  const artObjectName = stableLinkedObjectName || objectName;
  const imageUrl = artObjectName ? scryfallImageUrl(artObjectName, "art_crop") : "";
  const imageErrored = !!imageUrl && failedImageUrl === imageUrl;
  const topStackObject = getVisibleTopStackObject(state);
  const detailCompiledText = Array.isArray(details?.compiled_text) ? details.compiled_text : null;
  const detailAbilities = Array.isArray(details?.abilities) ? details.abilities : null;
  const detailStableId = details?.stable_id != null ? String(details.stable_id) : null;
  const topStackId = topStackObject?.inspect_object_id != null
    ? String(topStackObject.inspect_object_id)
    : (topStackObject?.id != null ? String(topStackObject.id) : null);
  const topStackStableIds = stackStableIdCandidates(topStackObject).map((stableId) => String(stableId));
  const topStackName = topStackObject?.name != null ? String(topStackObject.name) : "";
  const hoveredStackAbilityText = String(hoveredStackObject?.ability_text || "");
  const hoveredStackEffectText = String(hoveredStackObject?.effect_text || "");
  const objectFamilyIds = useMemo(
    () => buildObjectFamilyIds(state, objectIdNum),
    [state, objectIdNum]
  );
  const groupedCardCount = objectFamilyIds.size > 0
    ? objectFamilyIds.size
    : Math.max(
      1,
      Array.isArray(previewCard?.member_ids)
        ? previewCard.member_ids.length + (previewCard?.id != null ? 1 : 0)
        : 1
    );

  const semanticScore = Number(details?.semantic_score);
  const hasSemanticScore = Number.isFinite(semanticScore);
  const similarityBadgeLabel = hasSemanticScore
    ? `Similarity ${(semanticScore * 100).toFixed(1)}%`
    : "Similarity --";
  const compiledText = detailCompiledText && detailCompiledText.length > 0
    ? stripInspectorAbilityPrefixes(detailCompiledText.join("\n"))
    : detailAbilities && detailAbilities.length > 0
    ? stripInspectorAbilityPrefixes(detailAbilities.join("\n"))
    : stripInspectorAbilityPrefixes(
      hoveredStackAbilityText
      || hoveredStackEffectText
      || String(oracleText || "")
    );
  const showCompiledText = debugInspector;
  const oracleRulesLines = useMemo(() => {
    return String(details?.oracle_text || "")
      .split("\n")
      .map((line) => String(line || "").trim())
      .filter(Boolean);
  }, [details?.oracle_text]);
  const compiledRulesLines = useMemo(() => {
    if (detailCompiledText && detailCompiledText.length > 0) {
      return detailCompiledText
        .map((line) => stripInspectorAbilityPrefixes(String(line || "")).trim())
        .filter(Boolean);
    }
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
  }, [detailAbilities, detailCompiledText, hoveredStackAbilityText, hoveredStackEffectText, oracleText]);
  const shouldPreferStackAbilityRules = (
    Boolean(hoveredStackObject?.ability_kind)
    && compiledRulesLines.length > 0
  );
  const displayRulesLines = useMemo(() => {
    if (shouldPreferStackAbilityRules) {
      return compiledRulesLines;
    }
    if (debugInspector && compiledRulesLines.length > 0) {
      return compiledRulesLines;
    }
    if (oracleRulesLines.length > 0) {
      return oracleRulesLines;
    }
    return compiledRulesLines;
  }, [compiledRulesLines, debugInspector, oracleRulesLines, shouldPreferStackAbilityRules]);
  const displayRulesText = displayRulesLines.join("\n");
  const displayObjectName = debugInspector ? null : objectName;
  const displayTypeLine = debugInspector ? null : typeLineDisplay;
  const displayTypeLineBadges = debugInspector ? [] : typeLineBadges;
  const displayZoneLine = debugInspector ? null : zoneLine;
  const displayOwnershipMetadataLines = useMemo(
    () => (debugInspector ? [] : ownershipMetadataLines),
    [debugInspector, ownershipMetadataLines]
  );
  const displayCountersLine = debugInspector ? null : countersLine;
  const displayManaCost = debugInspector ? null : manaCost;
  const displayStatsText = debugInspector ? null : statsText;
  const displayTopLeftDetailLines = useMemo(
    () => [displayTypeLine].filter(Boolean),
    [displayTypeLine]
  );
  const displayTopLeftOwnershipLines = useMemo(
    () => [...displayOwnershipMetadataLines].filter(Boolean),
    [displayOwnershipMetadataLines]
  );
  const displayTopRightDetailLines = useMemo(
    () => [displayCountersLine].filter(Boolean),
    [displayCountersLine]
  );
  const displayBottomRightZoneLines = useMemo(
    () => [displayZoneLine].filter(Boolean),
    [displayZoneLine]
  );
  const metadataText = [
    ...displayTopLeftDetailLines,
    ...displayTypeLineBadges,
    ...displayTopLeftOwnershipLines,
    ...displayTopRightDetailLines,
    ...displayBottomRightZoneLines,
  ].join("\n");
  const rulesRenderKey = useMemo(
    () => [
      objectIdKey || "none",
      debugInspector ? "debug" : "normal",
      showCompiledText ? "compiled" : "oracle",
      displayRulesText,
    ].join("|"),
    [debugInspector, displayRulesText, objectIdKey, showCompiledText]
  );
  const inspectorScaleSessionKey = useMemo(
    () => (
      compact || displayMode !== "inspector"
        ? null
        : [
          objectIdKey || "none",
          displayMode,
          displayStatsText || "",
          transitionTitle || "",
          metadataText || "",
          displayRulesText,
        ].join("|")
    ),
    [compact, displayMode, displayRulesText, displayStatsText, metadataText, objectIdKey, transitionTitle]
  );
  const inspectorTitleScaleSessionKey = useMemo(
    () => (
      compact || displayMode !== "inspector"
        ? null
        : [
          objectIdKey || "none",
          displayMode,
          displayObjectName || "",
          groupedCardCount,
        ].join("|")
    ),
    [compact, displayMode, displayObjectName, groupedCardCount, objectIdKey]
  );
  const preferredInlineWidth = null;
  const preferredInspectorWidth = null;
  const activeMeasuredPreferredInspectorWidth = null;
  const resolvedPreferredInspectorWidth = null;
  const activeInspectorTextScale = compact || displayMode !== "inspector"
    ? 1
    : (inspectorScaleSession.key === inspectorScaleSessionKey ? inspectorScaleSession.scale : 1);
  const activeInspectorTitleScale = compact || displayMode !== "inspector"
    ? 1
    : (
      inspectorTitleScaleSession.key === inspectorTitleScaleSessionKey
        ? inspectorTitleScaleSession.scale
        : activeInspectorTextScale
    );
  const topStackMatchesInspectorObject = useMemo(() => {
    if (!topStackObject) return false;
    if (objectIdNum != null && topStackId === String(objectIdNum)) return true;
    if (detailStableId != null && topStackStableIds.length > 0) {
      if (topStackStableIds.includes(detailStableId)) return true;
    }
    if (objectName && topStackName && topStackName === String(objectName)) return true;
    return false;
  }, [topStackObject, objectIdNum, topStackId, detailStableId, topStackStableIds, objectName, topStackName]);
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

  const copyDebugButton = debugInspector ? (
    <div className="absolute right-3 top-3 z-20 pointer-events-auto">
      <button
        type="button"
        className={`inspector-chip inspector-chip--icon inline-flex h-7 w-7 items-center justify-center rounded-none border bg-[rgba(21,16,13,0.9)] shadow-[0_10px_26px_rgba(0,0,0,0.46)] backdrop-blur-[6px] transition-colors ${
          canCopyDebug
            ? "border-[rgba(181,148,97,0.58)] text-[#ead9b2] hover:border-[#e8cc91] hover:text-[#fff0c8]"
            : "border-[rgba(92,79,61,0.7)] text-[#8f836f] opacity-60"
        }`}
        disabled={!canCopyDebug}
        title={canCopyDebug ? "Copy compiled + raw definition" : "No debug text available"}
        onClick={copyDebugPayload}
      >
        {copiedDebug ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </button>
    </div>
  ) : null;

  const similarityBadge = debugInspector ? (
    <div className="pointer-events-none absolute left-1/2 top-3 z-20 -translate-x-1/2">
      <div
        className="inspector-chip inspector-chip--meta rounded-none border border-[rgba(181,148,97,0.34)] bg-[rgba(22,17,14,0.88)] px-3 py-1 text-[12px] font-extrabold leading-none tracking-[0.08em] text-[#eadfbe] shadow-[0_10px_28px_rgba(0,0,0,0.5)] backdrop-blur-[8px]"
        style={METADATA_TEXT_STYLE}
      >
        {similarityBadgeLabel}
      </div>
    </div>
  ) : null;

  useLayoutEffect(() => {
    if (compact || displayMode !== "inspector" || !displayObjectName) return undefined;

    const banner = inspectorTitleRef.current;
    const titleHost = banner?.parentElement;
    if (!banner || !titleHost || !inspectorTitleScaleSessionKey) return undefined;

    let rafId = null;
    const publishScale = () => {
      const currentScale = Math.max(activeInspectorTitleScale, 0.01);
      const naturalWidth = banner.scrollWidth / currentScale;
      const availableWidth = Math.max(0, Math.floor(titleHost.clientWidth) - 2);

      if (!Number.isFinite(naturalWidth) || naturalWidth <= 0 || availableWidth <= 0) {
        return;
      }

      const fittedScale = clampNumber(
        (availableWidth / naturalWidth) * 0.995,
        minInspectorTitleScale,
        1
      );
      const nextScale = Math.min(activeInspectorTextScale, fittedScale);

      setInspectorTitleScaleSession((currentSession) => {
        const sessionScale = currentSession.key === inspectorTitleScaleSessionKey
          ? currentSession.scale
          : activeInspectorTextScale;
        if (
          currentSession.key === inspectorTitleScaleSessionKey
          && Math.abs(sessionScale - nextScale) < 0.01
        ) {
          return currentSession;
        }
        return {
          key: inspectorTitleScaleSessionKey,
          scale: nextScale,
        };
      });
    };

    const scheduleScale = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        publishScale();
      });
    };

    scheduleScale();
    const observer = new ResizeObserver(scheduleScale);
    observer.observe(banner);
    observer.observe(titleHost);
    window.addEventListener("resize", scheduleScale);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", scheduleScale);
    };
  }, [
    activeInspectorTextScale,
    activeInspectorTitleScale,
    compact,
    displayMode,
    inspectorTitleScaleSessionKey,
    displayObjectName,
    minInspectorTitleScale,
  ]);

  useLayoutEffect(() => {
    if (typeof onProtectedTopChange !== "function") return undefined;
    const leftNode = topHeaderRef.current;
    const rightNode = topMetadataRef.current;
    const overlayNode = leftNode?.parentElement || rightNode?.parentElement || null;
    if (!overlayNode || (!leftNode && !rightNode)) {
      onProtectedTopChange(null);
      return undefined;
    }

    let rafId = null;
    const publishProtectedTop = () => {
      const overlayRect = overlayNode.getBoundingClientRect();
      if (!overlayRect) {
        onProtectedTopChange(null);
        return;
      }
      const candidateBottoms = [leftNode, rightNode]
        .filter(Boolean)
        .map((node) => node.getBoundingClientRect().bottom - overlayRect.top);
      onProtectedTopChange(candidateBottoms.length > 0 ? Math.max(...candidateBottoms) : null);
    };

    publishProtectedTop();
    const observer = new ResizeObserver(() => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(publishProtectedTop);
    });
    if (leftNode) observer.observe(leftNode);
    if (rightNode) observer.observe(rightNode);
    window.addEventListener("resize", publishProtectedTop);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", publishProtectedTop);
      onProtectedTopChange(null);
    };
  }, [
    activeInspectorTextScale,
    displayManaCost,
    displayObjectName,
    metadataText,
    onProtectedTopChange,
    displayStatsText,
  ]);

  useLayoutEffect(() => {
    if (typeof onOracleTextHeightChange !== "function") return undefined;
    const node = oracleContainerRef.current;
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
  }, [
    displayRulesText,
    highlightedRuleLineIndices,
    activeInspectorTextScale,
    metadataText,
    onOracleTextHeightChange,
    displayStatsText,
  ]);

  useLayoutEffect(() => {
    if (typeof onPreferredWidthChange !== "function") return;
    onPreferredWidthChange(preferredInlineWidth);
  }, [onPreferredWidthChange, preferredInlineWidth, objectIdKey]);
  useLayoutEffect(() => {
    if (typeof onPreferredInspectorWidthChange !== "function") return;
    onPreferredInspectorWidthChange(resolvedPreferredInspectorWidth);
  }, [objectIdKey, onPreferredInspectorWidthChange, resolvedPreferredInspectorWidth]);

  useEffect(
    () => () => {
      if (typeof onPreferredWidthChange === "function") {
        onPreferredWidthChange(null);
      }
    },
    [onPreferredWidthChange]
  );
  useEffect(
    () => () => {
      if (typeof onPreferredInspectorWidthChange === "function") {
        onPreferredInspectorWidthChange(null);
      }
    },
    [onPreferredInspectorWidthChange]
  );
  useEffect(() => {
    if (typeof onInspectorAccentChange !== "function") return undefined;
    onInspectorAccentChange(inspectorAccent || null);
  }, [inspectorAccent, onInspectorAccentChange]);
  useEffect(
    () => () => {
      if (typeof onInspectorAccentChange === "function") {
        onInspectorAccentChange(null);
      }
    },
    [onInspectorAccentChange]
  );

  useLayoutEffect(() => {
    if (compact || displayMode !== "inspector") return undefined;

    let rafId = null;
    const scroller = oracleScrollRef.current;
    const content = oracleContainerRef.current;
    if (!scroller || !content) return undefined;

    const publishScale = () => {
      const previousSession = inspectorScaleSession;
      const baseScale = previousSession.key === inspectorScaleSessionKey
        ? previousSession.scale
        : 1;
      const preferredWidth = Number(resolvedPreferredInspectorWidth);
      const availableWidth = Number(availableInspectorWidth);
      let nextScale = baseScale;

      if (
        Number.isFinite(preferredWidth)
        && preferredWidth > 0
        && Number.isFinite(availableWidth)
        && availableWidth > 0
      ) {
        nextScale = Math.min(
          nextScale,
          clampNumber(availableWidth / preferredWidth, minInspectorTextScale, 1)
        );
      }

      const clientHeight = scroller.clientHeight;
      const scrollHeight = scroller.scrollHeight;
      if (clientHeight > 0 && scrollHeight > clientHeight + 1) {
        nextScale = Math.min(nextScale, Math.max(
          minInspectorTextScale,
          baseScale * (clientHeight / scrollHeight)
        ));
      }

      setInspectorScaleSession((currentSession) => {
        const currentScale = currentSession.key === inspectorScaleSessionKey
          ? currentSession.scale
          : 1;
        if (
          currentSession.key === inspectorScaleSessionKey
          && Math.abs(currentScale - nextScale) < 0.01
        ) {
          return currentSession;
        }
        return {
          key: inspectorScaleSessionKey,
          scale: nextScale,
        };
      });
    };

    const scheduleScale = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        publishScale();
      });
    };

    scheduleScale();
    const observer = new ResizeObserver(scheduleScale);
    observer.observe(scroller);
    observer.observe(content);
    window.addEventListener("resize", scheduleScale);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", scheduleScale);
    };
  }, [
    availableInspectorHeight,
    availableInspectorWidth,
    compact,
    displayMode,
    displayRulesText,
    inspectorScaleSession,
    inspectorScaleSessionKey,
    metadataText,
    minInspectorTextScale,
    objectIdKey,
    resolvedPreferredInspectorWidth,
    displayStatsText,
  ]);

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

  const inspectorScale = activeInspectorTextScale;
  const inspectorTitleScale = activeInspectorTitleScale;
  const oracleContainerClass = compact
    ? "relative z-10 flex flex-col items-center px-2.5"
    : "relative z-10 min-h-full flex flex-col items-center justify-end";
  const hasTopLeftMetadata = Boolean(
    transitionTitle
    || displayStatsText
    || displayTopLeftDetailLines.length > 0
    || displayTypeLineBadges.length > 0
    || displayTopLeftOwnershipLines.length > 0
  );
  const inspectorTopMetadataReserve = debugInspector
    ? 52
    : (
      16
      + (transitionTitle ? 28 * inspectorScale : 0)
      + (displayStatsText ? ((INSPECTOR_STATS_FONT_SIZE * inspectorScale) + 14) : 0)
      + (displayTopLeftDetailLines.length * ((INSPECTOR_METADATA_FONT_SIZE * inspectorScale * 1.35) + 14))
      + (displayTypeLineBadges.length > 0 ? ((18 * inspectorScale) + 10) : 0)
      + (displayTopLeftOwnershipLines.length * ((INSPECTOR_METADATA_FONT_SIZE * inspectorScale * 1.35) + 8))
      + (hasTopLeftMetadata ? 36 * inspectorScale : 0)
    );
  const inspectorOracleTopPadding = debugInspector
    ? 52
    : Math.max(
      INSPECTOR_ORACLE_TOP_PADDING + (transitionTitle ? 28 : 0),
      inspectorTopMetadataReserve
    );
  const compactOraclePaddingTop = debugInspector
    ? 52
    : (
      14
      + (transitionTitle ? 24 : 0)
      + (displayManaCost ? 22 : 0)
      + (displayStatsText ? 22 : 0)
      + (displayTopLeftDetailLines.length > 0 ? 20 : 0)
      + (displayTypeLineBadges.length > 0 ? 18 : 0)
      + (displayTopLeftOwnershipLines.length > 0 ? 16 : 0)
      + (hasTopLeftMetadata ? 28 : 0)
    );
  const compactOraclePaddingBottom = (
    12
      + (displayObjectName ? 30 : 0)
      + ((displayTopLeftOwnershipLines.length + displayBottomRightZoneLines.length) * 16)
  );
  const topMetadataTextClassName = compact
    ? "text-[11px] leading-snug text-[#d1e2f6] text-left"
    : "leading-snug text-[#d1e2f6] text-left";
  const bottomMetadataTextClassName = compact
    ? "text-[11px] leading-snug text-[#d1e2f6] text-center"
    : "leading-snug text-[#d1e2f6] text-center";
  const rulesTextClassName = compact
    ? "text-[13px] leading-[1.28] text-white font-semibold text-left"
    : "text-white font-semibold text-left";
  const inspectorTitleStyle = compact ? undefined : {
    fontSize: `${INSPECTOR_TITLE_FONT_SIZE * inspectorTitleScale}px`,
    padding: `${6 * inspectorTitleScale}px ${12 * inspectorTitleScale}px`,
  };
  const inspectorTopMetaStyle = compact ? undefined : {
    padding: `${4 * inspectorScale}px ${10 * inspectorScale}px`,
    fontSize: `${INSPECTOR_METADATA_FONT_SIZE * inspectorScale}px`,
  };
  const inspectorStatsStyle = compact ? undefined : {
    padding: `${4 * inspectorScale}px ${10 * inspectorScale}px`,
    fontSize: `${INSPECTOR_STATS_FONT_SIZE * inspectorScale}px`,
  };
  const inspectorManaStyle = compact ? undefined : {
    padding: `${4 * inspectorScale}px ${8 * inspectorScale}px`,
  };
  const inspectorBottomOverlayPadding = compact
    ? compactOraclePaddingBottom
    : (
      (displayObjectName ? ((INSPECTOR_TITLE_FONT_SIZE * inspectorTitleScale) + 28) : 0)
      + ((displayTopLeftOwnershipLines.length + displayBottomRightZoneLines.length) * ((INSPECTOR_METADATA_FONT_SIZE * inspectorScale * 1.35) + 4))
      + 22
    );
  const inspectorOracleContainerStyle = compact ? undefined : {
    paddingTop: `${inspectorOracleTopPadding * inspectorScale}px`,
    paddingBottom: `${Math.max(INSPECTOR_ORACLE_BOTTOM_PADDING * inspectorScale, inspectorBottomOverlayPadding)}px`,
    paddingLeft: `${10 * inspectorScale}px`,
    paddingRight: `${10 * inspectorScale}px`,
  };
  const resolvedOracleContainerStyle = compact
    ? { paddingTop: `${compactOraclePaddingTop}px`, paddingBottom: `${compactOraclePaddingBottom}px` }
    : inspectorOracleContainerStyle;
  const rulesTextStyle = compact ? ORACLE_TEXT_STYLE : {
    ...ORACLE_TEXT_STYLE,
    fontSize: `${INSPECTOR_RULES_FONT_SIZE * inspectorScale}px`,
    lineHeight: INSPECTOR_RULES_LINE_HEIGHT / INSPECTOR_RULES_FONT_SIZE,
  };

  const showImageBackdrop = !!imageUrl && !imageErrored;
  const hasRenderableContent = Boolean(
    transitionTitle
    || displayObjectName
    || displayTypeLine
    || displayTypeLineBadges.length > 0
    || displayZoneLine
    || displayTopRightDetailLines.length > 0
    || displayManaCost
    || displayStatsText
    || displayRulesLines.length > 0
  );

  if (isFullArtMode) {
    return (
      <div
        className="hover-art-stage hover-art-drop-in absolute inset-0 z-30 overflow-hidden pointer-events-auto"
      >
        <div className="absolute inset-0 bg-[radial-gradient(92%_92%_at_50%_14%,rgba(188,150,92,0.28),rgba(8,13,20,0)_62%),linear-gradient(180deg,rgba(16,12,9,0.96),rgba(8,7,7,0.98))]" />
        <div className="absolute inset-[10px] overflow-hidden rounded-none border border-[rgba(177,145,98,0.38)] bg-[rgba(16,12,10,0.94)] shadow-[0_0_0_1px_rgba(196,164,112,0.12),0_0_28px_rgba(156,118,62,0.18),0_28px_52px_rgba(0,0,0,0.48)]">
          <div className="absolute inset-0 bg-[radial-gradient(78%_62%_at_50%_24%,rgba(210,178,112,0.12),rgba(6,10,16,0)_62%)]" />
          <div className="absolute inset-[10px] rounded-none border border-white/6 bg-[linear-gradient(180deg,rgba(255,255,255,0.04),rgba(255,255,255,0.01))]" />
          {showImageBackdrop && (
            <InspectorArtImageLayers
              imageUrl={imageUrl}
              objectName={objectName}
              fullArt
              onError={setFailedImageUrl}
            />
          )}
          {copyDebugButton}
          {similarityBadge}
          {!showImageBackdrop && !hasRenderableContent && (
            <div className="absolute inset-0 flex items-center justify-center px-6 text-center text-[13px] font-semibold uppercase tracking-[0.14em] text-[#dbc9a3]">
              Card details unavailable
            </div>
          )}
        </div>
        {!debugInspector && (
          <div className="pointer-events-auto absolute inset-x-3 top-3 z-10 flex items-start justify-between gap-2">
            <div className="flex max-w-[72%] flex-col items-start gap-1.5">
              {transitionTitle && (
                <div
                  className="inspector-chip inspector-chip--meta flex items-center gap-1 rounded-none border border-[rgba(142,181,220,0.36)] bg-[rgba(12,20,31,0.82)] px-2 py-1 text-[11px] font-extrabold leading-none tracking-[0.14em] text-[#d8ebff] shadow-[0_0_18px_rgba(90,148,211,0.14)] backdrop-blur-[10px]"
                  style={METADATA_TEXT_STYLE}
                >
                  {hasTransitionNavigator && (
                    <button
                      type="button"
                      className="pointer-events-auto inline-flex h-5 w-5 items-center justify-center border border-[#9bc6ec]/40 bg-[rgba(4,9,16,0.45)] text-[#d8ebff] transition-colors hover:bg-[rgba(34,56,80,0.72)]"
                      onPointerDown={handleInspectorChevronPointerDown}
                      onClick={(event) => handleInspectorChevronClick(onShowPreviousTransientPreview, event)}
                      aria-label="Show previous moved card"
                    >
                      <ChevronLeft className="h-3.5 w-3.5" />
                    </button>
                  )}
                  <span>{transitionTitle}</span>
                  {transitionSequenceLabel && (
                    <span className="border border-[#9bc6ec]/30 bg-[rgba(4,9,16,0.42)] px-1.5 py-0.5 text-[10px] tracking-[0.12em] text-[#cae5ff]">
                      {transitionSequenceLabel}
                    </span>
                  )}
                  {hasTransitionNavigator && (
                    <button
                      type="button"
                      className="pointer-events-auto inline-flex h-5 w-5 items-center justify-center border border-[#9bc6ec]/40 bg-[rgba(4,9,16,0.45)] text-[#d8ebff] transition-colors hover:bg-[rgba(34,56,80,0.72)]"
                      onPointerDown={handleInspectorChevronPointerDown}
                      onClick={(event) => handleInspectorChevronClick(onShowNextTransientPreview, event)}
                      aria-label="Show next moved card"
                    >
                      <ChevronRight className="h-3.5 w-3.5" />
                    </button>
                  )}
                </div>
              )}
              {displayStatsText && (
                <div
                  className="inspector-chip inspector-chip--stats rounded-none border border-[#f5d08b]/34 bg-[rgba(30,21,13,0.82)] px-2.5 py-1 text-[14px] font-extrabold leading-none tracking-[0.08em] text-[#f8d98e] shadow-[0_0_16px_rgba(245,208,139,0.1)] backdrop-blur-[10px]"
                  style={METADATA_TEXT_STYLE}
                >
                  {displayStatsText}
                </div>
              )}
              <InspectorMetadataBlock
                lines={displayTopLeftDetailLines}
                className="inspector-chip inspector-chip--meta max-w-full self-start rounded-none border border-[rgba(174,145,98,0.28)] bg-[rgba(24,18,14,0.76)] px-3 py-2 text-left text-[12px] font-semibold leading-tight text-[#e0d1b2] shadow-[0_0_18px_rgba(185,150,93,0.08)] backdrop-blur-[10px]"
                style={METADATA_TEXT_STYLE}
              />
              {displayTypeLineBadges.length > 0 && (
                <div className="flex max-w-full flex-wrap gap-1">
                  {displayTypeLineBadges.map((badge) => (
                    <span
                      key={badge}
                      className="inspector-chip inspector-chip--meta rounded-none border border-[rgba(142,181,220,0.42)] bg-[rgba(12,20,31,0.72)] px-2 py-1 text-[10px] font-extrabold uppercase leading-none tracking-[0.12em] text-[#d8ebff] shadow-[0_0_16px_rgba(90,148,211,0.12)] backdrop-blur-[10px]"
                      style={METADATA_TEXT_STYLE}
                      title={badge === "All creature types" ? "This object has every creature type." : badge}
                    >
                      {badge}
                    </span>
                  ))}
                </div>
              )}
            </div>
            {(displayTopRightDetailLines.length > 0 || displayManaCost) && (
              <div className="flex shrink-0 flex-col items-end gap-1">
                {displayManaCost && (
                  <div className="inspector-chip inspector-chip--mana rounded-none border border-[rgba(174,145,98,0.3)] bg-[rgba(24,18,14,0.78)] px-2.5 py-1 shadow-[0_0_16px_rgba(185,150,93,0.1)] backdrop-blur-[10px]">
                    <ManaCostIcons cost={displayManaCost} size={16} />
                  </div>
                )}
                <InspectorMetadataBlock
                  lines={displayTopRightDetailLines}
                  className="inspector-chip inspector-chip--meta max-w-full self-end rounded-none border border-[rgba(174,145,98,0.28)] bg-[rgba(24,18,14,0.76)] px-3 py-2 text-right text-[12px] font-semibold leading-tight text-[#e0d1b2] shadow-[0_0_18px_rgba(185,150,93,0.08)] backdrop-blur-[10px]"
                  style={METADATA_TEXT_STYLE}
                />
              </div>
            )}
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
      <div className="absolute inset-0 bg-[radial-gradient(120%_84%_at_50%_18%,rgba(188,150,92,0.16),rgba(6,11,18,0)_52%),linear-gradient(180deg,rgba(16,12,9,0.94),rgba(7,8,9,0.98))]" />
      {showImageBackdrop && (
        <div className="hover-art-slice-in absolute inset-0">
          <InspectorArtImageLayers
            imageUrl={imageUrl}
            objectName={objectName}
            onError={setFailedImageUrl}
          />
        </div>
      )}
      {copyDebugButton}
      <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,rgba(0,0,0,0.08)_0%,rgba(0,0,0,0.16)_48%,rgba(0,0,0,0.3)_100%)]" />
        <div className="absolute inset-0 overflow-hidden">
          <div className="pointer-events-none absolute inset-x-0 bottom-0 top-[34%] bg-[linear-gradient(180deg,rgba(0,0,0,0)_0%,rgba(0,0,0,0.52)_46%,rgba(0,0,0,0.74)_100%)]" />
        {(transitionTitle || displayStatsText || displayTopLeftDetailLines.length > 0 || displayTopLeftOwnershipLines.length > 0) && (
          <div ref={topHeaderRef} className="pointer-events-auto absolute top-0 left-0 z-10 flex max-w-[72%] flex-col items-start gap-1">
            {transitionTitle && (
              <div
                className={cn(
                  "inspector-banner inspector-banner--meta flex items-center gap-1 rounded-none bg-[rgba(8,18,30,0.72)] px-2 py-1 font-extrabold tracking-[0.12em] text-[#d8ebff] backdrop-blur-[1.8px]",
                  topMetadataTextClassName
                )}
                style={{ ...METADATA_TEXT_STYLE, ...inspectorTopMetaStyle }}
              >
                {hasTransitionNavigator && (
                  <button
                    type="button"
                    className="pointer-events-auto inline-flex h-5 w-5 items-center justify-center rounded-none border border-[#9bc6ec]/40 bg-[rgba(4,9,16,0.45)] text-[#d8ebff] transition-colors hover:bg-[rgba(34,56,80,0.72)]"
                    onPointerDown={handleInspectorChevronPointerDown}
                    onClick={(event) => handleInspectorChevronClick(onShowPreviousTransientPreview, event)}
                    aria-label="Show previous moved card"
                  >
                    <ChevronLeft className="h-3.5 w-3.5" />
                  </button>
                )}
                <span>{transitionTitle}</span>
                {transitionSequenceLabel && (
                  <span className="rounded-none border border-[#9bc6ec]/30 bg-[rgba(4,9,16,0.42)] px-1.5 py-0.5 text-[10px] tracking-[0.12em] text-[#cae5ff]">
                    {transitionSequenceLabel}
                  </span>
                )}
                {hasTransitionNavigator && (
                  <button
                    type="button"
                    className="pointer-events-auto inline-flex h-5 w-5 items-center justify-center rounded-none border border-[#9bc6ec]/40 bg-[rgba(4,9,16,0.45)] text-[#d8ebff] transition-colors hover:bg-[rgba(34,56,80,0.72)]"
                    onPointerDown={handleInspectorChevronPointerDown}
                    onClick={(event) => handleInspectorChevronClick(onShowNextTransientPreview, event)}
                    aria-label="Show next moved card"
                  >
                    <ChevronRight className="h-3.5 w-3.5" />
                  </button>
                )}
              </div>
            )}
            {displayStatsText && (
              <div
                className={cn(
                  "inspector-banner inspector-banner--stats rounded-none bg-[rgba(0,0,0,0.52)] px-2.5 py-1 text-[#f8d98e] tracking-wide backdrop-blur-[1.8px]",
                  compact ? "text-[15px] font-extrabold leading-none" : "text-[20px] font-extrabold leading-none"
                )}
                style={{ ...METADATA_TEXT_STYLE, ...inspectorStatsStyle }}
              >
                {displayStatsText}
              </div>
            )}
            <InspectorMetadataBlock
              lines={displayTopLeftDetailLines}
              className={cn(
                "inspector-banner inspector-banner--meta self-start rounded-none bg-[rgba(0,0,0,0.48)] px-2.5 py-1 backdrop-blur-[1.8px]",
                topMetadataTextClassName
              )}
              lineClassName="text-left"
              style={{ ...METADATA_TEXT_STYLE, ...inspectorTopMetaStyle }}
            />
            {displayTypeLineBadges.length > 0 && (
              <div className="flex max-w-full flex-wrap gap-1">
                {displayTypeLineBadges.map((badge) => (
                  <span
                    key={badge}
                    className={cn(
                      "inspector-banner inspector-banner--meta rounded-none bg-[rgba(8,18,30,0.62)] px-2 py-1 font-extrabold uppercase leading-none tracking-[0.12em] text-[#d8ebff] backdrop-blur-[1.8px]",
                      compact ? "text-[9px]" : "text-[10px]"
                    )}
                    style={{ ...METADATA_TEXT_STYLE, ...inspectorTopMetaStyle }}
                    title={badge === "All creature types" ? "This object has every creature type." : badge}
                  >
                    {badge}
                  </span>
                ))}
              </div>
            )}
            <InspectorMetadataBlock
              lines={displayTopLeftOwnershipLines}
              className={cn(
                "inspector-banner inspector-banner--meta self-start rounded-none bg-[rgba(0,0,0,0.48)] px-2.5 py-1 backdrop-blur-[1.8px]",
                topMetadataTextClassName
              )}
              lineClassName="whitespace-nowrap text-left leading-none"
              style={{
                ...METADATA_TEXT_STYLE,
                ...inspectorTopMetaStyle,
                fontSize: compact ? "10px" : "clamp(8px, 0.72vw, 12px)",
              }}
            />
          </div>
        )}
        {(displayManaCost || displayTopRightDetailLines.length > 0) && (
          <div ref={topMetadataRef} className="absolute top-0 right-0 z-10 flex max-w-[40%] flex-col items-end gap-0">
            {displayManaCost && (
              <div className="flex items-center justify-end gap-1.5">
                <div className="inspector-banner inspector-banner--mana rounded-none bg-[rgba(0,0,0,0.52)] px-2 py-1" style={inspectorManaStyle}>
                  <ManaCostIcons cost={displayManaCost} size={compact ? 14 : Math.max(13, Math.round(18 * inspectorScale))} />
                </div>
              </div>
            )}
            <InspectorMetadataBlock
              lines={displayTopRightDetailLines}
              className={cn(
                "inspector-banner inspector-banner--meta mt-1 self-end rounded-none bg-[rgba(0,0,0,0.48)] px-2.5 py-1 text-right backdrop-blur-[1.8px]",
                topMetadataTextClassName
              )}
              lineClassName="text-right"
              style={{ ...METADATA_TEXT_STYLE, ...inspectorTopMetaStyle }}
            />
          </div>
        )}
        <div
          key={rulesRenderKey}
          ref={oracleScrollRef}
          className="inspector-oracle-scroll absolute inset-x-0 top-0 overflow-y-auto pointer-events-auto overscroll-contain touch-pan-y"
          style={{ bottom: `${Math.max(0, stackTimelineHeight - 4)}px` }}
        >
          <div ref={oracleContainerRef} className={oracleContainerClass} style={resolvedOracleContainerStyle}>
            <div ref={oracleBodyRef} className="space-y-1 w-full max-w-[min(92%,44rem)] self-center text-left">
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
                          /^\s*[•*-]\s+/.test(String(line || "")) && "inspector-oracle-line-bullet"
                        )}
                        style={rulesTextStyle}
                      />
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
        {displayObjectName && (
          <div className="pointer-events-none absolute bottom-0 left-0 z-10 flex max-w-[72%] items-end px-3 pb-3">
            <div
              ref={inspectorTitleRef}
              className={cn(
                "inspector-banner inspector-banner--title max-w-full overflow-hidden rounded-none bg-[linear-gradient(90deg,rgba(0,0,0,0.66)_0%,rgba(0,0,0,0.44)_82%,rgba(0,0,0,0.12)_100%)] px-3 py-1.5 font-extrabold leading-[1.02] tracking-[0.02em] text-[#f3f8ff] backdrop-blur-[2px]",
                compact ? "text-[17px]" : "text-[22px]"
              )}
              style={inspectorTitleStyle}
            >
              <span className="inline-flex max-w-full items-center gap-2 whitespace-nowrap">
                {groupedCardCount > 1 && (
                  <span className="inspector-chip-count inline-flex h-5 min-w-5 items-center justify-center rounded-none border border-[#f5d08b]/70 bg-[rgba(0,0,0,0.45)] px-1 text-[12px] font-bold leading-none tracking-wide text-[#f5d08b]">
                    x{groupedCardCount}
                  </span>
                )}
                <span>{displayObjectName}</span>
              </span>
            </div>
          </div>
        )}
        {displayBottomRightZoneLines.length > 0 && (
          <div className="pointer-events-none absolute bottom-0 right-0 z-10 flex max-w-[48%] items-end px-3 pb-3">
            <InspectorMetadataBlock
              lines={displayBottomRightZoneLines}
              className={cn(
                "inspector-banner inspector-banner--meta rounded-none bg-[rgba(0,0,0,0.48)] px-3 py-2 text-right backdrop-blur-[1.8px]",
                topMetadataTextClassName
              )}
              lineClassName="text-right"
              style={{ ...METADATA_TEXT_STYLE, ...inspectorTopMetaStyle }}
            />
          </div>
        )}
        {!showImageBackdrop && !hasRenderableContent && (
          <div className="absolute inset-0 flex items-center justify-center px-5 text-center text-[12px] font-semibold uppercase tracking-[0.14em] text-[#b5d3f2]">
            Card details unavailable
          </div>
        )}
      </div>
      {similarityBadge}
    </div>
  );
}
