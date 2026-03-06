import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useDragActions } from "@/context/DragContext";
import { useHoverActions } from "@/context/HoverContext";
import TableCore from "@/components/board/TableCore";
import RightRail from "@/components/right-rail/RightRail";
import HandZone from "@/components/board/HandZone";
import DragOverlay from "@/components/overlays/DragOverlay";
import CastParticles from "@/components/overlays/CastParticles";
import ArrowOverlay from "@/components/overlays/ArrowOverlay";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";

const HAND_PEEK_HEIGHT = 46;
const HAND_REVEAL_HEIGHT = 164;
const HAND_GLOW_OVERFLOW = 14;
const HAND_COLLAPSED_SHELL_HEIGHT = HAND_PEEK_HEIGHT + HAND_GLOW_OVERFLOW;

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
  }

  return false;
}

function shouldExpandInlineInspector(player, objectId) {
  if (!player || objectId == null) return false;
  const needle = String(objectId);

  if ((player.hand_cards || []).some((card) => String(card?.id) === needle)) {
    return true;
  }

  for (const zone of [player.graveyard_cards || [], player.exile_cards || []]) {
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

export default function Workspace({
  zoneViews,
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
}) {
  const [selectedObjectId, setSelectedObjectId] = useState(null);
  const [expandedInspectorObjectId, setExpandedInspectorObjectId] = useState(null);
  const [handLaneHovered, setHandLaneHovered] = useState(false);
  const previousStackIdsRef = useRef([]);
  const handRevealShellRef = useRef(null);
  const handRevealMotionRef = useRef(null);
  const handHoverCloseTimerRef = useRef(null);
  const { state, dispatch, status } = useGame();
  const { endDrag } = useDragActions();
  const { clearHover, hoverCard } = useHoverActions();

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const addCardError = status?.isError && typeof status?.msg === "string" && status.msg.startsWith("Add card failed:")
    ? status.msg
    : null;
  const [dismissedAddCardError, setDismissedAddCardError] = useState(null);
  const selectedObjectIsValid = objectExistsInState(state, selectedObjectId);
  const forceInlineInspectorExpanded = sameObjectId(expandedInspectorObjectId, selectedObjectId);
  const inlineInspectorExpanded =
    shouldExpandInlineInspector(me, selectedObjectId) || forceInlineInspectorExpanded;
  const handLaneOpen = handLaneHovered;
  const decision = state?.decision || null;
  const combatDeclarationActive = decision?.kind === "attackers" || decision?.kind === "blockers";

  useEffect(() => {
    if (selectedObjectId == null) return;
    if (selectedObjectIsValid) return;
    const invalidSelection = String(selectedObjectId);
    queueMicrotask(() => {
      setSelectedObjectId((currentSelection) => (
        String(currentSelection) === invalidSelection ? null : currentSelection
      ));
      setExpandedInspectorObjectId((currentExpanded) => (
        sameObjectId(currentExpanded, invalidSelection) ? null : currentExpanded
      ));
    });
  }, [selectedObjectId, selectedObjectIsValid]);

  useEffect(() => {
    const stackObjects = state?.stack_objects || [];
    const currentStackIds = stackObjects.map((entry) => String(entry?.id));
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
      setExpandedInspectorObjectId(null);
    });
  }, [combatDeclarationActive]);

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

  const handleInspectObject = useCallback(
    (objectId) => {
      if (combatDeclarationActive) return;
      setSelectedObjectId(objectId);
      setExpandedInspectorObjectId((currentExpanded) => (
        sameObjectId(currentExpanded, objectId) ? currentExpanded : null
      ));
      if (objectId != null) hoverCard(objectId);
    },
    [combatDeclarationActive, hoverCard]
  );

  const handleExpandInspector = useCallback(
    (objectId) => {
      if (combatDeclarationActive || objectId == null) return;
      setSelectedObjectId(objectId);
      setExpandedInspectorObjectId((currentExpanded) => (
        sameObjectId(currentExpanded, objectId) ? null : String(objectId)
      ));
      hoverCard(objectId);
    },
    [combatDeclarationActive, hoverCard]
  );

  const handleHandLaneEnter = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
    setHandLaneHovered(true);
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
    setHandLaneHovered(false);
  }, []);

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
          setExpandedInspectorObjectId(null);
        }
        return;
      }

      // Multiple possible actions: pin inspector to this card while actions
      // remain available in the action strip.
      if (!combatDeclarationActive) {
        setSelectedObjectId(ds.objectId != null ? ds.objectId : null);
        setExpandedInspectorObjectId(null);
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
      setExpandedInspectorObjectId(null);
      clearHover();
    };

    document.addEventListener("pointerdown", onDeadZonePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onDeadZonePointerDown, true);
    };
  }, [clearHover]);

  return (
    <section
      className="relative min-h-0 h-full overflow-visible"
    >
      <DragOverlay />
      <CastParticles />
      <ArrowOverlay />
      {addCardError && dismissedAddCardError !== addCardError && (
        <button
          type="button"
          className="add-card-error-toast absolute top-2 right-2 z-[120] max-w-[min(420px,48vw)] rounded border border-[#9f2b2b] bg-[rgba(24,8,8,0.96)] px-3 py-2 text-left text-[13px] font-semibold leading-tight text-[#ff7f7f] shadow-[0_10px_26px_rgba(0,0,0,0.45)] hover:border-[#c04040] hover:text-[#ff9f9f] transition-colors"
          onClick={() => setDismissedAddCardError(addCardError)}
          title="Click to dismiss"
        >
          {addCardError}
        </button>
      )}
      <div className="min-h-0 h-full overflow-hidden">
        <TableCore
          selectedObjectId={selectedObjectId}
          onInspect={handleInspectObject}
          onExpandInspector={handleExpandInspector}
          zoneViews={zoneViews}
          deckLoadingMode={deckLoadingMode}
          onLoadDecks={onLoadDecks}
          onCancelDeckLoading={onCancelDeckLoading}
        />
      </div>
      <div
        className="pointer-events-none absolute inset-x-0 bottom-0 z-30 flex items-end gap-1.5 overflow-visible"
        style={{ height: `${HAND_PEEK_HEIGHT}px` }}
      >
        <div
          className="pointer-events-none relative min-w-0 flex-1 h-full overflow-visible"
        >
          <div
            ref={handRevealShellRef}
            className="hand-reveal-shell pointer-events-auto absolute left-0 bottom-0 max-w-full overflow-hidden"
            data-open={handLaneOpen ? "true" : "false"}
            aria-expanded={handLaneOpen}
            style={{ height: `${handLaneOpen ? HAND_REVEAL_HEIGHT : HAND_COLLAPSED_SHELL_HEIGHT}px` }}
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
              style={{ height: `${HAND_REVEAL_HEIGHT}px` }}
            >
              <HandZone
                player={me}
                selectedObjectId={selectedObjectId}
                onInspect={handleInspectObject}
              />
            </div>
          </div>
        </div>
        <RightRail
          pinnedObjectId={selectedObjectId}
          inline
          inlineExpanded={inlineInspectorExpanded}
          forceInlineExpanded={forceInlineInspectorExpanded}
        />
      </div>
    </section>
  );
}
