import DecisionPanel from "@/components/left-rail/DecisionPanel";
import InspectorPanel from "./InspectorPanel";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";

export default function RightRail({ pinnedObjectId }) {
  const { state } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const topStackObject = (state?.stack_objects || [])[0];
  const resolvingCastObjectId = state?.stack_size > 0 && topStackObject && !topStackObject.ability_kind
    ? String(topStackObject.id)
    : null;
  const selectedObjectId = hoveredObjectId ?? resolvingCastObjectId ?? pinnedObjectId;

  return (
    <aside className="rail-gradient rounded flex flex-col min-h-0 overflow-hidden">
      <div className="flex flex-col flex-1 min-h-0">
        <div className="basis-[44%] max-h-[44%] min-h-0 shrink overflow-hidden">
          <DecisionPanel />
        </div>
        <div className="flex-1 min-h-0 overflow-hidden">
          <InspectorPanel selectedObjectId={selectedObjectId} pinnedObjectId={pinnedObjectId} />
        </div>
      </div>
    </aside>
  );
}
