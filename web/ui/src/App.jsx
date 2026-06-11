import { GameProvider } from "@/context/GameContext";
import { HoverProvider } from "@/context/HoverContext";
import { DragProvider } from "@/context/DragContext";
import { CombatArrowProvider } from "@/context/CombatArrowContext";
import { I18nProvider } from "@/i18n/I18nContext";
import { TooltipProvider } from "@/components/ui/tooltip";
import Shell from "@/components/layout/Shell";
import MobileLandscapeGate from "@/components/layout/MobileLandscapeGate";

export default function App() {
  return (
    <I18nProvider>
      <GameProvider>
        <HoverProvider>
          <DragProvider>
            <CombatArrowProvider>
              <TooltipProvider>
                <MobileLandscapeGate />
                <Shell />
              </TooltipProvider>
            </CombatArrowProvider>
          </DragProvider>
        </HoverProvider>
      </GameProvider>
    </I18nProvider>
  );
}
