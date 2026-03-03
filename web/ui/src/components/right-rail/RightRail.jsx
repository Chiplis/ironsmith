import DecisionPanel from "@/components/left-rail/DecisionPanel";
import InspectorPanel from "./InspectorPanel";

export default function RightRail({ selectedObjectId, pinnedObjectId }) {
  return (
    <aside className="rail-gradient rounded flex flex-col min-h-0 overflow-hidden">
      <div className="flex flex-col flex-1 min-h-0">
        <DecisionPanel />
        <div className="flex-1 min-h-0 overflow-hidden">
          <InspectorPanel selectedObjectId={selectedObjectId} pinnedObjectId={pinnedObjectId} />
        </div>
      </div>
    </aside>
  );
}
