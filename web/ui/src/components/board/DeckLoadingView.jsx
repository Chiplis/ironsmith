import { useState, useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { Badge } from "@/components/ui/badge";
import { parseDeckList } from "@/lib/decklists";

const pill = "text-[13px] uppercase cursor-pointer hover:brightness-125 transition-all select-none";

export default function DeckLoadingView({ onLoad, onCancel }) {
  const { state } = useGame();
  const players = state?.players || [];
  const [texts, setTexts] = useState(() => players.map(() => ""));

  const handleTextChange = (index, value) => {
    setTexts((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  };

  const cardCounts = useMemo(
    () => texts.map((t) => parseDeckList(t).length),
    [texts]
  );
  const totalCards = cardCounts.reduce((a, b) => a + b, 0);

  const handleLoad = () => {
    const decks = texts.map(parseDeckList);
    onLoad(decks);
  };

  return (
    <main
      className="table-gradient rounded grid gap-1.5 p-1.5 min-h-0 overflow-hidden"
      style={{ gridTemplateRows: "minmax(0,1fr) auto" }}
    >
      <div
        className="grid gap-1.5 min-h-0"
        style={{ gridTemplateColumns: `repeat(${players.length}, minmax(0, 1fr))` }}
      >
        {players.map((player, i) => (
          <div
            key={player.id}
            className="border border-[#2b3e55] bg-gradient-to-b from-[#101826] to-[#0a121d] p-2 grid gap-1.5 min-h-0"
            style={{ gridTemplateRows: "auto minmax(0,1fr)" }}
          >
            <div className="flex justify-between items-baseline">
              <span className="text-[16px] text-[#a4bdd7] uppercase tracking-wider font-bold">
                {player.name}
              </span>
              <span className="text-[14px] text-muted-foreground">
                {cardCounts[i]} cards
              </span>
            </div>
            <textarea
              className="w-full min-h-0 h-full bg-[#0b1118] border border-[#344a61] text-foreground text-[14px] p-2 rounded resize-none font-mono focus:outline-none focus:border-primary/50"
              placeholder={`Paste ${player.name}'s deck list...\n\nMTGA / MTGO / Moxfield format:\n4 Lightning Bolt\n2 Counterspell\n20 Island`}
              value={texts[i] || ""}
              onChange={(e) => handleTextChange(i, e.target.value)}
            />
          </div>
        ))}
      </div>
      <div className="flex items-center justify-center gap-2 py-1">
        <Badge variant="secondary" className={`${pill} px-4`} onClick={handleLoad}>
          Load{totalCards > 0 ? ` (${totalCards} cards)` : ""}
        </Badge>
        <Badge variant="secondary" className={pill} onClick={onCancel}>
          Cancel
        </Badge>
      </div>
    </main>
  );
}
