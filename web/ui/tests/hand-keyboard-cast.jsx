import HandPriorityAction from '../src/components/decisions/HandPriorityAction';
import useHandPriorityAction from '../src/hooks/useHandPriorityAction';
import { installMainDecisionShortcut } from '../src/lib/main-decision-shortcut';
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider, useDragState, useDragSession } from "../src/context/DragContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import HandZone from "../src/components/board/HandZone";
import DragOverlay from "../src/components/overlays/DragOverlay";
import { HAND_KEYBOARD_CAST_EVENT } from "../src/lib/hand-cast-keyboard";
import { installHandToggleShortcut, useHandOpen } from "../src/lib/hand-open";
import "../src/index.css";

const { state: initialState, mainAction } = window.__handKeyboardFixture;

window.__keyboardCasts = [];
window.addEventListener(HAND_KEYBOARD_CAST_EVENT, (event) => {
  const { objectId, cardName, actions, glowKind, anchorRect } = event.detail || {};
  window.__keyboardCasts.push({
    objectId,
    cardName,
    glowKind,
    anchorRect,
    actions: actions.map((action) => action.label),
  });
});

/** The provider owns the hold, so the probe reads it the way the board does. */
export function DragProbe() {
  const dragState = useDragState();
  const session = useDragSession();
  useEffect(() => { window.__dragSessionUpdates = (window.__dragSessionUpdates || 0) + 1; }, [session]);
  useEffect(() => {
    window.__dragState = dragState
      ? {
        objectId: dragState.objectId,
        cardName: dragState.cardName,
        keyboard: Boolean(dragState.keyboard),
        currentX: dragState.currentX,
        currentY: dragState.currentY,
        startX: dragState.startX,
        startY: dragState.startY,
        actions: dragState.actions.map((action) => action.label),
      }
      : null;
  }, [dragState]);
  return null;
}

function HandActionDock({ state }) {
  const action = useHandPriorityAction(state);
  return <div style={{ position: 'fixed', top: 20, left: 20, zIndex: 100 }}>
    {action.active ? <HandPriorityAction action={action} /> : <button className="decision-main-button" onClick={() => { window.__passed = true; }}>Pass priority</button>}
  </div>;
}

export function Fixture() {
  const [state, setState] = useState(initialState);
  window.__setHandFixtureState = setState;
  useEffect(() => mainAction ? installMainDecisionShortcut(document) : undefined, []);
  const handOpen = useHandOpen();
  useEffect(() => installHandToggleShortcut(document), []);
  return (
    <I18nProvider>
      <GameContext.Provider value={{ state, multiplayer: null }}>
        <HoverProvider>
          <DragProvider>
            {mainAction && <HandActionDock state={state} />}
            <DragProbe />
            <DragOverlay />
            {/* The hand sits at the foot of the viewport, as it does on the
                board, so an arrow lifted towards the middle points upwards. */}
            <div style={{ position: "fixed", inset: 0, display: "flex", alignItems: "flex-end", background: "#14171c" }}>
              <div data-hand-case data-hand-open={handOpen} style={{ position: "relative", width: "100%", height: 300 }}>
                <HandZone player={state.players[0]} selectedObjectId={null} onInspect={() => {}} isExpanded layout="fan" />
              </div>
            </div>
          </DragProvider>
        </HoverProvider>
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
