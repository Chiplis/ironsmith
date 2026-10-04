import { GameProvider } from "@/context/GameContext";
import { HoverProvider } from "@/context/HoverContext";
import { DragProvider } from "@/context/DragContext";
import { ObjectSelectionProvider } from "@/context/ObjectSelectionContext";
import { CombatArrowProvider } from "@/context/CombatArrowContext";
import { I18nProvider } from "@/i18n/I18nContext";
import { TooltipProvider } from "@/components/ui/tooltip";
import Shell from "@/components/layout/Shell";
import VisualIndicatorsPreview from "@/components/layout/VisualIndicatorsPreview";

const visualIndicatorsPreview = typeof window !== "undefined"
  && new URLSearchParams(window.location.search).get("test") === "visual-indicators";

export default function App() {
  return (
    <I18nProvider>
      <GameProvider>
        <ObjectSelectionProvider>
          <HoverProvider>
            <DragProvider>
              <CombatArrowProvider>
                <TooltipProvider>
                  {visualIndicatorsPreview ? <VisualIndicatorsPreview /> : <Shell />}
                </TooltipProvider>
              </CombatArrowProvider>
            </DragProvider>
          </HoverProvider>
        </ObjectSelectionProvider>
      </GameProvider>
    </I18nProvider>
  );
}
