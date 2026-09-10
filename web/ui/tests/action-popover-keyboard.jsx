import { createRoot } from "react-dom/client";
import { HoverProvider } from "../src/context/HoverContext";
import ActionPopover from "../src/components/overlays/ActionPopover";
import "../src/index.css";

const { actions, anchorRect } = window.__popoverFixture;
window.__chosen = [];
window.__closed = 0;

createRoot(document.getElementById("root")).render(
  <HoverProvider>
    <ActionPopover
      anchorRect={anchorRect}
      actions={actions}
      collapseEquivalentActions={false}
      focusOnOpen
      ariaLabel="Ways to play Dark Ritual"
      variant="game"
      previewCards={false}
      onAction={(action) => window.__chosen.push(action.label)}
      onClose={() => { window.__closed += 1; }}
    />
  </HoverProvider>
);
