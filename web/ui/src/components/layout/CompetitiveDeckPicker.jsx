import useUiText from "@/i18n/useUiText";
import useSavedDecks from "@/hooks/useSavedDecks";
import CompetitiveDeckBrowser from "@/components/board/CompetitiveDeckBrowser";
import { splitSavedLobbyDeck } from "@/lib/decklists";

export default function CompetitiveDeckPicker({ onApply, format = "modern", lockFormat = false, allowUnreleased = false }) {
  const ui = useUiText();
  const { decks: savedDecks, error } = useSavedDecks();

  return (
    <div className="lobby-deck-picker flex h-[560px] min-w-0 flex-col bg-[#0e0f11] p-3">
      {error && <p role="alert" className="text-sm text-[#f0a9a0]">{ui("Could not load saved decks. Try reopening this page.")}</p>}
      <CompetitiveDeckBrowser
        key={format}
        initialFormat={format}
        lockFormat={lockFormat}
        legalityFormat={lockFormat ? format : null}
        allowUnreleased={allowUnreleased}
        savedDecks={savedDecks}
        onSelect={(deck) => onApply?.({
          ...splitSavedLobbyDeck(deck.deckText),
          ...(deck.commanderText ? { commanderText: deck.commanderText } : {}),
          deck,
        })}
      />
    </div>
  );
}
