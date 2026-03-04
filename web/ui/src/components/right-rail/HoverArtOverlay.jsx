import { useCallback, useEffect, useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";
import { Check, Copy } from "lucide-react";

const ORACLE_TEXT_STYLE = {
  textShadow: "0 1px 2px rgba(0, 0, 0, 0.98), 0 3px 12px rgba(0, 0, 0, 0.9), 0 0 2px rgba(0, 0, 0, 0.95)",
  WebkitTextStroke: "0.45px rgba(3, 7, 14, 0.95)",
};

const METADATA_TEXT_STYLE = {
  textShadow: "0 1px 2px rgba(0, 0, 0, 0.96), 0 2px 10px rgba(0, 0, 0, 0.84)",
};

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

export default function HoverArtOverlay({ objectId, suppressStableId = null }) {
  const { state, game, inspectorDebug } = useGame();
  const objectNameById = useMemo(() => buildObjectNameMap(state), [state]);
  const objectIdNum = objectId != null ? Number(objectId) : null;
  const objectIdKey = Number.isFinite(objectIdNum) ? String(objectIdNum) : null;

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
  );

  const suppressObject =
    suppressStableId != null
    && details != null
    && Number(details.stable_id) === Number(suppressStableId);

  const semanticScore = Number(details?.semantic_score);
  const hasSemanticScore = Number.isFinite(semanticScore);
  const compiledText = Array.isArray(details?.abilities) && details.abilities.length > 0
    ? details.abilities.join("\n")
    : (oracleText || "");
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

  if (!imageUrl || imageErrored || suppressObject) return null;

  return (
    <div className="absolute inset-0 z-30 pointer-events-none overflow-hidden">
      <img
        key={imageUrl}
        src={imageUrl}
        alt={objectName || "Card art"}
        className="hover-art-slice-in h-full w-full object-cover"
        loading="lazy"
        referrerPolicy="no-referrer"
        onError={() => setFailedImageUrl(imageUrl)}
      />
      <div className="absolute inset-0 bg-[linear-gradient(180deg,rgba(4,8,14,0.05)_0%,rgba(4,8,14,0.2)_50%,rgba(4,8,14,0.66)_100%)]" />
      <div className="absolute inset-0 overflow-hidden">
        <div className="absolute inset-x-0 bottom-0 top-[30%] bg-[linear-gradient(180deg,rgba(4,8,14,0)_0%,rgba(4,8,14,0.76)_44%,rgba(4,8,14,0.96)_100%)] backdrop-blur-[1.6px]" />
        <div className="absolute top-0 left-0 z-10 flex flex-col items-start gap-0">
          {objectName && (
            <div className="bg-[rgba(5,11,20,0.78)] px-2 py-1 text-[14px] font-bold leading-tight text-[#f3f8ff]">
              {objectName}
            </div>
          )}
          {manaCost && (
            <div className="bg-[rgba(5,11,20,0.78)] px-2 py-1">
              <ManaCostIcons cost={manaCost} size={15} />
            </div>
          )}
        </div>
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

        <div className="absolute inset-0 overflow-y-auto">
          <div className="relative z-10 min-h-full flex flex-col justify-end px-2 pt-[72px] pb-2">
            <div className="space-y-1">
              {statsText && (
                <div
                  className="text-[13px] font-bold leading-none text-[#f8d98e] tracking-wide text-right"
                  style={METADATA_TEXT_STYLE}
                >
                  {statsText}
                </div>
              )}
              {metadataText && (
                <div
                  className="text-[12px] leading-snug text-[#d1e2f6]"
                  style={METADATA_TEXT_STYLE}
                >
                  {metadataText}
                </div>
              )}
              {oracleText && (
                <SymbolText
                  text={hideOracleText ? "" : oracleText}
                  className="text-[14px] leading-[1.38] text-[#ecf4ff] block"
                  style={ORACLE_TEXT_STYLE}
                />
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
