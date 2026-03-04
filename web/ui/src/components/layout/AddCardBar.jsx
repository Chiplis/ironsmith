import { useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { formatStep } from "@/lib/constants";
import { Badge } from "@/components/ui/badge";
import { Slider } from "@/components/ui/slider";

const pill = "text-[13px] uppercase cursor-pointer hover:brightness-125 transition-all select-none";
const inputPill = "rounded-full bg-secondary text-secondary-foreground px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none focus:ring-1 focus:ring-primary/50";
const selectPill = "rounded-full bg-secondary text-secondary-foreground px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";

export default function AddCardBar() {
  const { game, state, refresh, setStatus, semanticThreshold, setSemanticThreshold, cardsMeetingThreshold } = useGame();
  const [cardName, setCardName] = useState("");
  const [zone, setZone] = useState("battlefield");
  const [playerIndex, setPlayerIndex] = useState(0);
  const [skipTriggers, setSkipTriggers] = useState(false);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;

  const handleAdd = useCallback(async () => {
    const name = cardName.trim();
    if (!name) {
      setStatus("Enter a card name to add", true);
      return;
    }
    if (!game || typeof game.addCardToZone !== "function") {
      setStatus("This WASM build does not expose addCardToZone", true);
      return;
    }
    try {
      await game.addCardToZone(playerIndex || perspective, name, zone, skipTriggers);
      setCardName("");
      await refresh(`Added ${name} to ${zone}`);
    } catch (err) {
      setStatus(`Add card failed: ${err}`, true);
    }
  }, [cardName, game, playerIndex, perspective, zone, skipTriggers, refresh, setStatus]);

  return (
    <div className="panel-gradient flex items-center gap-1.5 rounded px-2.5 py-1">
      <Badge variant="secondary" className={pill} onClick={handleAdd}>Add</Badge>

      <input
        className={`${inputPill} w-36`}
        placeholder="Card name"
        value={cardName}
        onChange={(e) => setCardName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            handleAdd();
          }
        }}
      />

      <select
        className={selectPill}
        value={playerIndex || perspective}
        onChange={(e) => setPlayerIndex(Number(e.target.value))}
      >
        {players.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>

      <select
        className={selectPill}
        value={zone}
        onChange={(e) => setZone(e.target.value)}
      >
        <option value="battlefield">Battlefield</option>
        <option value="hand">Hand</option>
        <option value="graveyard">Graveyard</option>
        <option value="exile">Exile</option>
      </select>

      <label className="flex items-center gap-1 text-muted-foreground text-[13px] whitespace-nowrap cursor-pointer uppercase">
        <input
          type="checkbox"
          checked={skipTriggers}
          onChange={(e) => setSkipTriggers(e.target.checked)}
          className="h-3 w-3"
        />
        Skip triggers
      </label>

      <span className="mx-1 text-muted-foreground/40">|</span>

      <span className="text-muted-foreground text-[13px] uppercase whitespace-nowrap">Fidelity</span>
      <Slider
        className="w-20"
        min={0}
        max={100}
        step={1}
        value={[Math.round(semanticThreshold)]}
        onValueChange={([v]) => setSemanticThreshold(v)}
      />
      <span className="text-muted-foreground text-[13px] tabular-nums whitespace-nowrap">
        {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : "Off"}
        {" "}({cardsMeetingThreshold})
      </span>

      <span className="mx-1 text-muted-foreground/40">|</span>

      <Badge variant="secondary" className="text-[13px] uppercase">
        Turn {state?.turn_number ?? "-"}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        Phase {state?.phase ?? "-"}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        Step {formatStep(state?.step)}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        Active {(() => { const p = (state?.players || []).find(p => p.id === state?.active_player); return p?.name || "-"; })()}
      </Badge>
      {state?.priority_player != null && (() => { const p = (state?.players || []).find(p => p.id === state?.priority_player); return p ? (
        <Badge variant="secondary" className="text-[13px] uppercase">
          Priority {p.name}
        </Badge>
      ) : null; })()}

      <div className="flex-1" />
    </div>
  );
}
