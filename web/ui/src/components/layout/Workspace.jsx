import { useState } from "react";
import TableCore from "@/components/board/TableCore";
import RightRail from "@/components/right-rail/RightRail";

export default function Workspace({ zoneView, deckLoadingMode, onLoadDecks, onCancelDeckLoading }) {
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [selectedObjectId, setSelectedObjectId] = useState(null);

  return (
    <section
      className="grid gap-2 min-h-0 h-full"
      style={{
        gridTemplateColumns: "clamp(286px,23vw,390px) minmax(0,1fr)",
      }}
    >
      <RightRail
        inspectorOpen={inspectorOpen}
        setInspectorOpen={setInspectorOpen}
        selectedObjectId={selectedObjectId}
        onInspect={setSelectedObjectId}
      />
      <TableCore
        selectedObjectId={selectedObjectId}
        onInspect={setSelectedObjectId}
        zoneView={zoneView}
        deckLoadingMode={deckLoadingMode}
        onLoadDecks={onLoadDecks}
        onCancelDeckLoading={onCancelDeckLoading}
      />
    </section>
  );
}
