import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronRight } from "lucide-react";
import InspectorPanel from "./InspectorPanel";
import StackPanel from "./StackPanel";
import TurnPane from "@/components/left-rail/TurnPane";

export default function RightRail({ inspectorOpen, setInspectorOpen, selectedObjectId, onInspect }) {
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

      <StackPanel onInspect={onInspect} />
      <TurnPane />
    </aside>
  );
}
