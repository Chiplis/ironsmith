import { registerCustomCardFlavor } from "@/lib/custom-card-flavor";
import { useState } from "react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";
import { sampleCustomCards } from "@/lib/custom-card-pool";
import { setCompiledCardNames } from "@/lib/scryfall";

export default function CustomCardsButton({ className = "", label = "Custom cards" }) {
  const { game, state, multiplayer, runWasmInteraction, refresh, setStatus } = useGame();
  const ui = useUiText();
  const [busy, setBusy] = useState(false);
  const locked = multiplayer.mode !== "idle" || multiplayer.matchStarted;
  const refill = async () => {
    if (locked || busy || !game) return;
    setBusy(true);
    try {
      const response = await fetch(`${import.meta.env.BASE_URL}custommagic/pool.json`);
      if (!response.ok) throw new Error("The verified CustomMagic card pool is not available yet");
      const pool = await response.json();
      const selected = sampleCustomCards(pool.cards);
      await runWasmInteraction(async () => {
        await game.replaceHandWithCustomCards(state?.perspective ?? 0, selected.map(card => card.draft));
        registerCustomCardFlavor(selected);
        setCompiledCardNames(selected.flatMap(card => card.draft.faces.map(face => face.name)));
        await refresh("Replaced your hand with 7 random custom cards");
      });
    } catch (error) {
      setStatus(`Custom cards: ${String(error?.message || error)}`, true);
    } finally {
      setBusy(false);
    }
  };
  return (
    <button type="button" className={className} disabled={locked || busy || !game}
      aria-busy={busy} onClick={() => void refill()}
      title={ui("Replace your hand with 7 random custom cards")}>
      {ui(busy ? "Loading custom cards…" : label)}
    </button>
  );
}
