import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { useCastObjectHovered, useCastTargeting } from "@/context/DragContext";
import { useHoverActions } from "@/context/HoverContext";
import { resolveStackInspectObjectId } from "@/lib/inspector-selection";
import { samePlayerId } from "@/lib/player-display";
import { stackEntryAimedObjectIds, stackEntryIsLegalTarget, stackEntryTargetObjectIds } from "@/lib/stack-targets";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import PlayerStackAlert from "@/components/board/PlayerStackAlert";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import { getPlayerAccent, playerAccentVars } from "@/lib/player-colors";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";
import { ArrowDown, ArrowUp } from "lucide-react";

// The short word under a tile's name: what kind of thing is waiting.
function stackEntryKindLabel(entry) {
  const abilityKind = String(entry?.ability_kind || "").trim();
  const normalized = abilityKind.toLowerCase();
  if (!abilityKind) return "Spell";
  if (normalized === "triggered") return "Trigger";
  if (normalized === "activated") return "Activation";
  return `${abilityKind} ability`;
}

// One line of what the entry does. A pending trigger carries its own text
// (the ordering option's detail); a live ability shows its printed line; a
// spell shows nothing here, its full text belongs to the card preview.
function stackEntryDetailText(entry) {
  const subtitle = String(entry?.__subtitle || "").trim();
  if (subtitle) return subtitle;
  if (!entry?.ability_kind) return "";
  return String(entry?.source_ability_text || entry?.ability_text || "").trim();
}

export default function StackCard({
  entry,
  isNew = false,
  isActive = false,
  isLeaving = false,
  showStackAlert = false,
  className = "",
  onClick,
  reorderControls = null,
  entryMotion = "default",
  variant = "default",
  positionLabel = null,
  density = "default",
}) {
  const ui = useUiText();
  const { state } = useGame();
  const {
    hoverCard,
    clearHover,
    setHoverLinkedObjects,
    clearAnchoredCardPreview,
    showAnchoredCardPreview,
    scheduleAnchoredCardPreviewClear,
  } = useHoverActions();
  // A spell being aimed from the hand -- Counterspell dragged out of it -- can
  // land on this tile the way it lands on a creature: the tile lights up as a
  // legal target while the gesture is live and the release names it. The
  // decision may be the engine's own (one way to cast the card puts it on the
  // stack at once) or the preview a provisional gesture carries.
  const castIntent = useCastTargeting();
  const liveTargetDecision = state?.decision?.kind === "targets" ? state.decision : null;
  const targetDecision = liveTargetDecision || castIntent?.targetDecision || null;
  const targetingMode = Boolean(castIntent) || Boolean(liveTargetDecision);
  const targetObjectIdKey = stackEntryTargetObjectIds(entry).join(",");
  const targetObjectIds = useMemo(
    () => (targetObjectIdKey ? targetObjectIdKey.split(",").map(Number) : []),
    [targetObjectIdKey],
  );
  const isLegalTarget = targetingMode && stackEntryIsLegalTarget(targetDecision, entry);
  const castObjectHovered = useCastObjectHovered(targetObjectIds);
  const isCastTargetHovered = isLegalTarget && castObjectHovered;
  // A tile that owns its click (the desktop rails) also owns the pick. The
  // mobile rail wraps the tile in its own tap handling and passes no onClick.
  const canPickTarget = isLegalTarget
    && typeof onClick === "function"
    && samePlayerId(targetDecision?.player, state?.perspective);
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const name = entry.name || `Object#${entry.id}`;
  const artUrl = useScryfallImageUrl(name, "art_crop");
  const scryfallUrl = useScryfallImageUrl(name);
  const isCastEntry = !entry.ability_kind;
  const isPendingTrigger = Boolean(entry?.__trigger_ordering);
  const kindLabel = stackEntryKindLabel(entry);
  const detailText = stackEntryDetailText(entry);
  const pt = entry.power_toughness
    || (entry.power != null && entry.toughness != null
      ? `${entry.power}/${entry.toughness}`
      : null);
  const stackAccent = getPlayerAccent(state?.players || [], entry?.controller, state?.perspective);
  const stackAccentStyle = stackAccent
    ? {
      ...playerAccentVars(stackAccent),
      "--glow-rgb": stackAccent.rgb,
    }
    : undefined;
  const hasReorderControls = !!reorderControls;
  // A stack tile's click belongs to whatever is live -- a target pick, a
  // resolve, an inspector request. Hover is the one read path nothing else
  // claims, so it is what keeps the spell or ability readable mid-decision.
  //
  // It has to be the *anchored* preview, not a plain hoverCard: the passive
  // hover surface rejects anything a visible stack entry refers to
  // (canHoverInspectorObject), and every Workspace inspector dock is mounted
  // with allowHoverFallback={false}. The anchored path is the one that exists
  // precisely so a spell or ability on the stack can still be previewed.
  const inspectObjectId = resolveStackInspectObjectId(state, entry);
  // The stack entry itself: what the frame keys its ability highlight off.
  // Falls back to the card when an entry has no id of its own. A pending
  // trigger's id is a placeholder that no state carries, so its preview is
  // the source object the engine named for it.
  const previewObjectId = isPendingTrigger ? inspectObjectId : (entry?.id ?? inspectObjectId);
  const canPreview = !isLeaving && inspectObjectId != null;
  const handleHoverEnter = useCallback((event) => {
    if (!canPreview) return;
    hoverCard(inspectObjectId);
    // What the entry is aimed at lights up the way a linked permanent does,
    // and a pile holding one of its targets opens to show it. clearHover on
    // leave drops the links again.
    setHoverLinkedObjects(stackEntryAimedObjectIds(entry));
    // Anchor to the stack, not to this tile: the frame then holds one position
    // for every entry, and sits flush against the stack so the pointer can
    // reach it. Falling back to the tile keeps standalone usages working.
    const anchor = event.currentTarget.closest("[data-stack-preview-anchor]") || event.currentTarget;
    // Previewed by *this entry's* id, not the card's. That is what the frame
    // matches to highlight the one ability that is on the stack, and it is
    // what a click would have set -- so a click has nothing left to change.
    showAnchoredCardPreview(previewObjectId, anchor, { placement: "stack" });
  }, [canPreview, entry, hoverCard, inspectObjectId, previewObjectId, setHoverLinkedObjects, showAnchoredCardPreview]);
  const handleHoverLeave = useCallback(() => {
    clearHover();
    // Not a dismissal: the pointer may be on its way into the frame, which
    // cancels this. Moving anywhere else lets it close.
    scheduleAnchoredCardPreviewClear();
  }, [clearHover, scheduleAnchoredCardPreviewClear]);
  // The click does not touch the preview. Hovering already shows this entry,
  // so there is nothing for a click to reveal -- and clearing it here made the
  // frame blink out and reappear from the pinned path instead.
  const handleClick = useCallback((event) => {
    // The pointerdown already made the pick; the click the browser follows it
    // with must not toggle that pick back off.
    if (canPickTarget && !shouldHandleClick(event)) return;
    onClick?.(entry.inspect_object_id ?? entry.id, {
      source: "stack",
      stackEntry: entry,
    });
  }, [canPickTarget, entry, onClick, shouldHandleClick]);
  // Picked on pointerdown like a battlefield card, so the choice lands the
  // moment the button goes down. A provisional gesture (several ways to cast
  // the held card) has opened no decision yet: its release names the target,
  // and registering the press only keeps the trailing click from inspecting.
  const handlePointerDown = useCallback((event) => {
    if (!canPickTarget || !registerPointerDown(event)) return;
    if (!liveTargetDecision) return;
    event.preventDefault();
    event.stopPropagation();
    clearHover();
    clearAnchoredCardPreview();
    window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
      detail: { target: { kind: "object", object: targetObjectIds[0] } },
    }));
  }, [canPickTarget, clearAnchoredCardPreview, clearHover, liveTargetDecision, registerPointerDown, targetObjectIds]);
  const rootRef = useRef(null);
  const motionRef = useRef(null);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node) return undefined;

    cancelMotion(motionRef.current);
    motionRef.current = null;
    node.style.opacity = "";
    node.style.transform = "";

    if (isLeaving) {
      motionRef.current = createTimeline({ autoplay: true }).add(node, {
        opacity: [1, 0],
        y: [0, -14],
        scale: [1, 0.97],
        duration: 360,
        ease: "out(2)",
      });
    } else if (isNew) {
      const enterX = entryMotion === "mobile-stack" ? [-16, 0] : [0, 0];
      motionRef.current = createTimeline({ autoplay: true }).add(node, {
        opacity: [0, 1],
        x: enterX,
        y: [18, 0],
        scale: [0.92, 1],
        duration: 380,
        ease: uiSpring({ duration: 380, bounce: 0.18 }),
        onComplete: () => {
          node.style.opacity = "";
          node.style.transform = "";
        },
      });
    }
  }, [entryMotion, isLeaving, isNew]);

  useEffect(() => () => {
    cancelMotion(motionRef.current);
    motionRef.current = null;
  }, []);

  const sharedProps = {
    ref: rootRef,
    "data-object-id": entry.id,
    "data-target-object-ids": targetObjectIdKey,
    "data-card-image-url": artUrl || "",
    "data-card-name": name,
    "data-pending-trigger": isPendingTrigger ? "true" : undefined,
    onClick: handleClick,
    onPointerDown: handlePointerDown,
    onMouseEnter: handleHoverEnter,
    onMouseLeave: handleHoverLeave,
    style: stackAccentStyle,
  };
  const stateClasses = cn(
    onClick ? "cursor-pointer" : "cursor-default",
    isActive && "stack-card-active",
    isPendingTrigger && "stack-card-pending",
    targetingMode && "card-targeting-mode",
    isLegalTarget && "target-legal",
    isCastTargetHovered && "hovered",
    isLeaving && "pointer-events-none",
  );

  // Compact tiles (mobile stack rail): full-bleed art with the name over a
  // bottom scrim -- the default layout's title can't fit a ~56px tile.
  if (variant === "compact") {
    return (
      <div
        {...sharedProps}
        className={cn("game-card stack-card stack-card--compact overflow-hidden", stateClasses, className)}
      >
        {artUrl && (
          <img
            className="stack-card-compact-art"
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        )}
        <div className="stack-card-compact-scrim" aria-hidden="true" />
        <span className="stack-card-accent" aria-hidden="true" />
        <PlayerStackAlert
          visible={showStackAlert}
          className="absolute right-1 top-1 z-[3]"
        />
        <div className="stack-card-compact-name">{name}</div>
      </div>
    );
  }

  return (
    <div
      {...sharedProps}
      className={cn(
        "game-card stack-card stack-card--flat w-full min-w-0 overflow-hidden",
        hasReorderControls && "stack-card-reorderable",
        stateClasses,
        className
      )}
      data-density={density}
    >
      <span className="stack-card-accent" aria-hidden="true" />

      <div className="stack-card-body">
        <div className="stack-card-art">
          {artUrl && (
            <img
              src={artUrl}
              alt=""
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          )}
          {scryfallUrl && !hasReorderControls && (
            <a
              className="stack-card-img-link"
              href={scryfallUrl}
              target="_blank"
              rel="noopener noreferrer"
              onClick={(e) => e.stopPropagation()}
            >{ui("img")}</a>
          )}
        </div>
        <div className="stack-card-text">
          <div className="stack-card-head">
            {positionLabel && (
              <span
                className={cn(
                  "stack-card-position",
                  positionLabel === "Resolving" && "stack-card-position--resolving",
                  positionLabel === "Top" && "stack-card-position--top"
                )}
              >
                {ui(positionLabel)}
              </span>
            )}
            <span className="stack-card-title">{name}</span>
            <span className="stack-card-head-spacer" aria-hidden="true" />
            {isCastEntry && entry.mana_cost && (
              <span className="stack-card-cost">
                <ManaCostIcons cost={entry.mana_cost} />
              </span>
            )}
            {pt && <span className="stack-card-pt">{pt}</span>}
            <PlayerStackAlert visible={showStackAlert} className="stack-card-alert" />
          </div>
          <div className="stack-card-sub">
            <span className="stack-card-kind">
              {ui(isPendingTrigger ? "Pending" : kindLabel)}
            </span>
            {detailText && (
              <span className="stack-card-effect">
                <SymbolText text={detailText} style={{ whiteSpace: "inherit" }} />
              </span>
            )}
          </div>
        </div>
      </div>

      {hasReorderControls && (
        <div className="stack-card-reorder" role="group">
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-up"
            disabled={!reorderControls.canMoveLeft}
            onClick={(event) => {
              event.stopPropagation();
              reorderControls.onMoveLeft?.();
            }}
            aria-label={ui(reorderControls.leftLabel || `Move ${name} toward the top of the stack`)}
            title={ui(reorderControls.leftTitle || "Move toward the top of the stack")}
          >
            <ArrowUp className="size-3.5" />
          </button>
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-down"
            disabled={!reorderControls.canMoveRight}
            onClick={(event) => {
              event.stopPropagation();
              reorderControls.onMoveRight?.();
            }}
            aria-label={ui(reorderControls.rightLabel || `Move ${name} toward the bottom of the stack`)}
            title={ui(reorderControls.rightTitle || "Move toward the bottom of the stack")}
          >
            <ArrowDown className="size-3.5" />
          </button>
        </div>
      )}
    </div>
  );
}
