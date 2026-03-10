import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import { useDragActions } from "@/context/DragContext";
import { useHoverActions } from "@/context/HoverContext";
import TableCore from "@/components/board/TableCore";
import RightRail from "@/components/right-rail/RightRail";
import HandZone from "@/components/board/HandZone";
import DragOverlay from "@/components/overlays/DragOverlay";
import CastParticles from "@/components/overlays/CastParticles";
import ArrowOverlay from "@/components/overlays/ArrowOverlay";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildStackTargetPresentation, normalizeZoneViews } from "@/lib/stack-targets";

const HAND_PEEK_HEIGHT = 46;
const HAND_REVEAL_HEIGHT = 164;
const HAND_COLLAPSED_SHELL_HEIGHT = HAND_PEEK_HEIGHT;
const HAND_LANE_HOVER_FUZZ = 6;

function objectExistsInState(state, objectId) {
  if (!state || objectId == null) return false;
  const needle = String(objectId);
  const players = state?.players || [];

  for (const player of players) {
    const zones = [
      player?.battlefield || [],
      player?.hand_cards || [],
      player?.graveyard_cards || [],
      player?.exile_cards || [],
      player?.command_cards || [],
    ];
    for (const cards of zones) {
      for (const card of cards) {
        if (String(card?.id) === needle) return true;
        if (Array.isArray(card?.member_ids) && card.member_ids.some((id) => String(id) === needle)) {
          return true;
        }
      }
    }
  }

  for (const entry of state?.stack_objects || []) {
    if (String(entry?.id) === needle) return true;
    if (String(entry?.inspect_object_id) === needle) return true;
  }

  if ((state?.viewed_cards?.card_ids || []).some((id) => String(id) === needle)) {
    return true;
  }

  return false;
}

function shouldExpandInlineInspector(player, objectId) {
  if (!player || objectId == null) return false;
  const needle = String(objectId);

  if ((player.hand_cards || []).some((card) => String(card?.id) === needle)) {
    return true;
  }

  for (const zone of [player.graveyard_cards || [], player.exile_cards || [], player.command_cards || []]) {
    for (const card of zone) {
      if (String(card?.id) === needle && card?.show_in_pseudo_hand) {
        return true;
      }
    }
  }

  return false;
}

function sameObjectId(left, right) {
  return left != null && right != null && String(left) === String(right);
}

function rectContainsPoint(rect, x, y, fuzz = 0) {
  if (!rect) return false;
  return (
    x >= (rect.left - fuzz)
    && x <= (rect.right + fuzz)
    && y >= (rect.top - fuzz)
    && y <= (rect.bottom + fuzz)
  );
}

function stackSelectionKeys(entry) {
  const keys = [entry?.id, entry?.inspect_object_id]
    .filter((value) => value != null)
    .map((value) => String(value));
  return Array.from(new Set(keys));
}

export default function Workspace({
  zoneViews,
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
  notices = [],
  onDismissNotice,
}) {
  const [selectedObjectId, setSelectedObjectId] = useState(null);
  const [pinnedInspectorObjectId, setPinnedInspectorObjectId] = useState(null);
  const [expandedInspectorObjectId, setExpandedInspectorObjectId] = useState(null);
  const [suppressFallbackInspector, setSuppressFallbackInspector] = useState(false);
  const [handLaneHovered, setHandLaneHovered] = useState(false);
  const [opponentsInspectorDockTop, setOpponentsInspectorDockTop] = useState(null);
  const [opponentsZoneHostRect, setOpponentsZoneHostRect] = useState(null);
  const [myZoneHostRect, setMyZoneHostRect] = useState(null);
  const workspaceRef = useRef(null);
  const previousStackIdsRef = useRef([]);
  const handRevealShellRef = useRef(null);
  const handRevealMotionRef = useRef(null);
  const handHoverCloseTimerRef = useRef(null);
  const {
    game,
    state,
    dispatch,
    refresh,
    setStatus,
    multiplayer,
  } = useGame();
  const { updateStackArrows, clearStackArrows } = useCombatArrows();
  const { endDrag } = useDragActions();
  const { clearHover, hoverCard } = useHoverActions();

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const selectedObjectIsValid = objectExistsInState(state, selectedObjectId);
  const forceInlineInspectorExpanded =
    sameObjectId(pinnedInspectorObjectId, selectedObjectId)
    || sameObjectId(expandedInspectorObjectId, selectedObjectId);
  const forceInlineInspectorFullArt = sameObjectId(expandedInspectorObjectId, selectedObjectId);
  const inlineInspectorExpanded =
    shouldExpandInlineInspector(me, selectedObjectId) || forceInlineInspectorExpanded;
  const handLaneOpen = handLaneHovered;
  const decision = state?.decision || null;
  const combatDeclarationActive = decision?.kind === "attackers" || decision?.kind === "blockers";
  const legalTargetObjectIds = useMemo(() => {
    const ids = new Set();
    if (!decision || decision.kind !== "targets") return ids;
    for (const req of decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "object" && target.object != null) {
          ids.add(Number(target.object));
        }
      }
    }
    return ids;
  }, [decision]);
  const legalTargetPlayerIds = useMemo(() => {
    const ids = new Set();
    if (!decision || decision.kind !== "targets") return ids;
    for (const req of decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "player" && target.player != null) {
          ids.add(Number(target.player));
        }
      }
    }
    return ids;
  }, [decision]);
  const activeViewedCards = state?.viewed_cards || null;
  const activeViewedCardIds = useMemo(
    () => new Set((activeViewedCards?.card_ids || []).map((id) => String(id))),
    [activeViewedCards?.card_ids]
  );
  const stackTargetPresentation = useMemo(
    () => buildStackTargetPresentation(state, zoneViews, selectedObjectId),
    [selectedObjectId, state, zoneViews]
  );
  const temporaryZoneViews = useMemo(
    () => (combatDeclarationActive ? [] : stackTargetPresentation.temporaryZoneViews),
    [combatDeclarationActive, stackTargetPresentation.temporaryZoneViews]
  );
  const effectiveZoneViews = useMemo(() => {
    const merged = new Set(normalizeZoneViews(zoneViews));
    for (const zone of temporaryZoneViews) {
      merged.add(zone);
    }
    return normalizeZoneViews(Array.from(merged));
  }, [temporaryZoneViews, zoneViews]);
  const stackArrowSignature = useMemo(
    () => stackTargetPresentation.arrows.map((arrow) => arrow.key).join("|"),
    [stackTargetPresentation.arrows]
  );

  useEffect(() => {
    if (selectedObjectId == null) return;
    if (selectedObjectIsValid) return;
    const invalidSelection = String(selectedObjectId);
    queueMicrotask(() => {
      setSelectedObjectId((currentSelection) => (
        String(currentSelection) === invalidSelection ? null : currentSelection
      ));
      setPinnedInspectorObjectId((currentPinned) => (
        sameObjectId(currentPinned, invalidSelection) ? null : currentPinned
      ));
      setExpandedInspectorObjectId((currentExpanded) => (
        sameObjectId(currentExpanded, invalidSelection) ? null : currentExpanded
      ));
    });
  }, [selectedObjectId, selectedObjectIsValid]);

  useEffect(() => {
    const viewedIds = activeViewedCards?.card_ids || [];
    if (viewedIds.length === 0) return;
    const selectedKey = selectedObjectId == null ? null : String(selectedObjectId);
    if (selectedKey != null && activeViewedCardIds.has(selectedKey)) return;
    const nextViewedId = viewedIds[0];
    if (nextViewedId == null) return;
    queueMicrotask(() => {
      setSelectedObjectId(nextViewedId);
      setPinnedInspectorObjectId(String(nextViewedId));
      setExpandedInspectorObjectId(null);
      setSuppressFallbackInspector(false);
    });
  }, [activeViewedCardIds, activeViewedCards?.card_ids, selectedObjectId]);

  useEffect(() => {
    const stackObjects = state?.stack_objects || [];
    const currentStackIds = stackObjects.flatMap((entry) => stackSelectionKeys(entry));
    const previousStackIds = previousStackIdsRef.current;
    const removedIds = previousStackIds.filter((id) => !currentStackIds.includes(id));

    if (
      removedIds.length > 0
      && selectedObjectId != null
      && !combatDeclarationActive
      && previousStackIds.includes(String(selectedObjectId))
    ) {
      const nextTopId = stackObjects[0]?.id ?? null;
      const selectedSnapshot = String(selectedObjectId);
      queueMicrotask(() => {
        setSelectedObjectId((currentSelection) => {
          if (String(currentSelection) !== selectedSnapshot) return currentSelection;
          return nextTopId;
        });
        setPinnedInspectorObjectId(null);
        setExpandedInspectorObjectId((currentExpanded) => (
          currentExpanded == null ? currentExpanded : null
        ));
      });
    }

    previousStackIdsRef.current = currentStackIds;
  }, [state?.stack_objects, selectedObjectId, combatDeclarationActive]);

  useEffect(() => {
    if (!combatDeclarationActive) return;
    queueMicrotask(() => {
      setSelectedObjectId(null);
      setPinnedInspectorObjectId(null);
      setExpandedInspectorObjectId(null);
    });
  }, [combatDeclarationActive]);

  useEffect(() => {
    if (combatDeclarationActive || stackTargetPresentation.arrows.length === 0) {
      clearStackArrows();
      return undefined;
    }

    let firstFrameId = 0;
    let secondFrameId = 0;
    firstFrameId = window.requestAnimationFrame(() => {
      secondFrameId = window.requestAnimationFrame(() => {
        updateStackArrows(stackTargetPresentation.arrows);
      });
    });

    return () => {
      if (firstFrameId) window.cancelAnimationFrame(firstFrameId);
      if (secondFrameId) window.cancelAnimationFrame(secondFrameId);
    };
  }, [
    clearStackArrows,
    combatDeclarationActive,
    effectiveZoneViews,
    stackArrowSignature,
    stackTargetPresentation.arrows,
    updateStackArrows,
  ]);

  useEffect(() => () => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
  }, []);

  useLayoutEffect(() => {
    const shellEl = handRevealShellRef.current;
    if (!shellEl) return undefined;

    cancelMotion(handRevealMotionRef.current);
    handRevealMotionRef.current = animate(shellEl, {
      height: handLaneOpen ? HAND_REVEAL_HEIGHT : HAND_COLLAPSED_SHELL_HEIGHT,
      duration: 420,
      ease: uiSpring({ duration: 420, bounce: 0.16 }),
    });

    return () => {
      cancelMotion(handRevealMotionRef.current);
      handRevealMotionRef.current = null;
    };
  }, [handLaneOpen]);

  useLayoutEffect(() => {
    const root = workspaceRef.current;
    if (!root || deckLoadingMode) return undefined;

    let rafId = null;
    let resizeObserver = null;

    const measureDockTop = () => {
      const opponentsEl = root.querySelector("[data-opponents-zones]");
      const myZoneEl = root.querySelector("[data-my-zone]");
      if (!opponentsEl) {
        setOpponentsInspectorDockTop(null);
        setOpponentsZoneHostRect(null);
        return;
      }

      const opponentsRect = opponentsEl.getBoundingClientRect();
      const nextTop = Math.max(0, Math.round(opponentsRect.bottom - HAND_PEEK_HEIGHT));
      setOpponentsInspectorDockTop((currentTop) => (
        currentTop == null || Math.abs(currentTop - nextTop) >= 1 ? nextTop : currentTop
      ));
      const nextOpponentsRect = {
        top: Math.round(opponentsRect.top),
        height: Math.round(opponentsRect.height),
      };
      setOpponentsZoneHostRect((currentRect) => (
        currentRect == null
        || currentRect.top !== nextOpponentsRect.top
        || currentRect.height !== nextOpponentsRect.height
          ? nextOpponentsRect
          : currentRect
      ));
      if (!myZoneEl) {
        setMyZoneHostRect(null);
        return;
      }
      const myZoneRect = myZoneEl.getBoundingClientRect();
      const nextMyZoneRect = {
        top: Math.round(myZoneRect.top),
        height: Math.round(myZoneRect.height),
      };
      setMyZoneHostRect((currentRect) => (
        currentRect == null
        || currentRect.top !== nextMyZoneRect.top
        || currentRect.height !== nextMyZoneRect.height
          ? nextMyZoneRect
          : currentRect
      ));
    };

    const scheduleMeasure = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        measureDockTop();
      });
    };

    scheduleMeasure();

    resizeObserver = new ResizeObserver(scheduleMeasure);
    resizeObserver.observe(root);
    const tableEl = root.querySelector("[data-drop-zone]");
    const opponentsEl = root.querySelector("[data-opponents-zones]");
    if (tableEl) resizeObserver.observe(tableEl);
    if (opponentsEl) resizeObserver.observe(opponentsEl);
    window.addEventListener("resize", scheduleMeasure);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", scheduleMeasure);
    };
  }, [deckLoadingMode, effectiveZoneViews, players.length]);

  const handleInspectObject = useCallback(
    async (objectId, options = null) => {
      if (combatDeclarationActive) return;
      if (
        decision?.kind === "targets"
        && decision.player === state?.perspective
        && objectId != null
        && legalTargetObjectIds.has(Number(objectId))
      ) {
        window.dispatchEvent(
          new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "object", object: Number(objectId) } },
          })
        );
        return;
      }
      const stackEntry = options?.source === "stack" ? options?.stackEntry : null;
      if (
        stackEntry
        && !multiplayer.matchStarted
        && game
        && Number.isFinite(Number(stackEntry.controller))
        && Number(stackEntry.controller) !== Number(state?.perspective)
      ) {
        try {
          await game.setPerspective(Number(stackEntry.controller));
          await refresh(`Viewing as player ${Number(stackEntry.controller)}`);
        } catch (err) {
          setStatus(`Change player failed: ${err}`, true);
          return;
        }
      }
      setSelectedObjectId(objectId);
      setPinnedInspectorObjectId(objectId == null ? null : String(objectId));
      setExpandedInspectorObjectId(null);
      setSuppressFallbackInspector(false);
      if (objectId != null) hoverCard(objectId);
    },
    [
      combatDeclarationActive,
      decision,
      game,
      hoverCard,
      legalTargetObjectIds,
      multiplayer.matchStarted,
      refresh,
      setStatus,
      state?.perspective,
    ]
  );

  const handleExpandInspector = useCallback(
    (objectId) => {
      if (combatDeclarationActive || objectId == null) return;
      setSelectedObjectId(objectId);
      setPinnedInspectorObjectId(String(objectId));
      setExpandedInspectorObjectId((currentExpanded) => (
        sameObjectId(currentExpanded, objectId) ? null : String(objectId)
      ));
      setSuppressFallbackInspector(false);
      hoverCard(objectId);
    },
    [combatDeclarationActive, hoverCard]
  );

  const handleNoticeClick = useCallback(
    async (notice) => {
      if (!notice?.copyText) return;
      const copied = await copyTextToClipboard(notice.copyText);
      if (copied) {
        setStatus(notice.copyStatusMessage || "Copied to clipboard");
      } else {
        setStatus("Could not copy to clipboard", true);
      }
    },
    [setStatus]
  );

  const handleHandLaneEnter = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
    setHandLaneHovered((currentHovered) => (currentHovered ? currentHovered : true));
  }, []);

  const handleHandLaneLeave = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
    }
    handHoverCloseTimerRef.current = setTimeout(() => {
      setHandLaneHovered(false);
      handHoverCloseTimerRef.current = null;
    }, 90);
  }, []);

  const collapseHandLane = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
    setHandLaneHovered((currentHovered) => (currentHovered ? false : currentHovered));
  }, []);

  useEffect(() => {
    const onPointerMove = (event) => {
      const shellEl = handRevealShellRef.current;
      if (!shellEl) return;

      const target = event.target;
      const overHandCard = target instanceof Element
        && target.closest(".hand-reveal-shell .game-card.hand-card");
      const insideExpandedShell = handLaneOpen && rectContainsPoint(
        shellEl.getBoundingClientRect(),
        event.clientX,
        event.clientY,
        HAND_LANE_HOVER_FUZZ
      );

      if (overHandCard || insideExpandedShell) {
        handleHandLaneEnter();
        return;
      }

      if (handLaneOpen) {
        handleHandLaneLeave();
      }
    };

    document.addEventListener("pointermove", onPointerMove, { passive: true });
    return () => {
      document.removeEventListener("pointermove", onPointerMove);
    };
  }, [handLaneOpen, handleHandLaneEnter, handleHandLaneLeave]);

  // Handle drag drop — if user drops on the battlefield area, dispatch the action
  useEffect(() => {
    const onPointerUp = (e) => {
      const ds = endDrag();
      if (!ds || !ds.actions || ds.actions.length === 0) return;

      // Check if dropped over the table area (anywhere above the hand)
      const el = document.elementFromPoint(e.clientX, e.clientY);
      const isOverTable = el && (
        el.closest("[data-drop-zone]") ||
        el.closest(".table-gradient") ||
        el.closest(".board-zone-bg")
      );

      if (!isOverTable) return;

      collapseHandLane();

      if (ds.actions.length === 1) {
        const onlyAction = ds.actions[0];
        window.__castParticles?.(e.clientX, e.clientY, ds.glowKind || "spell");
        dispatch(
          { type: "priority_action", action_index: onlyAction.index },
          onlyAction.label
        );
        if (!combatDeclarationActive && ds.objectId != null) {
          setSelectedObjectId(ds.objectId);
          setPinnedInspectorObjectId(null);
          setExpandedInspectorObjectId(null);
          setSuppressFallbackInspector(false);
        }
        return;
      }

      // Multiple possible actions: pin inspector to this card while actions
      // remain available in the action strip.
      if (!combatDeclarationActive) {
        setSelectedObjectId(ds.objectId != null ? ds.objectId : null);
        setPinnedInspectorObjectId(null);
        setExpandedInspectorObjectId(null);
        setSuppressFallbackInspector(false);
      }
      clearHover();
    };

    const onPointerCancel = () => {
      endDrag();
    };

    const onWindowBlur = () => {
      endDrag();
    };

    document.addEventListener("pointerup", onPointerUp);
    document.addEventListener("pointercancel", onPointerCancel);
    window.addEventListener("blur", onWindowBlur);
    return () => {
      document.removeEventListener("pointerup", onPointerUp);
      document.removeEventListener("pointercancel", onPointerCancel);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, [clearHover, collapseHandLane, combatDeclarationActive, dispatch, endDrag]);

  useEffect(() => {
    const onDeadZonePointerDown = (event) => {
      if (event.button !== 0) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (decision && decision.player === state?.perspective && decision.kind !== "priority") return;
      if (target.closest("[data-object-id]")) return;
      if (target.closest(".zone-viewer")) return;
      if (target.closest(".priority-inline-panel")) return;
      if (target.closest("button, input, label, a, [role='button']")) return;

      const inDeadZone = (
        target.closest("[data-drop-zone]")
        || target.closest(".table-gradient")
        || target.closest(".board-zone-bg")
      );
      if (!inDeadZone) return;

      setSelectedObjectId(null);
      setPinnedInspectorObjectId(null);
      setExpandedInspectorObjectId(null);
      setSuppressFallbackInspector(true);
      clearHover();
    };

    document.addEventListener("pointerdown", onDeadZonePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onDeadZonePointerDown, true);
    };
  }, [clearHover, decision, state?.perspective]);

  return (
    <section
      ref={workspaceRef}
      className="relative min-h-0 h-full w-full min-w-0 overflow-visible"
      data-workspace-shell
    >
      <DragOverlay />
      <CastParticles />
      <ArrowOverlay />
      {notices.length > 0 && (
        <div className="absolute top-2 right-2 z-[120] flex max-w-[min(460px,52vw)] flex-col gap-2">
          {notices.map((notice) => {
            const toneClasses = notice.tone === "success"
              ? "border-[#285f3c] bg-[rgba(8,24,14,0.96)] text-[#8fe2ad] hover:border-[#3e8b5b]"
              : notice.tone === "error"
                ? "border-[#9f2b2b] bg-[rgba(24,8,8,0.96)] text-[#ff7f7f] hover:border-[#c04040]"
                : "border-[#35506c] bg-[rgba(8,13,20,0.96)] text-[#cfe3fb] hover:border-[#4d7093]";
            const clickable = Boolean(notice.copyText);
            return (
              <div
                key={notice.id}
                className={`relative overflow-hidden rounded border shadow-[0_10px_26px_rgba(0,0,0,0.45)] ${toneClasses}`}
              >
                {clickable ? (
                  <button
                    type="button"
                    className="w-full px-3 py-2 pr-9 text-left transition-colors"
                    onClick={() => handleNoticeClick(notice)}
                    title="Click to copy"
                  >
                    <div className="text-[13px] font-bold uppercase tracking-wide">
                      {notice.title}
                    </div>
                    {notice.body ? (
                      <div className="mt-1 text-[13px] font-semibold leading-tight">
                        {notice.body}
                      </div>
                    ) : null}
                  </button>
                ) : (
                  <div className="px-3 py-2 pr-9 text-left">
                    <div className="text-[13px] font-bold uppercase tracking-wide">
                      {notice.title}
                    </div>
                    {notice.body ? (
                      <div className="mt-1 text-[13px] font-semibold leading-tight">
                        {notice.body}
                      </div>
                    ) : null}
                  </div>
                )}
                <button
                  type="button"
                  className="absolute right-1.5 top-1.5 rounded px-1 text-[12px] font-bold text-current opacity-80 transition-opacity hover:opacity-100"
                  onClick={() => onDismissNotice?.(notice.id)}
                  aria-label={`Dismiss ${notice.title}`}
                >
                  x
                </button>
              </div>
            );
          })}
        </div>
      )}
      <div className="min-h-0 h-full overflow-visible">
        <TableCore
          selectedObjectId={selectedObjectId}
          onInspect={handleInspectObject}
          onExpandInspector={handleExpandInspector}
          zoneViews={effectiveZoneViews}
          deckLoadingMode={deckLoadingMode}
          onLoadDecks={onLoadDecks}
          onCancelDeckLoading={onCancelDeckLoading}
          legalTargetPlayerIds={legalTargetPlayerIds}
          legalTargetObjectIds={legalTargetObjectIds}
        />
      </div>
      {!deckLoadingMode && opponentsInspectorDockTop != null && (
        <div
          className="pointer-events-none fixed inset-x-0 z-30 flex items-end justify-end overflow-visible px-2"
          style={{ top: `${opponentsInspectorDockTop}px`, height: `${HAND_PEEK_HEIGHT}px` }}
          data-inspector-dock="top"
          data-opponents-inspector-dock
        >
          <div className="pointer-events-none relative flex shrink-0 items-end gap-1.5 self-end overflow-visible">
            <RightRail
              pinnedObjectId={selectedObjectId}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              inlineDockPlacement="top"
              allowTopInlinePlacement
              inlineExpanded={inlineInspectorExpanded}
              forceInlineExpanded={forceInlineInspectorExpanded}
              fullArtInlineExpanded={forceInlineInspectorFullArt}
            />
          </div>
        </div>
      )}
      {!deckLoadingMode && opponentsZoneHostRect != null && (
        <div
          className="pointer-events-none fixed inset-x-0 z-30 flex items-start justify-start overflow-visible px-2"
          style={{
            top: `${opponentsZoneHostRect.top}px`,
            height: `${opponentsZoneHostRect.height}px`,
          }}
        >
          <div className="pointer-events-none relative flex h-full shrink-0 items-start gap-1.5 self-start overflow-visible">
            <RightRail
              pinnedObjectId={selectedObjectId}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              inlineDockPlacement="top"
              inlineHostSide="left"
              inlineExpandedSide="left"
              allowTopInlinePlacement
              inlineExpanded={inlineInspectorExpanded}
              forceInlineExpanded={forceInlineInspectorExpanded}
              fullArtInlineExpanded={forceInlineInspectorFullArt}
            />
          </div>
        </div>
      )}
      <div
        className="pointer-events-none fixed inset-x-0 bottom-2 z-30 flex items-end gap-1.5 overflow-visible px-2"
        style={{ height: `${HAND_PEEK_HEIGHT}px` }}
        data-bottom-dock
        data-inspector-dock="bottom"
      >
        <div
          className="pointer-events-none relative min-w-0 flex-1 h-full overflow-visible"
          data-hand-dock-lane
        >
          <div
            ref={handRevealShellRef}
            className="hand-reveal-shell absolute left-0 bottom-0"
            data-open={handLaneOpen ? "true" : "false"}
            aria-expanded={handLaneOpen}
            style={{
              height: `${handLaneOpen ? HAND_REVEAL_HEIGHT : HAND_COLLAPSED_SHELL_HEIGHT}px`,
              "--hand-shell-offset-x": "3vw",
            }}
            onMouseEnter={handleHandLaneEnter}
            onMouseLeave={handleHandLaneLeave}
            onFocusCapture={handleHandLaneEnter}
            onBlurCapture={(event) => {
              if (event.currentTarget.contains(event.relatedTarget)) return;
              handleHandLaneLeave();
            }}
          >
            <div
              className="hand-reveal-body"
              style={{ height: "100%" }}
            >
              <HandZone
                player={me}
                selectedObjectId={selectedObjectId}
                onInspect={handleInspectObject}
                isExpanded={handLaneOpen}
              />
            </div>
          </div>
        </div>
        <div className="pointer-events-none relative flex shrink-0 items-end gap-1.5 self-end overflow-visible">
          <RightRail
            pinnedObjectId={selectedObjectId}
            onInspectObject={handleInspectObject}
            suppressFallback={suppressFallbackInspector}
            inline
            allowTopInlinePlacement={opponentsInspectorDockTop != null}
            inlineExpanded={inlineInspectorExpanded}
            forceInlineExpanded={forceInlineInspectorExpanded}
            fullArtInlineExpanded={forceInlineInspectorFullArt}
          />
        </div>
      </div>
      {!deckLoadingMode && myZoneHostRect != null && (
        <div
          className="pointer-events-none fixed inset-x-0 z-30 flex items-start justify-start overflow-visible px-2"
          style={{
            top: `${myZoneHostRect.top}px`,
            height: `${myZoneHostRect.height}px`,
          }}
        >
          <div className="pointer-events-none relative flex h-full shrink-0 items-start gap-1.5 self-start overflow-visible">
            <RightRail
              pinnedObjectId={selectedObjectId}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              inlineHostSide="left"
              inlineExpandedSide="left"
              allowTopInlinePlacement={opponentsInspectorDockTop != null}
              inlineExpanded={inlineInspectorExpanded}
              forceInlineExpanded={forceInlineInspectorExpanded}
              fullArtInlineExpanded={forceInlineInspectorFullArt}
            />
          </div>
        </div>
      )}
    </section>
  );
}
