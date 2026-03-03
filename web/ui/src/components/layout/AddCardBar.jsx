import { useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { formatStep } from "@/lib/constants";
import { Badge } from "@/components/ui/badge";
import { Slider } from "@/components/ui/slider";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

const pill = "text-[13px] uppercase cursor-pointer hover:brightness-125 transition-all select-none";
const inputPill = "rounded-full bg-secondary text-secondary-foreground px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none focus:ring-1 focus:ring-primary/50";
const selectPill = "rounded-full bg-secondary text-secondary-foreground px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";

export default function AddCardBar({ zoneView, setZoneView }) {
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

      <Badge variant="secondary" className={pill} onClick={handleAdd}>Add</Badge>

      <span className="mx-1 text-muted-foreground/40">|</span>

      <span className="text-muted-foreground text-[13px] uppercase whitespace-nowrap">Fidelity</span>
      <Slider
        className="w-20"
        min={0}
        max={100}
        step={1}
        value={[Math.round(semanticThreshold * 100)]}
        onValueChange={([v]) => setSemanticThreshold(v / 100)}
      />
      <span className="text-muted-foreground text-[13px] tabular-nums whitespace-nowrap">
        {semanticThreshold > 0 ? `${Math.round(semanticThreshold * 100)}%` : "Off"}
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

      <div className="flex-1" />

      <svg className="text-muted-foreground shrink-0" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
        <circle cx="12" cy="12" r="3" />
      </svg>

      <ToggleGroup type="single" value={zoneView} onValueChange={(v) => v && setZoneView(v)} className="gap-0.5">
        {["battlefield", "hand", "graveyard", "exile"].map((z) => (
          <ToggleGroupItem
            key={z}
            value={z}
            className="h-6 px-1.5 text-[14px] uppercase tracking-wide data-[state=on]:bg-[rgba(62,98,138,0.35)] data-[state=on]:text-foreground"
          >
            {z === "battlefield" ? "Battlefield" : z.charAt(0).toUpperCase() + z.slice(1)}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </div>
  );
}
