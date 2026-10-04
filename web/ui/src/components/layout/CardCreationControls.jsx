import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import useUiText from "@/i18n/useUiText";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import AddCardSheet from "./AddCardSheet";

const triggerPill = "stone-pill table-zone-action-button inline-flex items-center justify-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45";

export default function CardCreationControls({ onAddCardNotice, shortLabels = false }) {
  const { state, multiplayer } = useGame();
  const { t } = useI18n();
  const ui = useUiText();
  const [zone, setZone] = useState("battlefield");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);
  const players = state?.players || [];
  const selectedPlayer = playerIndex ?? state?.perspective ?? 0;
  const addLocked = multiplayer.mode !== "idle" && !multiplayer.matchStarted;

  return (
    <>
      <CreateCardForgeSheet
        disabled={addLocked}
        players={players}
        selectedPlayer={selectedPlayer}
        onSelectPlayer={setPlayerIndex}
        zone={zone}
        onZoneChange={setZone}
        skipTriggers={skipTriggers}
        onSkipTriggersChange={(checked) => setSkipTriggers(checked === true)}
        trigger={(
          <button
            type="button"
            className={triggerPill}
            disabled={addLocked}
          >
            {shortLabels ? ui("Compile") : t("action.compileCard")}
          </button>
        )}
      />
      <AddCardSheet
        onAddCardNotice={onAddCardNotice}
        trigger={(
          <button
            type="button"
            className={triggerPill}
            disabled={addLocked}
          >
            {shortLabels ? ui("Add card") : t("action.addCard")}
          </button>
        )}
      />
    </>
  );
}
