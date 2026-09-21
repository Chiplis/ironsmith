import { useState } from "react";
import CompetitiveDeckBrowser from "@/components/board/CompetitiveDeckBrowser";
import { listSavedDeckPresets } from "@/lib/decklists";

export default function CompetitiveDeckPicker({ onApply, format = "modern" }) {
  const [savedDecks] = useState(() => listSavedDeckPresets());

  return (
    <div className="lobby-deck-picker flex h-[560px] min-w-0 flex-col bg-[#0e0f11] p-3">
      <CompetitiveDeckBrowser
        initialFormat={format}
        savedDecks={savedDecks}
        onSelect={(deck) => onApply?.({
          deckText: deck.deckText,
          commanderText: deck.commanderText || "",
          deck,
        })}
      />
    </div>
  );
}
