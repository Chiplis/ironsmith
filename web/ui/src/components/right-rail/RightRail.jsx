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
  const resolvingCastStableId = resolvingCastObjectId && topStackObject?.stable_id != null
    ? Number(topStackObject.stable_id)
    : null;
  const selectedObjectId = hoveredObjectId ?? resolvingCastObjectId ?? pinnedObjectId;
  const suppressDirectResolvingCastInspector =
    selectedObjectId != null
    && resolvingCastObjectId != null
    && String(selectedObjectId) === String(resolvingCastObjectId);
  const shouldShowInspector = selectedObjectId != null && !suppressDirectResolvingCastInspector;

  return (
    <aside className="rail-gradient rounded flex flex-col min-h-0 overflow-hidden">
      <div className="flex flex-col flex-1 min-h-0">
        <div className="flex-1 min-h-0 overflow-hidden">
          <DecisionPanel />
        </div>
        {shouldShowInspector && (
          <>
            <div className="shrink-0 border-t border-game-line-2/70 mx-1" aria-hidden="true" />
            <div className="shrink-0 min-h-0 max-h-[58%] overflow-hidden">
              <InspectorPanel
                selectedObjectId={selectedObjectId}
                pinnedObjectId={pinnedObjectId}
                resolvingCastStableId={resolvingCastStableId}
              />
            </div>
          </>
        )}
      </div>
    </aside>
  );
}
