import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Check, Copy } from "lucide-react";

const ORACLE_TEXT_STYLE = {
  textShadow: "0 1px 2px rgba(0, 0, 0, 0.98), 0 3px 12px rgba(0, 0, 0, 0.9), 0 0 2px rgba(0, 0, 0, 0.95)",
  WebkitTextStroke: "0.45px rgba(3, 7, 14, 0.95)",
};

const METADATA_TEXT_STYLE = {
  textShadow: "0 1px 2px rgba(0, 0, 0, 0.96), 0 2px 10px rgba(0, 0, 0, 0.84)",
};

function simplifyActionLabel(label = "") {
  const activateMatch = String(label).match(/^Activate\s+.+?:\s*(.+)$/i);
  if (activateMatch) return activateMatch[1];
  return label;
}

function isInspectorDecision(decision) {
  return (
    !!decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
}

function inspectorDecisionTitle(decision) {
  if (!decision) return "Decision";
  switch (decision.kind) {
    case "targets":
      return "Choose Targets";
    case "select_objects":
      return "Choose Objects";
    case "select_options":
      return "Choose Option";
    case "number":
      return "Choose Number";
    default:
      return "Decision";
  }
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

function actionTargetsObjectName(action, lowerName) {
  if (!lowerName) return false;
  const label = String(action?.label || "").trim().toLowerCase();
  if (!label) return false;
  return (
    label.startsWith(`activate ${lowerName}:`)
    || label.startsWith(`cast ${lowerName}`)
    || label.startsWith(`play ${lowerName}`)
  );
}

export default function HoverArtOverlay({
  objectId,
  suppressStableId = null,
  submitAction = null,
  onProtectedTopChange = null,
  onOracleTextHeightChange = null,
  onInspectorSubmitChange = null,
}) {
  const { state, game, inspectorDebug, dispatch, cancelDecision } = useGame();
  const objectNameById = useMemo(() => buildObjectNameMap(state), [state]);
  const objectIdNum = objectId != null ? Number(objectId) : null;
  const objectIdKey = Number.isFinite(objectIdNum) ? String(objectIdNum) : null;
  const topHeaderRef = useRef(null);
  const oracleBodyRef = useRef(null);

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
  const decision = state?.decision || null;
  const canAct = !!decision && decision.player === state?.perspective;
  const inspectorDecision = isInspectorDecision(decision) && canAct
    ? decision
    : null;
  const topStackObject = (state?.stack_objects || [])[0] || null;
  const hasStackEntries = (state?.stack_objects?.length || 0) > 0 || (state?.stack_preview?.length || 0) > 0;
  const decisionSourceName = decision && decision.kind !== "priority" && decision.kind !== "attackers" && decision.kind !== "blockers"
    ? decision.source_name || null
    : null;
  const isBattlefieldSource = String(details?.zone || "").toLowerCase() === "battlefield";
  const hideOracleText = Boolean(
    decision
    && decision.player === state?.perspective
    && decision.kind !== "priority"
    && decisionSourceName
    && details?.name
    && decisionSourceName === details.name
    && isBattlefieldSource
  ) || !!inspectorDecision;
  const selectedObjectNameLower = String(objectName || "").trim().toLowerCase();
  const objectFamilyIds = useMemo(
    () => buildObjectFamilyIds(state, objectIdNum),
    [state, objectIdNum]
  );
  const inspectorActions = useMemo(() => {
    if (!decision || decision.kind !== "priority") return [];
    if (decision.player !== state?.perspective) return [];
    if (!Number.isFinite(objectIdNum) && !selectedObjectNameLower) return [];
    const matched = [];
    for (const action of decision.actions || []) {
      if (action.kind === "pass_priority") continue;
      const actionObjectId = Number(action.object_id);
      if (Number.isFinite(actionObjectId) && objectFamilyIds.has(actionObjectId)) {
        matched.push(action);
        continue;
      }
      if (actionTargetsObjectName(action, selectedObjectNameLower)) {
        matched.push(action);
      }
    }
    const seen = new Set();
    return matched.filter((action) => {
      if (seen.has(action.index)) return false;
      seen.add(action.index);
      return true;
    });
  }, [decision, state?.perspective, objectIdNum, objectFamilyIds, selectedObjectNameLower]);

  const suppressObject =
    suppressStableId != null
    && details != null
    && Number(details.stable_id) === Number(suppressStableId);

  const semanticScore = Number(details?.semantic_score);
  const hasSemanticScore = Number.isFinite(semanticScore);
  const compiledText = Array.isArray(details?.abilities) && details.abilities.length > 0
    ? details.abilities.join("\n")
    : (oracleText || "");
  const displayRulesText = (
    String(compiledText || "").trim()
    || String(hoveredStackObject?.ability_text || "").trim()
    || String(hoveredStackObject?.effect_text || "").trim()
    || String(oracleText || "").trim()
  );
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

  const triggerInspectorAction = useCallback((event, action) => {
    event.preventDefault();
    event.stopPropagation();
    if (!action) return;
    dispatch(
      { type: "priority_action", action_index: action.index },
      action.label
    );
  }, [dispatch]);

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
  }, [onOracleTextHeightChange, displayRulesText, metadataText, statsText, hideOracleText]);

  const resolvingEffectText = useMemo(() => {
    if (!inspectorDecision) return null;
    const kind = String(topStackObject?.ability_kind || "").trim().toLowerCase();
    if (kind.includes("trigger")) return "Triggered ability";
    if (kind.includes("activat")) return "Activated ability";
    if (kind.includes("mana")) return "Mana ability";
    return "Spell";
  }, [inspectorDecision, topStackObject?.ability_kind]);
  const inspectorDecisionSubtitle = inspectorDecision
    ? [inspectorDecision.source_name || topStackObject?.name || objectName, resolvingEffectText]
      .filter(Boolean)
      .join(" · ")
    : null;
  const oracleTopPaddingClass = inspectorDecision
    ? "pt-[382px]"
    : inspectorActions.length > 0
      ? "pt-[246px]"
      : "pt-[164px]";
  const submitLabel = submitAction?.label || "Submit";
  const canSubmit = canAct
    && !!submitAction
    && !submitAction.disabled
    && typeof submitAction.onSubmit === "function";

  useEffect(() => {
    if (!onInspectorSubmitChange || inspectorDecision) return;
    onInspectorSubmitChange(null);
  }, [onInspectorSubmitChange, inspectorDecision]);

  useEffect(
    () => () => {
      if (onInspectorSubmitChange) onInspectorSubmitChange(null);
    },
    [onInspectorSubmitChange]
  );

  if (!imageUrl || imageErrored || suppressObject) return null;

  return (
    <div
      key={imageUrl}
      className="hover-art-stage hover-art-drop-in absolute inset-0 z-30 pointer-events-none overflow-hidden"
    >
      <div className="hover-art-slice-in hover-art-media absolute inset-0">
        <img
          key={imageUrl}
          src={imageUrl}
          alt={objectName || "Card art"}
          className="hover-art-pan h-full w-full object-cover"
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setFailedImageUrl(imageUrl)}
        />
      </div>
      <div className="absolute inset-0 bg-[linear-gradient(180deg,rgba(4,8,14,0.05)_0%,rgba(4,8,14,0.2)_50%,rgba(4,8,14,0.66)_100%)]" />
      <div className="absolute inset-0 overflow-hidden">
        <div className="absolute inset-x-0 bottom-0 top-[30%] bg-[linear-gradient(180deg,rgba(4,8,14,0)_0%,rgba(4,8,14,0.76)_44%,rgba(4,8,14,0.96)_100%)] backdrop-blur-[1.6px]" />
        <div ref={topHeaderRef} className="absolute top-0 left-0 z-10 flex flex-col items-start gap-0">
          {objectName && (
            <div className="bg-[rgba(5,11,20,0.8)] px-3 py-1.5 text-[22px] font-extrabold leading-[1.02] tracking-[0.02em] text-[#f3f8ff]">
              {objectName}
            </div>
          )}
          {manaCost && (
            <div className="bg-[rgba(5,11,20,0.8)] px-3 py-1.5">
              <ManaCostIcons cost={manaCost} size={20} />
            </div>
          )}
        </div>
        {inspectorActions.length > 0 && (
          <div className="absolute top-[88px] left-2 right-2 z-[11] overflow-hidden rounded border border-[#5f7f9f] bg-[rgba(7,16,26,0.88)] shadow-[0_12px_26px_rgba(0,0,0,0.5)] pointer-events-auto backdrop-blur-[1.5px]">
            <div className="border-b border-[#35506b] px-2.5 py-1.5">
              <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#8cc4ff]">
                Choose Action
              </div>
              <div className="text-[11px] text-[#b8d2ef]">
                {inspectorActions.length} option{inspectorActions.length === 1 ? "" : "s"}
              </div>
            </div>
            <div className="max-h-[152px] overflow-y-auto divide-y divide-[#2f4965]">
              {inspectorActions.map((action) => (
                <button
                  key={action.index}
                  type="button"
                  data-inspector-action="true"
                  className="w-full px-3 py-1.5 text-left transition-colors hover:bg-[rgba(18,35,54,0.9)]"
                  onClick={(event) => triggerInspectorAction(event, action)}
                >
                  <SymbolText
                    text={normalizeDecisionText(simplifyActionLabel(action.label || ""))}
                    className="block text-[18px] font-extrabold leading-[1.12] text-[#f2f8ff]"
                    style={ORACLE_TEXT_STYLE}
                  />
                </button>
              ))}
            </div>
          </div>
        )}
        {inspectorDecision && (
          <div
            className={cn(
              "absolute inset-x-2 top-[88px] z-[12] min-h-0 overflow-hidden rounded border border-[#5d7ea0] bg-[linear-gradient(180deg,rgba(6,14,22,0.76),rgba(6,14,22,0.9))] shadow-[0_16px_34px_rgba(0,0,0,0.55)] pointer-events-auto backdrop-blur-[2.2px] flex flex-col",
              hasStackEntries ? "bottom-[176px]" : "bottom-[8px]"
            )}
          >
            <div className="border-b border-[#3c5876] bg-[rgba(8,19,31,0.9)] px-2.5 py-1.5">
              <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#8cc4ff]">
                {inspectorDecisionTitle(inspectorDecision)}
              </div>
              {inspectorDecisionSubtitle && (
                <SymbolText
                  text={normalizeDecisionText(inspectorDecisionSubtitle)}
                  className="mt-0.5 block text-[13px] leading-snug text-[#d2e5fb]"
                />
              )}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-1.5 py-1">
              <DecisionRouter
                decision={inspectorDecision}
                canAct={canAct}
                inspectorOracleTextHeight={0}
                inlineSubmit={false}
                onSubmitActionChange={onInspectorSubmitChange}
                hideDescription
              />
            </div>
            <article className="shrink-0 border-t border-[#3c5876] bg-[rgba(8,18,30,0.88)] px-2 py-1.5">
              <div className="text-[10px] font-bold uppercase tracking-[0.14em] text-[#9dccff]">
                Current Decision
              </div>
              <div className="mt-1 grid grid-cols-2 gap-1.5">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 rounded-sm border border-[#3d6ea5] bg-[rgba(40,84,136,0.78)] px-2 text-[12px] font-bold tracking-wide text-[#d9ecff] transition-colors hover:bg-[rgba(58,114,182,0.9)]"
                  disabled={!canSubmit}
                  onClick={() => {
                    if (!canSubmit) return;
                    submitAction.onSubmit();
                  }}
                >
                  {submitLabel}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 rounded-sm border border-[#8b3f4a] bg-[rgba(120,35,46,0.76)] px-2 text-[12px] font-bold uppercase tracking-wide text-[#ffd8df] transition-colors hover:bg-[rgba(163,50,64,0.9)]"
                  disabled={!canAct}
                  onClick={() => {
                    cancelDecision();
                  }}
                >
                  Cancel
                </Button>
              </div>
            </article>
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

        {!inspectorDecision && (
          <div className="absolute inset-x-0 top-0 bottom-[172px] overflow-y-auto">
            <div className={`relative z-10 min-h-full flex flex-col justify-end px-2.5 ${oracleTopPaddingClass} pb-2.5`}>
              <div ref={oracleBodyRef} className="space-y-1">
                {statsText && (
                  <div
                    className="text-[20px] font-extrabold leading-none text-[#f8d98e] tracking-wide text-right"
                    style={METADATA_TEXT_STYLE}
                  >
                    {statsText}
                  </div>
                )}
                {metadataText && (
                  <div
                    className="text-[15px] leading-snug text-[#d1e2f6]"
                    style={METADATA_TEXT_STYLE}
                  >
                    {metadataText}
                  </div>
                )}
                {displayRulesText && (
                  <SymbolText
                    text={hideOracleText ? "" : displayRulesText}
                    className="text-[18px] leading-[1.32] text-[#ecf4ff] block"
                    style={ORACLE_TEXT_STYLE}
                  />
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
