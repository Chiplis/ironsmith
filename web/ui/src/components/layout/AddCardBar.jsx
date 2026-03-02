import { useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

export default function AddCardBar({ zoneView, setZoneView }) {
  const { game, state, refresh, setStatus } = useGame();
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
    <div className="panel-gradient flex items-center gap-2 rounded px-2.5 py-1">
      <select
        className="h-6 text-[11px] px-1.5 bg-[#0b1118] border border-[#344a61] text-foreground rounded-sm uppercase tracking-wide cursor-pointer"
        value={playerIndex || perspective}
        onChange={(e) => setPlayerIndex(Number(e.target.value))}
      >
        {players.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>

      <Input
        className="h-6 text-[12px] w-36 bg-transparent"
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
        className="h-6 text-[11px] px-1.5 bg-[#0b1118] border border-[#344a61] text-foreground rounded-sm uppercase tracking-wide cursor-pointer"
        value={zone}
        onChange={(e) => setZone(e.target.value)}
      >
        <option value="battlefield">Battlefield</option>
        <option value="hand">Hand</option>
        <option value="graveyard">Graveyard</option>
        <option value="exile">Exile</option>
      </select>

      <label className="flex items-center gap-1 text-muted-foreground text-[11px] whitespace-nowrap cursor-pointer">
        <input
          type="checkbox"
          checked={skipTriggers}
          onChange={(e) => setSkipTriggers(e.target.checked)}
          className="h-3 w-3"
        />
        Skip triggers
      </label>

      <Button variant="outline" size="sm" className="h-6 text-[12px] px-2" onClick={handleAdd}>
        ADD
      </Button>

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
            className="h-6 px-1.5 text-[11px] uppercase tracking-wide data-[state=on]:bg-[rgba(62,98,138,0.35)] data-[state=on]:text-foreground"
          >
            {z === "battlefield" ? "Battlefield" : z.charAt(0).toUpperCase() + z.slice(1)}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </div>
  );
}
