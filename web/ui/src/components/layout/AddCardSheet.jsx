import { useState, useCallback, useEffect, useRef } from "react";
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

function formatPercent(value, digits = 1) {
  const amount = Number(value);
  if (!Number.isFinite(amount)) return null;
  return `${(amount * 100).toFixed(digits)}%`;
}

function formatCardLoadDiagnosticsClipboard(diagnostics, fallbackName, fallbackError, includeDebug = false) {
  const compiledText = Array.isArray(diagnostics?.compiledText) && diagnostics.compiledText.length > 0
    ? diagnostics.compiledText.join("\n")
    : "-";
  const compiledAbilities = Array.isArray(diagnostics?.compiledAbilities) && diagnostics.compiledAbilities.length > 0
    ? diagnostics.compiledAbilities.join("\n")
    : "-";
  const primaryError = diagnostics?.error || fallbackError || null;
  const parseError = diagnostics?.parseError || null;

  return [
    diagnostics?.canonicalName || fallbackName ? `Card: ${diagnostics?.canonicalName || fallbackName}` : "",
    diagnostics?.query ? `Query: ${diagnostics.query}` : "",
    primaryError ? `Error: ${primaryError}` : "",
    parseError && parseError !== primaryError ? `Parse error: ${parseError}` : "",
    includeDebug && formatPercent(diagnostics?.semanticScore)
      ? `Similarity score: ${formatPercent(diagnostics?.semanticScore)}`
      : "",
    Number.isFinite(diagnostics?.thresholdPercent) ? `Threshold: ${diagnostics.thresholdPercent.toFixed(0)}%` : "",
    `Oracle text:\n${diagnostics?.oracleText || "-"}`,
    `Compiled text:\n${compiledText}`,
    `Compiled abilities:\n${compiledAbilities}`,
  ]
    .filter(Boolean)
    .join("\n\n");
}

function buildLowFidelityNotice(diagnostics, fallbackName, zone, includeDebug = false) {
  const semanticScore = Number(diagnostics?.semanticScore);
  const thresholdPercent = Number(diagnostics?.thresholdPercent);
  if (!Number.isFinite(semanticScore) || !Number.isFinite(thresholdPercent)) {
    return null;
  }

  const fidelityPercent = semanticScore * 100;
  if (fidelityPercent >= thresholdPercent) {
    return null;
  }

  const canonicalName = diagnostics?.canonicalName || fallbackName;
  const warningMessage =
    `Added ${canonicalName} to ${zone} at ${fidelityPercent.toFixed(1)}% fidelity, below the current ${thresholdPercent.toFixed(0)}% threshold.`;

  return {
    tone: "warning",
    title: `Added ${canonicalName} below threshold`,
    body: `Fidelity ${fidelityPercent.toFixed(1)}% is below the current ${thresholdPercent.toFixed(0)}% threshold. Click to copy diagnostics.`,
    copyText: formatCardLoadDiagnosticsClipboard(
      diagnostics,
      canonicalName,
      warningMessage,
      includeDebug
    ),
    copyStatusMessage: `Copied diagnostics for ${canonicalName}`,
  };
}

export default function AddCardSheet({
  trigger,
  onAddCardNotice,
  triggerClassName = "",
}) {
  const {
    game,
    state,
    refresh,
    runWasmInteraction,
    setStatus,
    inspectorDebug,
    multiplayer,
  } = useGame();
  const [open, setOpen] = useState(false);
  const [cardName, setCardName] = useState("");
  const [zone, setZone] = useState("hand");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);
  const [autocompleteOptions, setAutocompleteOptions] = useState([]);
  const [autocompleteOpen, setAutocompleteOpen] = useState(false);
  const [autocompleteIndex, setAutocompleteIndex] = useState(-1);
  const autocompleteRef = useRef(null);
  const cardNameInputRef = useRef(null);
  const suppressAutocompleteRef = useRef(false);
  const autocompleteRequestRef = useRef(0);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const selectedPlayer = playerIndex ?? perspective;
  const addLocked = multiplayer.mode !== "idle";

  const visibleAutocompleteOptions =
    addLocked || !cardName.trim() ? [] : autocompleteOptions;
  const autocompleteVisible =
    autocompleteOpen && visibleAutocompleteOptions.length > 0;

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
        const matches = await game.autocompleteCardNames(query, 5);
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
    };
  }, [addLocked, cardName, game]);

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
    setAutocompleteOpen(false);
    setAutocompleteIndex(-1);
  }, []);

  const handleAdd = useCallback(async (requestedName = cardName) => {
    return runWasmInteraction(async () => {
      if (addLocked) {
        setStatus("Card injection is disabled while a lobby is active", true);
        return;
      }
      const name = String(requestedName || "").trim();
      if (!name) {
        setStatus("Enter a card name to add", true);
        return;
      }
      if (!game || typeof game.addCardToZone !== "function") {
        setStatus("This WASM build does not expose addCardToZone", true);
        return;
      }
      try {
        await game.addCardToZone(selectedPlayer, name, zone, skipTriggers);
        let lowFidelityNotice = null;
        if (game && typeof game.cardLoadDiagnostics === "function") {
          try {
            const diagnostics = await game.cardLoadDiagnostics(name);
            lowFidelityNotice = buildLowFidelityNotice(diagnostics, name, zone, inspectorDebug);
          } catch (diagnosticsError) {
            console.warn("cardLoadDiagnostics failed:", diagnosticsError);
          }
        }
        setCardName("");
        setAutocompleteOptions([]);
        setAutocompleteOpen(false);
        setAutocompleteIndex(-1);
        closeSheet();
        await refresh(`Added ${name} to ${zone}`);
        if (lowFidelityNotice && typeof onAddCardNotice === "function") {
          onAddCardNotice(lowFidelityNotice);
        }
      } catch (err) {
        const errMsg = String(err?.message || err);
        setStatus(`Add card failed: ${errMsg}`, true);
        if (typeof onAddCardNotice === "function") {
          let copyText = `Card: ${name}\n\nError: ${errMsg}`;
          if (game && typeof game.cardLoadDiagnostics === "function") {
            try {
              const diagnostics = await game.cardLoadDiagnostics(name, errMsg);
              copyText = formatCardLoadDiagnosticsClipboard(diagnostics, name, errMsg, inspectorDebug);
            } catch (diagnosticsError) {
              console.warn("cardLoadDiagnostics failed:", diagnosticsError);
            }
          }

          onAddCardNotice({
            tone: "error",
            title: `Could not add ${name}`,
            body: `${errMsg} Click to copy diagnostics.`,
            copyText,
            copyStatusMessage: `Copied diagnostics for ${name}`,
          });
        }
      }
    });
  }, [
    addLocked,
    cardName,
    closeSheet,
    game,
    inspectorDebug,
    onAddCardNotice,
    refresh,
    runWasmInteraction,
    selectedPlayer,
    setStatus,
    skipTriggers,
    zone,
  ]);

  const handleAutocompletePick = useCallback((name) => {
    suppressAutocompleteRef.current = true;
    setCardName(name);
    setAutocompleteOptions([]);
    setAutocompleteOpen(false);
    setAutocompleteIndex(-1);
    window.requestAnimationFrame(() => {
      cardNameInputRef.current?.focus();
    });
  }, []);

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
          <div className="text-[11px] uppercase tracking-[0.24em] text-[#cdb27a]">Tools</div>
          <SheetTitle className="text-[22px] uppercase tracking-[0.18em] text-foreground">
            Add Card
          </SheetTitle>
          <SheetDescription className="max-w-[34ch] text-[13px] leading-5">
            Inject a card directly into a player zone for testing and board setup.
          </SheetDescription>
        </SheetHeader>

        <div className="add-card-sheet-body grid gap-4 p-4">
          <div className="relative grid gap-1" ref={autocompleteRef}>
            <label className={labelClass}>
              Card Name
              <input
                ref={cardNameInputRef}
                className={inputClass}
                placeholder="Card name"
                value={cardName}
                disabled={addLocked}
                onChange={(event) => {
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

                  if (event.key === "Escape") {
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
              <div className="add-card-autocomplete absolute left-0 top-[calc(100%+0.35rem)] z-40 w-full overflow-hidden p-1">
                {visibleAutocompleteOptions.map((option, index) => (
                  <button
                    key={option}
                    type="button"
                    className={`add-card-autocomplete-option block w-full px-3 py-2 text-left text-[13px] transition-colors ${
                      index === autocompleteIndex ? "is-active font-medium" : ""
                    }`}
                    onMouseEnter={() => setAutocompleteIndex(index)}
                    onClick={() => handleAutocompletePick(option)}
                  >
                    {option}
                  </button>
                ))}
              </div>
            ) : null}
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <label className={labelClass}>
              Player
              <select
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

            <label className={labelClass}>
              Zone
              <select
                className={selectClass}
                value={zone}
                disabled={addLocked}
                onChange={(event) => setZone(event.target.value)}
              >
                <option value="hand">Hand</option>
                <option value="battlefield">Battlefield</option>
                <option value="graveyard">GY</option>
                <option value="exile">Exile</option>
                <option value="library">Library</option>
                <option value="command">Command</option>
              </select>
            </label>
          </div>

          <label className="toolbar-checkbox flex items-center gap-2 text-[13px] uppercase tracking-wide">
            <Checkbox
              checked={skipTriggers}
              disabled={addLocked}
              onCheckedChange={(checked) => setSkipTriggers(checked === true)}
              className="h-3.5 w-3.5"
            />
            Skip triggers
          </label>

          <div className="add-card-sheet-footer grid gap-2 sm:grid-cols-2">
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="stone-pill"
              onClick={closeSheet}
            >
              Cancel
            </Button>
            <Button
              type="button"
              size="sm"
              className="add-card-submit w-full justify-center uppercase tracking-wide"
              onClick={() => handleAdd()}
              disabled={addLocked}
            >
              Add to Game
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
