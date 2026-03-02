import { GameProvider } from "@/context/GameContext";
import { TooltipProvider } from "@/components/ui/tooltip";
import Shell from "@/components/layout/Shell";

export default function App() {
  return (
    <GameProvider>
      <TooltipProvider>
        <Shell />
      </TooltipProvider>
    </GameProvider>
  );
}
