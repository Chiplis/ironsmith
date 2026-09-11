import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider, useHoveredObjectId } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import HandZone from "../src/components/board/HandZone";
import ActionPopover from "../src/components/overlays/ActionPopover";
import "../src/index.css";

const { state, actions, anchorRect } = window.__actionPopoverHoverFixture;

/** The inspector reads the same hover state, so the probe stands in for it. */
function HoverProbe() {
  const hoveredObjectId = useHoveredObjectId();
  useEffect(() => { window.__hoveredObjectId = hoveredObjectId; }, [hoveredObjectId]);
  return null;
}

export function Fixture() {
  return (
    <I18nProvider>
      <GameContext.Provider value={{ state, multiplayer: null }}>
        <HoverProvider>
          <DragProvider>
            <HoverProbe />
            <div style={{ position: "fixed", inset: 0, display: "flex", alignItems: "flex-end", background: "#14171c" }}>
              <div data-hand-case style={{ position: "relative", width: "100%", height: 300 }}>
                <HandZone player={state.players[0]} selectedObjectId={null} onInspect={() => {}} isExpanded layout="fan" />
              </div>
            </div>
            <ActionPopover
              anchorRect={anchorRect}
              actions={actions}
              collapseEquivalentActions={false}
              variant="game"
              onAction={() => {}}
              onClose={() => {}}
            />
          </DragProvider>
        </HoverProvider>
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
