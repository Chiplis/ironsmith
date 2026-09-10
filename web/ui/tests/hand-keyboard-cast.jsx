import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider, useDragState } from "../src/context/DragContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import HandZone from "../src/components/board/HandZone";
import { HAND_KEYBOARD_CAST_EVENT } from "../src/lib/hand-cast-keyboard";
import "../src/index.css";

const { state } = window.__handKeyboardFixture;

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

export function Fixture() {
  return (
    <I18nProvider>
      <GameContext.Provider value={{ state, multiplayer: null }}>
        <HoverProvider>
          <DragProvider>
            <DragProbe />
            <div data-hand-case style={{ position: "relative", width: 1100, height: 320, background: "#14171c" }}>
              <HandZone player={state.players[0]} selectedObjectId={null} onInspect={() => {}} isExpanded layout="fan" />
            </div>
          </DragProvider>
        </HoverProvider>
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
