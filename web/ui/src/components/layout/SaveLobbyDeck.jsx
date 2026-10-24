import useSavedDecks from "@/hooks/useSavedDecks";
import { useState } from "react";
import useUiText from "@/i18n/useUiText";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { saveSavedDeckPreset, serializeLobbyDeck } from "@/lib/decklists";

export default function SaveLobbyDeck({ deckText, commanderText = "", format, playerName }) {
  const ui = useUiText();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const { decks, loading, error: storageError } = useSavedDecks();
  const [saving, setSaving] = useState(false);
  const existing = decks.find(deck => deck.name.toLowerCase() === name.trim().toLowerCase());
  const save = async (event) => {
    event.preventDefault();
    if (saving || loading) return;
    setSaving(true);
    const result = await saveSavedDeckPreset(name, [serializeLobbyDeck(deckText, commanderText)], [playerName], [format]);
    setSaving(false);
    if (!result.saved) {
      setError(ui("Could not save this deck. Check that browser storage is available and try again."));
      return;
    }
    setOpen(false);
    setSaved(true);
  };
  return (
    <>
      <Button type="button" variant="secondary" disabled={!format || !deckText.trim()} onClick={() => { setName(""); setError(""); setSaved(false); setOpen(true); }}>{ui("Save Deck")}</Button>
      {saved && <span role="status" className="text-sm">{ui("Deck saved")}</span>}
      <Sheet open={open} onOpenChange={(value) => { if (!saving) setOpen(value); }}>
        <SheetContent side="center" className="w-[min(440px,calc(100vw-32px))] p-6">
          <SheetHeader>
            <SheetTitle>{ui("Save Deck")}</SheetTitle>
            <SheetDescription>{ui("Name your deck to keep it in My Decks on this browser between sessions.")}</SheetDescription>
          </SheetHeader>
          <form onSubmit={save} className="grid gap-4 mt-4">
            <label className="grid gap-2 text-sm">
              {ui("Deck name")}
              <input autoFocus required disabled={saving} maxLength={100} value={name} onChange={(event) => { setName(event.target.value); setError(""); }} className="fantasy-field w-full px-3 py-2" />
            </label>
            {existing && <p className="text-sm">{ui('Saving will replace the deck named "{0}".', { 0: existing.name })}</p>}
            {(error || storageError) && <p role="alert" className="text-sm text-[#f0a9a0]">{error || ui("Could not save this deck. Check that browser storage is available and try again.")}</p>}
            <div className="flex justify-end gap-2">
              <Button type="button" disabled={saving} variant="secondary" onClick={() => setOpen(false)}>{ui("Cancel")}</Button>
              <Button type="submit" disabled={!name.trim() || saving || loading || Boolean(storageError)}>{existing ? ui("Replace deck") : ui("Save")}</Button>
            </div>
          </form>
        </SheetContent>
      </Sheet>
    </>
  );
}
