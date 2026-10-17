import { useState } from "react";
import { Hammer } from "lucide-react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import CustomCardsButton from "./CustomCardsButton";
import SurrenderButton from "./SurrenderButton";

const triggerPill = "stone-pill table-zone-action-button inline-flex items-center justify-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45";

export default function CardCreationControls({ onAddCardNotice, navigation = false }) {
  const { state, multiplayer } = useGame();
  const ui = useUiText();
  const [zone, setZone] = useState("battlefield");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);
  const players = state?.players || [];
  const selectedPlayer = playerIndex ?? state?.perspective ?? 0;
  const addLocked = multiplayer.mode !== "idle" && !multiplayer.matchStarted;

  if (multiplayer.matchStarted || multiplayer.mode === "in_match") {
    return <SurrenderButton className={triggerPill} />;
  }

  return (
    <>
      <CreateCardForgeSheet
        disabled={addLocked}
        onAddCardNotice={onAddCardNotice}
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
            className={navigation ? "home-action home-action-build" : triggerPill}
            disabled={addLocked}
            aria-label={navigation ? ui("Build") : undefined}
            title={navigation ? ui("Build") : undefined}
          >
            {navigation ? <>
              <Hammer size={15} aria-hidden="true" className="home-action-icon" />
              <span className="home-action-label">{ui("Build")}</span>
            </> : ui("Build")}
          </button>
        )}
      />
      {!navigation && <CustomCardsButton className={triggerPill} />}
    </>
  );
}
