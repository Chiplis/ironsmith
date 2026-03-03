import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useGame } from "@/context/GameContext";
import InspectorPanel from "./InspectorPanel";
import StackPanel from "./StackPanel";
import TurnPane from "@/components/left-rail/TurnPane";
import DecisionPanel from "@/components/left-rail/DecisionPanel";

export default function RightRail({ inspectorOpen, setInspectorOpen, selectedObjectId, onInspect }) {
  const { state } = useGame();
  const stackHasContent =
    (state?.stack_objects?.length > 0) || (state?.stack_preview?.length > 0);

  return (
    <aside className="rail-gradient rounded flex flex-col min-h-0 overflow-hidden">
      <Button
        variant="ghost"
        size="sm"
        className="shrink-0 text-[14px] uppercase tracking-wide w-full justify-start rounded-none border-b border-game-line-2 h-7 gap-1"
        onClick={() => setInspectorOpen((o) => !o)}
      >
        {inspectorOpen ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
        Inspector
      </Button>

      {inspectorOpen && (
        <InspectorPanel selectedObjectId={selectedObjectId} />
      )}

      <div className="flex flex-col flex-1 min-h-0">
        <StackPanel onInspect={onInspect} />
        <div className={`border-t border-game-line-2 overflow-auto p-1.5 ${stackHasContent ? "max-h-[75%]" : "flex-1"}`}>
          <DecisionPanel />
        </div>
      </div>
      <TurnPane />
    </aside>
  );
}
