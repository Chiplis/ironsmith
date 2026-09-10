import { useCallback, useMemo, useRef, useState } from "react";
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
import {
  CARD_COLORS,
  PERMANENT_TYPES,
  RANDOM_GAME_ZONES,
  SPELL_TYPES,
  cardMatchesFilters,
  createSeededRng,
  generateRandomGamePayload,
  randomGameDefaults,
  zoneAcceptsCard,
} from "@/lib/random-game";
import { collectRandomGameCards, randomGameCardBudget, resolveNamedCards } from "@/lib/random-game-catalog";

const inputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none disabled:cursor-not-allowed disabled:opacity-50";
const numberClass =
  "fantasy-field w-full px-2 py-1 text-[13px] text-foreground outline-none tabular-nums disabled:cursor-not-allowed disabled:opacity-50";
const labelClass =
  "grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground";
const sectionClass = "grid gap-2 border-t border-[rgba(205,180,132,0.16)] pt-3";
const sectionTitleClass = "text-[11px] uppercase tracking-[0.24em] text-[#cdb27a]";
const toggleClass = "toolbar-checkbox flex items-center gap-2 text-[12px] uppercase tracking-wide";

const ZONE_LABELS = {
  battlefield: "Battlefield",
  hand: "Hand",
  library: "Library",
  graveyard: "Graveyard",
  exile: "Exile",
  command: "Command",
};

const CARD_TYPES = [...PERMANENT_TYPES, ...SPELL_TYPES];
const COLOR_KEYS = [...CARD_COLORS, "Colorless"];

function randomSeed() {
  return Math.random().toString(36).slice(2, 10);
}

/**
 * Configure and generate a random table.
 *
 * The generator only ever places a card where it could legally be, so the
 * result is a state the engine can carry on playing from: the battlefield takes
 * permanents, spells stay in the zones that hold cards, and the chosen colours
 * always contribute basic lands so the table can make mana.
 */
export default function RandomGameSheet({ trigger, onGenerate, disabled = false }) {
  const { setStatus } = useGame();
  const [open, setOpen] = useState(false);
  const [config, setConfig] = useState(randomGameDefaults);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState(null);
  const abortRef = useRef(null);

  const patch = useCallback((changes) => {
    setConfig((current) => ({ ...current, ...changes }));
  }, []);
  const patchZone = useCallback((zone, changes) => {
    setConfig((current) => ({
      ...current,
      zones: { ...current.zones, [zone]: { ...current.zones[zone], ...changes } },
    }));
  }, []);

  const budget = useMemo(() => randomGameCardBudget(config), [config]);
  const anyTypeChosen = CARD_TYPES.some((type) => config.types[type]);
  const anyColorChosen = COLOR_KEYS.some((color) => config.colors[color]);
  const permanentChosen = PERMANENT_TYPES.some((type) => config.types[type]);
  const battlefieldCount = Number(config.zones.battlefield.count) || 0;
  const battlefieldBasics = Number(config.zones.battlefield.basics) || 0;
  // The battlefield can only be filled from permanents, so asking for
  // non-basic permanents while every permanent type is off cannot be served.
  const battlefieldImpossible = battlefieldCount > battlefieldBasics && !permanentChosen;
  const blocked = !anyTypeChosen || !anyColorChosen || battlefieldImpossible;

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setBusy(false);
    setProgress(null);
    setOpen(false);
  }, []);

  const handleGenerate = useCallback(async () => {
    if (busy || blocked) return;
    const seed = String(config.seed || "").trim() || randomSeed();
    const controller = new AbortController();
    abortRef.current = controller;
    setBusy(true);
    setProgress({ collected: 0, target: budget });
    try {
      // One shared pool serves every zone and player; the generator decides
      // which of those cards each zone may hold.
      const { cards } = await collectRandomGameCards({
        config,
        rng: createSeededRng(`${seed}:catalog`),
        want: budget,
        accept: (card) => cardMatchesFilters(card, config)
          && RANDOM_GAME_ZONES.some((zone) => zoneAcceptsCard(zone, card)),
        onProgress: setProgress,
        signal: controller.signal,
      });
      if (controller.signal.aborted) return;
      // The cards the local battlefield is promised are read by name, so they
      // are placed whatever the filters would have sampled.
      const guaranteedCards = await resolveNamedCards(config.alwaysOnMyBattlefield);
      if (controller.signal.aborted) return;
      const { payload, shortfalls, eligibleCount, unavailableGuaranteed } = generateRandomGamePayload({
        config,
        cards,
        guaranteedCards,
        rng: createSeededRng(`${seed}:table`),
      });
      if (eligibleCount === 0) {
        setStatus("No cards matched those filters, so no table was generated", true);
        return;
      }
      const generated = await onGenerate?.(payload, `Random game generated (seed ${seed})`);
      if (generated === false) return;
      if (unavailableGuaranteed.length > 0) {
        setStatus(
          `Random game generated without ${unavailableGuaranteed.join(", ")}: not a card that can start on the battlefield`,
          true,
        );
      } else if (shortfalls.length > 0) {
        setStatus(`Random game generated; ${shortfalls.join(", ")} had fewer matching cards than requested`);
      }
      setOpen(false);
    } catch (error) {
      setStatus(`Random game failed: ${error?.message || error}`, true);
    } finally {
      abortRef.current = null;
      setBusy(false);
      setProgress(null);
    }
  }, [blocked, budget, busy, config, onGenerate, setStatus]);

  return (
    <Sheet
      open={open}
      onOpenChange={(next) => {
        if (!next && busy) abortRef.current?.abort();
        setOpen(next);
      }}
    >
      <SheetTrigger asChild>{trigger}</SheetTrigger>
      <SheetContent side="center" className="fantasy-sheet random-game-sheet w-[min(94vw,620px)] p-0">
        <SheetHeader className="fantasy-sheet-header pr-12">
          <div className={sectionTitleClass}>Tools</div>
          <SheetTitle className="text-[22px] uppercase tracking-[0.18em] text-foreground">
            Random Game
          </SheetTitle>
          <SheetDescription className="max-w-[46ch] text-[13px] leading-5">
            Fill every player&apos;s zones with random cards. Only legal placements are
            generated, so spells never start on the battlefield.
          </SheetDescription>
        </SheetHeader>

        <div className="random-game-sheet-body grid max-h-[70vh] gap-4 overflow-y-auto p-4">
          <div className="grid gap-3 sm:grid-cols-3">
            <label className={labelClass}>
              Players
              <input
                type="number"
                min={1}
                max={8}
                className={numberClass}
                value={config.playerCount}
                disabled={busy}
                onChange={(event) => patch({ playerCount: Number(event.target.value) })}
              />
            </label>
            <label className={labelClass}>
              Starting life
              <input
                type="number"
                min={1}
                max={999}
                className={numberClass}
                value={config.startingLife}
                disabled={busy}
                onChange={(event) => patch({ startingLife: Number(event.target.value) })}
              />
            </label>
            <label className={labelClass}>
              Seed
              <span className="flex gap-1">
                <input
                  className={inputClass}
                  placeholder="random"
                  value={config.seed}
                  disabled={busy}
                  onChange={(event) => patch({ seed: event.target.value })}
                />
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  className="stone-pill shrink-0"
                  disabled={busy}
                  onClick={() => patch({ seed: randomSeed() })}
                >
                  Roll
                </Button>
              </span>
            </label>
          </div>

          <div className={sectionClass}>
            <div className={sectionTitleClass}>Cards per zone</div>
            <div className="grid gap-2">
              {RANDOM_GAME_ZONES.map((zone) => (
                <div key={zone} className="grid grid-cols-[1fr_auto_auto] items-center gap-2">
                  <span className="text-[12px] uppercase tracking-wide text-muted-foreground">
                    {ZONE_LABELS[zone]}
                  </span>
                  <label className="flex items-center gap-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    Cards
                    <input
                      type="number"
                      min={0}
                      max={250}
                      className={`${numberClass} w-16`}
                      value={config.zones[zone].count}
                      disabled={busy}
                      onChange={(event) => patchZone(zone, { count: Number(event.target.value) })}
                    />
                  </label>
                  <label className="flex items-center gap-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
                    Basics
                    <input
                      type="number"
                      min={0}
                      max={250}
                      className={`${numberClass} w-16`}
                      value={zone === "command" ? 0 : config.zones[zone].basics}
                      disabled={busy || zone === "command"}
                      onChange={(event) => patchZone(zone, { basics: Number(event.target.value) })}
                    />
                  </label>
                </div>
              ))}
            </div>
            <p className="text-[11px] leading-4 text-muted-foreground">
              Basics come from the colours below, so a generated table can always make mana.
              The command zone only takes legendary creatures and planeswalkers.
            </p>
          </div>

          <div className={sectionClass}>
            <div className={sectionTitleClass}>Always on my battlefield</div>
            <input
              className={inputClass}
              placeholder="Omniscience"
              value={config.alwaysOnMyBattlefield.join(", ")}
              disabled={busy}
              onChange={(event) => patch({
                alwaysOnMyBattlefield: event.target.value.split(",").map((name) => name.trim()).filter(Boolean),
              })}
            />
            <p className="text-[11px] leading-4 text-muted-foreground">
              These cards always start on your own battlefield, whatever the filters below
              allow. They take battlefield slots, so the count above still holds.
            </p>
          </div>

          <div className={sectionClass}>
            <div className={sectionTitleClass}>Card types</div>
            <div className="flex flex-wrap gap-x-4 gap-y-1">
              {CARD_TYPES.map((type) => (
                <label key={type} className={toggleClass}>
                  <Checkbox
                    checked={Boolean(config.types[type])}
                    disabled={busy}
                    className="h-3.5 w-3.5"
                    onCheckedChange={(checked) => patch({
                      types: { ...config.types, [type]: checked === true },
                    })}
                  />
                  {type}
                </label>
              ))}
            </div>
          </div>

          <div className={sectionClass}>
            <div className={sectionTitleClass}>Colours</div>
            <div className="flex flex-wrap gap-x-4 gap-y-1">
              {COLOR_KEYS.map((color) => (
                <label key={color} className={toggleClass}>
                  <Checkbox
                    checked={Boolean(config.colors[color])}
                    disabled={busy}
                    className="h-3.5 w-3.5"
                    onCheckedChange={(checked) => patch({
                      colors: { ...config.colors, [color]: checked === true },
                    })}
                  />
                  {color}
                </label>
              ))}
            </div>
          </div>

          <div className={sectionClass}>
            <div className={sectionTitleClass}>Limits</div>
            <div className="grid gap-3 sm:grid-cols-3">
              <label className={labelClass}>
                Min mana value
                <input
                  type="number"
                  min={0}
                  max={20}
                  className={numberClass}
                  value={config.manaValue.min}
                  disabled={busy}
                  onChange={(event) => patch({
                    manaValue: { ...config.manaValue, min: Number(event.target.value) },
                  })}
                />
              </label>
              <label className={labelClass}>
                Max mana value
                <input
                  type="number"
                  min={0}
                  max={20}
                  className={numberClass}
                  value={config.manaValue.max}
                  disabled={busy}
                  onChange={(event) => patch({
                    manaValue: { ...config.manaValue, max: Number(event.target.value) },
                  })}
                />
              </label>
              <label className={labelClass}>
                Min fidelity
                <input
                  type="number"
                  min={0}
                  max={1}
                  step={0.01}
                  className={numberClass}
                  value={config.minScore}
                  disabled={busy}
                  onChange={(event) => patch({ minScore: Number(event.target.value) })}
                />
              </label>
            </div>
            <div className="flex flex-wrap gap-x-4 gap-y-1">
              <label className={toggleClass}>
                <Checkbox
                  checked={config.singleFacedOnly}
                  disabled={busy}
                  className="h-3.5 w-3.5"
                  onCheckedChange={(checked) => patch({ singleFacedOnly: checked === true })}
                />
                Single-faced only
              </label>
              <label className={toggleClass}>
                <Checkbox
                  checked={config.allowDuplicates}
                  disabled={busy}
                  className="h-3.5 w-3.5"
                  onCheckedChange={(checked) => patch({ allowDuplicates: checked === true })}
                />
                Allow duplicates
              </label>
              <label className={toggleClass}>
                <Checkbox
                  checked={config.allowDuplicateLegends}
                  disabled={busy || !config.allowDuplicates}
                  className="h-3.5 w-3.5"
                  onCheckedChange={(checked) => patch({ allowDuplicateLegends: checked === true })}
                />
                Duplicate legends on board
              </label>
            </div>
            <p className="text-[11px] leading-4 text-muted-foreground">
              Fidelity is the compiled-text similarity score; 1 keeps only cards the engine
              reproduces exactly. Lower it for a wider pool.
            </p>
          </div>

          {blocked ? (
            <p className="text-[12px] leading-4 text-[#f0a9a0]">
              {!anyTypeChosen
                ? "Pick at least one card type."
                : !anyColorChosen
                  ? "Pick at least one colour."
                  : "The battlefield can only hold permanents: pick a permanent type or fill it with basics."}
            </p>
          ) : null}

          <div className="random-game-sheet-footer grid gap-2 sm:grid-cols-2">
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="stone-pill"
              onClick={cancel}
            >
              {busy ? "Stop" : "Cancel"}
            </Button>
            <Button
              type="button"
              size="sm"
              className="random-game-submit ui-primary-action w-full justify-center uppercase tracking-wide"
              onClick={handleGenerate}
              disabled={disabled || busy || blocked}
            >
              {busy
                ? `Collecting ${progress?.collected ?? 0}/${progress?.target ?? budget}`
                : "Generate"}
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
