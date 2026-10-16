import { CUSTOM_CARD_LIBRARY_CHANGED, loadBundledCustomCards, listSavedCustomCards, customCardOptions } from "@/lib/custom-card-library";
import { registerCustomCardFlavor } from "@/lib/custom-card-flavor";
import { setCompiledCardNames, setCustomCardArtUrls } from "@/lib/scryfall";
import useUiText from "@/i18n/useUiText";
import { useState, useCallback, useEffect, useRef, useId } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";

const inputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none disabled:cursor-not-allowed disabled:opacity-50";
const labelClass =
  "grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground";
const selectClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none disabled:cursor-not-allowed disabled:opacity-50";

function formatAddCardFailureClipboard(cardName, zone, errorMessage) {
  return [
    cardName ? `Card: ${cardName}` : "",
    zone ? `Zone: ${zone}` : "",
    errorMessage ? `Error: ${errorMessage}` : "",
  ]
    .filter(Boolean)
    .join("\n\n");
}

export default function AddCardSheet({
  trigger,
  onAddCardNotice,
  triggerClassName = "",
  embedded = false,
  onClose,
  disabled = false,
}) {
  const ui = useUiText();
  const {
    game,
    state,
    refresh,
    runWasmInteraction,
    setStatus,
    multiplayer,
  } = useGame();
  const [includeCustom, setIncludeCustom] = useState(false);
  const [customCards, setCustomCards] = useState([]);
  const [libraryLoading, setLibraryLoading] = useState(true);
  const [libraryError, setLibraryError] = useState("");
  const [selectedOption, setSelectedOption] = useState(null);
  const [open, setOpen] = useState(embedded);
  const [cardName, setCardName] = useState("");
  const [zone, setZone] = useState("hand");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);
  const [autocompleteOptions, setAutocompleteOptions] = useState([]);
  const [autocompleteOpen, setAutocompleteOpen] = useState(false);
  const [autocompleteIndex, setAutocompleteIndex] = useState(-1);
  const autocompleteId = useId();
  const autocompleteRef = useRef(null);
  const cardNameInputRef = useRef(null);
  const suppressAutocompleteRef = useRef(false);
  const autocompleteRequestRef = useRef(0);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const selectedPlayer = playerIndex ?? perspective;
  const addLocked = disabled || (multiplayer.mode !== "idle" && !multiplayer.matchStarted);

  const visibleAutocompleteOptions =
    addLocked || !cardName.trim() ? [] : autocompleteOptions;
  const autocompleteVisible =
    autocompleteOpen && visibleAutocompleteOptions.length > 0;

  useEffect(() => {
    if (!open) return;
    let active = true;
    let revision = 0;
    const load = async () => {
      const request = ++revision;
      setLibraryLoading(true);
      const results = await Promise.allSettled([loadBundledCustomCards(), listSavedCustomCards()]);
      if (!active || revision !== request) return;
      setCustomCards(results.flatMap(result => result.status === "fulfilled" ? result.value : []));
      setLibraryError(results.filter(result => result.status === "rejected").map(result => String(result.reason?.message || result.reason)).join(". "));
      setLibraryLoading(false);
    };
    void load();
    window.addEventListener(CUSTOM_CARD_LIBRARY_CHANGED, load);
    window.addEventListener("focus", load);
    return () => {
      active = false;
      window.removeEventListener(CUSTOM_CARD_LIBRARY_CHANGED, load);
      window.removeEventListener("focus", load);
    };
  }, [open]);

  useEffect(() => {
    const query = cardName.trim();
    if (addLocked || !query || !game || typeof game.autocompleteCardNames !== "function") return;

    if (suppressAutocompleteRef.current) {
      suppressAutocompleteRef.current = false;
      return;
    }

    const requestId = autocompleteRequestRef.current + 1;
    autocompleteRequestRef.current = requestId;
    const timeoutId = window.setTimeout(async () => {
      try {
        const standard = await game.autocompleteCardNames(query, 5);
        const matches = [
          ...(includeCustom ? customCardOptions(customCards, query) : []),
          ...standard.map(name => ({ name })),
        ];
        if (autocompleteRequestRef.current !== requestId) return;
        setAutocompleteOptions(matches);
        setAutocompleteOpen(matches.length > 0);
        setAutocompleteIndex(matches.length === 1 ? 0 : -1);
      } catch (error) {
        if (autocompleteRequestRef.current !== requestId) return;
        console.warn("Autocomplete lookup failed:", error);
        setAutocompleteOptions([]);
        setAutocompleteOpen(false);
        setAutocompleteIndex(-1);
      }
    }, 150);

    return () => {
      window.clearTimeout(timeoutId);
      autocompleteRequestRef.current += 1;
    };
  }, [addLocked, cardName, game, includeCustom, customCards]);

  useEffect(() => {
    const handlePointerDown = (event) => {
      if (!autocompleteRef.current?.contains(event.target)) {
        setAutocompleteOpen(false);
        setAutocompleteIndex(-1);
      }
    };

    window.addEventListener("pointerdown", handlePointerDown);
    return () => window.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  useEffect(() => {
    if (!open) return;

    const frameId = window.requestAnimationFrame(() => {
      cardNameInputRef.current?.focus();
    });

    return () => window.cancelAnimationFrame(frameId);
  }, [open]);

  const closeSheet = useCallback(() => {
    setOpen(false);
    onClose?.();
    setAutocompleteOpen(false);
    setAutocompleteIndex(-1);
  }, [onClose]);

  const handleAdd = useCallback(async (requestedName = selectedOption || cardName) => {
    return runWasmInteraction(async () => {
      if (addLocked) {
        setStatus("Card injection is disabled while a lobby is active", true);
        return;
      }
      const option = typeof requestedName === "object" ? requestedName : null;
      const name = String(option?.name || requestedName || "").trim();
      const exactCustom = includeCustom && !option
        ? customCardOptions(customCards, name, customCards.length).filter(match =>
          match.name.toLowerCase() === name.toLowerCase()
          || match.card.draft.faces.some(face => face.name.toLowerCase() === name.toLowerCase()))
        : [];
      const custom = includeCustom ? (option?.card
        || (exactCustom.length === 1 ? exactCustom[0].card : null)) : null;
      if (!option && !selectedOption && exactCustom.length > 1) {
        setStatus("Choose a custom card from the search results", true);
        return;
      }
      if (!name) {
        setStatus("Enter a card name to add", true);
        return;
      }
      if (!game || typeof game.addCardToZone !== "function") {
        setStatus("This WASM build does not expose addCardToZone", true);
        return;
      }
      try {
        if (custom) {
          await game.createCustomCard({ draft: custom.draft, playerIndex: selectedPlayer, zoneName: zone, skipTriggers });
          registerCustomCardFlavor([custom]);
          setCustomCardArtUrls(custom.draft.faces.filter(face => face.artUrl).map(face => ({ name: face.name, artUrl: face.artUrl })));
          setCompiledCardNames(custom.draft.faces.map(face => face.name));
        } else {
          await game.addCardToZone(selectedPlayer, name, zone, skipTriggers);
        }
        setSelectedOption(null);
        const injectedDuringMatch = Boolean(multiplayer.matchStarted);
        setCardName("");
        setAutocompleteOptions([]);
        setAutocompleteOpen(false);
        setAutocompleteIndex(-1);
        closeSheet();
        // Keep the add-card critical path limited to the mutation + refresh.
        // Follow-up diagnostics are intentionally omitted here because a hung
        // worker call would leave the shared interaction gate blocked.
        await refresh(
          injectedDuringMatch
            ? `Added ${name} locally; peers will reject it if used`
            : `Added ${name} to ${zone}`
        );
      } catch (err) {
        const errMsg = String(err?.message || err);
        setStatus(`Add card failed: ${errMsg}`, true);
        if (typeof onAddCardNotice === "function") {
          onAddCardNotice({
            tone: "error",
            title: `Could not add ${name}`,
            body: `${errMsg} Click to copy diagnostics.`,
            copyText: formatAddCardFailureClipboard(name, zone, errMsg),
            copyStatusMessage: `Copied diagnostics for ${name}`,
          });
        }
      }
    });
  }, [
    addLocked,
    includeCustom,
    customCards,
    selectedOption,
    cardName,
    closeSheet,
    game,
    onAddCardNotice,
    refresh,
    runWasmInteraction,
    selectedPlayer,
    setStatus,
    skipTriggers,
    zone,
    multiplayer.matchStarted,
  ]);

  const handleAutocompletePick = useCallback((option) => {
    setSelectedOption(option);
    autocompleteRequestRef.current += 1;
    suppressAutocompleteRef.current = true;
    setCardName(option.name);
    setAutocompleteOptions([]);
    setAutocompleteOpen(false);
    setAutocompleteIndex(-1);
    window.requestAnimationFrame(() => {
      cardNameInputRef.current?.focus();
    });
  }, []);

  const body = (
        <div className="add-card-sheet-body grid gap-4 p-4">
          <label className="toolbar-checkbox flex items-center gap-2 text-[13px]">
            <Checkbox checked={includeCustom} disabled={addLocked} onCheckedChange={(checked) => {
              suppressAutocompleteRef.current = false;
              setIncludeCustom(checked === true);
              setSelectedOption(null);
              setAutocompleteOptions([]);
            }} />
            {ui("Include custom cards")} ({libraryLoading ? ui("Loading...") : customCards.length.toLocaleString()} {ui("available")})
          </label>
          {libraryError && <p role="alert" className="text-[12px] text-amber-300">{libraryError}</p>}
          <div className="relative grid gap-1" ref={autocompleteRef}>
            <label className={labelClass}>{ui("Card Name")}<input
                ref={cardNameInputRef}
                role="combobox"
                aria-autocomplete="list"
                aria-expanded={autocompleteVisible}
                aria-controls={autocompleteVisible ? autocompleteId : undefined}
                aria-activedescendant={autocompleteVisible && autocompleteIndex >= 0 ? `${autocompleteId}-${autocompleteIndex}` : undefined}
                className={inputClass}
                placeholder={ui("Card name")}
                value={cardName}
                disabled={addLocked}
                onChange={(event) => {
                  autocompleteRequestRef.current += 1;
                  setAutocompleteOptions([]);
                  setSelectedOption(null);
                  setCardName(event.target.value);
                  setAutocompleteOpen(true);
                  setAutocompleteIndex(-1);
                }}
                onFocus={() => {
                  if (visibleAutocompleteOptions.length > 0) {
                    setAutocompleteOpen(true);
                  }
                }}
                onKeyDown={(event) => {
                  if (event.key === "ArrowDown" && visibleAutocompleteOptions.length > 0) {
                    event.preventDefault();
                    setAutocompleteOpen(true);
                    setAutocompleteIndex((prev) => (
                      prev >= visibleAutocompleteOptions.length - 1 ? 0 : prev + 1
                    ));
                    return;
                  }

                  if (event.key === "ArrowUp" && visibleAutocompleteOptions.length > 0) {
                    event.preventDefault();
                    setAutocompleteOpen(true);
                    setAutocompleteIndex((prev) => (
                      prev <= 0 ? visibleAutocompleteOptions.length - 1 : prev - 1
                    ));
                    return;
                  }

                  if (event.key === "Escape" && autocompleteVisible) {
                    event.preventDefault();
                    event.stopPropagation();
                    setAutocompleteOpen(false);
                    setAutocompleteIndex(-1);
                    return;
                  }

                  if (event.key === "Enter") {
                    event.preventDefault();
                    if (
                      autocompleteVisible
                      && autocompleteIndex >= 0
                      && visibleAutocompleteOptions[autocompleteIndex]
                    ) {
                      handleAdd(visibleAutocompleteOptions[autocompleteIndex]);
                      return;
                    }
                    if (visibleAutocompleteOptions.length === 1) {
                      handleAdd(visibleAutocompleteOptions[0]);
                      return;
                    }
                    handleAdd();
                  }
                }}
              />
            </label>
            {autocompleteVisible ? (
              <div id={autocompleteId} role="listbox" aria-label={ui("Matching cards")} className="add-card-autocomplete absolute left-0 top-[calc(100%+0.35rem)] z-40 max-h-60 w-full overflow-y-auto p-1">
                {visibleAutocompleteOptions.map((option, index) => (
                  <button
                    key={option.card?.id || `standard:${option.name}`}
                    id={`${autocompleteId}-${index}`}
                    role="option"
                    aria-selected={index === autocompleteIndex}
                    tabIndex={-1}
                    type="button"
                    className={`add-card-autocomplete-option block w-full px-3 py-2 text-left text-[13px] transition-colors ${
                      index === autocompleteIndex ? "is-active font-medium" : ""
                    }`}
                    onMouseEnter={() => setAutocompleteIndex(index)}
                    onClick={() => handleAutocompletePick(option)}
                  >
                    {option.name}{" "}{option.card ? <span className="ml-2 text-[10px] text-muted-foreground">{ui(option.card.savedAt ? "Saved custom" : "Custom")}</span> : null}
                  </button>
                ))}
              </div>
            ) : null}
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <label className={labelClass}>{ui("Player")}<select
                className={selectClass}
                value={selectedPlayer}
                disabled={addLocked}
                onChange={(event) => setPlayerIndex(Number(event.target.value))}
              >
                {players.map((player) => (
                  <option key={player.id} value={player.id}>
                    {player.name}
                  </option>
                ))}
              </select>
            </label>

            <label className={labelClass}>{ui("Zone")}<select
                className={selectClass}
                value={zone}
                disabled={addLocked}
                onChange={(event) => setZone(event.target.value)}
              >
                <option value="hand">{ui("Hand")}</option>
                <option value="battlefield">{ui("Battlefield")}</option>
                <option value="graveyard">{ui("GY")}</option>
                <option value="exile">{ui("Exile")}</option>
                <option value="library">{ui("Library")}</option>
                <option value="command">{ui("Command")}</option>
              </select>
            </label>
          </div>

          <label className="toolbar-checkbox flex items-center gap-2 text-[13px] uppercase tracking-wide">
            <Checkbox
              checked={skipTriggers}
              disabled={addLocked}
              onCheckedChange={(checked) => setSkipTriggers(checked === true)}
              className="h-3.5 w-3.5"
            />{ui("Skip triggers")}</label>

          <div className="add-card-sheet-footer grid gap-2 sm:grid-cols-2">
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="stone-pill"
              onClick={closeSheet}
            >{ui("Cancel")}</Button>
            <Button
              type="button"
              size="sm"
              className="add-card-submit ui-primary-action w-full justify-center uppercase tracking-wide"
              onClick={() => handleAdd(selectedOption || (visibleAutocompleteOptions.length === 1 ? visibleAutocompleteOptions[0] : cardName))}
              disabled={addLocked || !cardName.trim()}
            >{ui("Add to Game")}</Button>
          </div>
        </div>
  );
  if (embedded) return body;

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        {trigger}
      </SheetTrigger>
      <SheetContent
        side="center"
        className={`fantasy-sheet add-card-sheet w-[min(92vw,460px)] p-0 ${triggerClassName}`}
      >
        <SheetHeader className="fantasy-sheet-header pr-12">
          <div className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Tools")}</div>
          <SheetTitle>{ui("Add Card")}</SheetTitle>
          <SheetDescription className="max-w-[34ch] text-[13px] leading-5">{ui("Inject a card directly into a player zone for testing and board setup.")}</SheetDescription>
        </SheetHeader>

        {body}
      </SheetContent>
    </Sheet>
  );
}
