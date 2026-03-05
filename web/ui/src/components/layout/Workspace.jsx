import { useCallback, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useDragActions } from "@/context/DragContext";
import { useHoverActions } from "@/context/HoverContext";
import TableCore from "@/components/board/TableCore";
import RightRail from "@/components/right-rail/RightRail";
import HandZone from "@/components/board/HandZone";
import DragOverlay from "@/components/overlays/DragOverlay";
import CastParticles from "@/components/overlays/CastParticles";
import ArrowOverlay from "@/components/overlays/ArrowOverlay";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";

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

export default function Workspace({
  zoneViews,
  setZoneViews,
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
}) {
  const [selectedObjectId, setSelectedObjectId] = useState(null);
  const { state, dispatch, status } = useGame();
  const { endDrag } = useDragActions();
  const { clearHover } = useHoverActions();

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const handRowHeight = 140;
  const actionBridgeHeight = 62;
  const inspectorReservedWidth = "clamp(240px, 24vw, 360px)";
  const addCardError = status?.isError && typeof status?.msg === "string" && status.msg.startsWith("Add card failed:")
    ? status.msg
    : null;
  const [dismissedAddCardError, setDismissedAddCardError] = useState(false);
  const selectedObjectIsValid = objectExistsInState(state, selectedObjectId);
  const decision = state?.decision || null;
  const combatDeclarationActive = decision?.kind === "attackers" || decision?.kind === "blockers";
  const hasFocusedDecision = Boolean(
    decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
  const hasStackEntries = (state?.stack_objects?.length || 0) > 0 || (state?.stack_preview?.length || 0) > 0;
  const reserveInspectorSpace = selectedObjectIsValid || hasFocusedDecision || hasStackEntries;

  useEffect(() => {
    if (addCardError) setDismissedAddCardError(false);
  }, [status, addCardError]);

  useEffect(() => {
    if (selectedObjectId == null) return;
    if (selectedObjectIsValid) return;
    setSelectedObjectId(null);
  }, [selectedObjectId, selectedObjectIsValid]);

  useEffect(() => {
    if (!combatDeclarationActive) return;
    setSelectedObjectId(null);
  }, [combatDeclarationActive]);

  const handleInspectObject = useCallback(
    (objectId) => {
      if (combatDeclarationActive) return;
      setSelectedObjectId(objectId);
    },
    [combatDeclarationActive]
  );

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

      if (ds.actions.length === 1) {
        const onlyAction = ds.actions[0];
        window.__castParticles?.(e.clientX, e.clientY, ds.glowKind || "spell");
        dispatch(
          { type: "priority_action", action_index: onlyAction.index },
          onlyAction.label
        );
        if (!combatDeclarationActive && ds.objectId != null) setSelectedObjectId(ds.objectId);
        return;
      }

      // Multiple possible actions: pin inspector to this card so choices
      // are shown inside the card inspector (instead of a floating popover).
      if (!combatDeclarationActive) {
        setSelectedObjectId(ds.objectId != null ? ds.objectId : null);
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
  }, [clearHover, combatDeclarationActive, dispatch, endDrag]);

  useEffect(() => {
    const onDeadZonePointerDown = (event) => {
      if (event.button !== 0) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (target.closest("[data-object-id]")) return;
      if (target.closest("[data-inspector-action]")) return;
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
      clearHover();
    };

    document.addEventListener("pointerdown", onDeadZonePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onDeadZonePointerDown, true);
    };
  }, [clearHover]);

  return (
    <section
      className="relative grid gap-2 min-h-0 h-full overflow-hidden"
      style={{
        gridTemplateRows: `minmax(0,1fr) ${actionBridgeHeight}px ${handRowHeight}px`,
      }}
    >
      <DragOverlay />
      <CastParticles />
      <ArrowOverlay />
      <RightRail
        pinnedObjectId={selectedObjectId}
        onInspectObject={handleInspectObject}
      />
      {addCardError && !dismissedAddCardError && (
        <button
          type="button"
          className="add-card-error-toast absolute top-2 right-2 z-[120] max-w-[min(420px,48vw)] rounded border border-[#9f2b2b] bg-[rgba(24,8,8,0.96)] px-3 py-2 text-left text-[13px] font-semibold leading-tight text-[#ff7f7f] shadow-[0_10px_26px_rgba(0,0,0,0.45)] hover:border-[#c04040] hover:text-[#ff9f9f] transition-colors"
          onClick={() => setDismissedAddCardError(true)}
          title="Click to dismiss"
        >
          {addCardError}
        </button>
      )}
      <div
        className="min-h-0 h-full overflow-hidden"
        style={{ paddingRight: reserveInspectorSpace ? inspectorReservedWidth : "0px" }}
      >
        <TableCore
          selectedObjectId={selectedObjectId}
          onInspect={handleInspectObject}
          zoneViews={zoneViews}
          setZoneViews={setZoneViews}
          deckLoadingMode={deckLoadingMode}
          onLoadDecks={onLoadDecks}
          onCancelDeckLoading={onCancelDeckLoading}
        />
      </div>
      <div className="relative z-20 flex items-center">
        <div className="h-full w-full rounded border border-[#2b3f57]/65 bg-[linear-gradient(90deg,rgba(7,15,23,0.92),rgba(14,28,44,0.86),rgba(7,15,23,0.92))] shadow-[inset_0_1px_0_rgba(170,208,245,0.12),0_8px_18px_rgba(0,0,0,0.32)]" />
        <DecisionPopupLayer priorityInline />
      </div>
      <div className="min-h-0 h-full overflow-visible">
        <HandZone player={me} selectedObjectId={selectedObjectId} onInspect={handleInspectObject} />
      </div>
    </section>
  );
}
