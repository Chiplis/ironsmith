import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Checkbox } from "@/components/ui/checkbox";
import { Slider } from "@/components/ui/slider";
import ZoneViewer from "@/components/board/ZoneViewer";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import AddCardSheet from "./AddCardSheet";

const triggerPill = "stone-pill inline-flex items-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45";
const selectPill = "stone-select rounded-none px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";

export default function AddCardBar({
  zoneViews = ["battlefield"],
  setZoneViews,
  onAddCardFailure,
}) {
  const {
    state,
    semanticThreshold,
    setSemanticThreshold,
    cardsMeetingThreshold,
    multiplayer,
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
  } = useGame();
  const [zone, setZone] = useState("hand");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const selectedPlayer = playerIndex ?? perspective;
  const addLocked = multiplayer.mode !== "idle";
  const matchLocked = multiplayer.matchStarted;

  return (
    <div className="add-card-toolbar table-toolbar table-toolbar--secondary rounded-none px-3 py-2">
      <div className="add-card-toolbar-left">
        <AddCardSheet
          onAddCardFailure={onAddCardFailure}
          trigger={(
            <button
              type="button"
              className={triggerPill}
              disabled={addLocked}
            >
              Add Card
            </button>
          )}
        />
        <CreateCardForgeSheet
          disabled={addLocked}
          players={players}
          selectedPlayer={selectedPlayer}
          onSelectPlayer={setPlayerIndex}
          zone={zone}
          onZoneChange={setZone}
          skipTriggers={skipTriggers}
          onSkipTriggersChange={(checked) => setSkipTriggers(checked === true)}
        />
        <span className="add-card-toolbar-separator" aria-hidden="true" />

        <span
          className="add-card-toolbar-meta text-[13px] uppercase whitespace-nowrap cursor-help"
          title="Controls the threshold for semantic similarity when parsing custom cards. Higher means stricter text matching."
        >
          Fidelity
        </span>
        <Slider
          className="w-20"
          min={0}
          max={100}
          step={1}
          value={[Math.round(semanticThreshold)]}
          onValueChange={([value]) => setSemanticThreshold(value)}
        />
        <span className="add-card-toolbar-meta-value text-[13px] tabular-nums whitespace-nowrap">
          {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : "Off"}
          {" "}({cardsMeetingThreshold})
        </span>
        <span className="add-card-toolbar-separator" aria-hidden="true" />
        <select
          className={selectPill}
          value={holdRule}
          disabled={matchLocked}
          onChange={(event) => setHoldRule(event.target.value)}
          aria-label="Auto-pass hold rule"
        >
          <option value="never">Never</option>
          <option value="if_actions">If actions</option>
          <option value="stack">Stack</option>
          <option value="main">Main</option>
          <option value="combat">Combat</option>
          <option value="ending">Ending</option>
          <option value="always">Always</option>
        </select>
        <label className="toolbar-checkbox add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap cursor-pointer uppercase">
          <Checkbox
            checked={autoPassEnabled}
            disabled={matchLocked}
            onCheckedChange={(value) => setAutoPassEnabled(!!value)}
            className="h-3.5 w-3.5"
          />
          Auto-pass
        </label>
      </div>
      <div className="add-card-toolbar-right">
        <ZoneViewer zoneViews={zoneViews} setZoneViews={setZoneViews} embedded />
      </div>
    </div>
  );
}
