import { useState } from "react";
import LeftRail from "@/components/left-rail/LeftRail";
import TableCore from "@/components/board/TableCore";
import RightRail from "@/components/right-rail/RightRail";

export default function Workspace({ zoneView }) {
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [selectedObjectId, setSelectedObjectId] = useState(null);

  return (
    <section
      className="grid gap-2 min-h-0 h-full"
      style={{
        gridTemplateColumns: "15vw minmax(0,1fr) clamp(286px,23vw,390px)",
      }}
    >
      <LeftRail />
      <TableCore
        selectedObjectId={selectedObjectId}
        onInspect={setSelectedObjectId}
        zoneView={zoneView}
      />
      <RightRail
        inspectorOpen={inspectorOpen}
        setInspectorOpen={setInspectorOpen}
        selectedObjectId={selectedObjectId}
        onInspect={setSelectedObjectId}
      />
    </section>
  );
}
