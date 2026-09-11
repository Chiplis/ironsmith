import { GameProvider } from "@/context/GameContext";
import { HoverProvider } from "@/context/HoverContext";
import { DragProvider } from "@/context/DragContext";
import { ObjectSelectionProvider } from "@/context/ObjectSelectionContext";
import { CombatArrowProvider } from "@/context/CombatArrowContext";
import { I18nProvider } from "@/i18n/I18nContext";
import { TooltipProvider } from "@/components/ui/tooltip";
import Shell from "@/components/layout/Shell";

export default function App() {
  return (
    <I18nProvider>
      <GameProvider>
        <ObjectSelectionProvider>
          <HoverProvider>
            <DragProvider>
              <CombatArrowProvider>
                <TooltipProvider>
                  <Shell />
                </TooltipProvider>
              </CombatArrowProvider>
            </DragProvider>
          </HoverProvider>
        </ObjectSelectionProvider>
      </GameProvider>
    </I18nProvider>
  );
}
