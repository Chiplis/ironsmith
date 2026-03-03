import { GameProvider } from "@/context/GameContext";
import { HoverProvider } from "@/context/HoverContext";
import { DragProvider } from "@/context/DragContext";
import { CombatArrowProvider } from "@/context/CombatArrowContext";
import { TooltipProvider } from "@/components/ui/tooltip";
import Shell from "@/components/layout/Shell";

export default function App() {
  return (
    <GameProvider>
      <HoverProvider>
        <DragProvider>
          <CombatArrowProvider>
            <TooltipProvider>
              <Shell />
            </TooltipProvider>
          </CombatArrowProvider>
        </DragProvider>
      </HoverProvider>
    </GameProvider>
  );
}
