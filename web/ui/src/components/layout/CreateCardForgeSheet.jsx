import { cloneElement, isValidElement, useCallback, useEffect, useMemo, useState } from "react";
import { RefreshCw, Sparkles, SquareSplitHorizontal, Layers3 } from "lucide-react";

import { useGame } from "@/context/GameContext";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const SUPER_TYPES = ["Legendary", "Basic", "Snow", "World"];
const CARD_TYPES = [
  "Artifact",
  "Battle",
  "Creature",
  "Enchantment",
  "Instant",
  "Kindred",
  "Land",
  "Planeswalker",
  "Sorcery",
];
const COLOR_CODES = ["W", "U", "B", "R", "G"];
const ZONE_OPTIONS = [
  ["battlefield", "Battlefield"],
  ["hand", "Hand"],
  ["graveyard", "Graveyard"],
  ["exile", "Exile"],
  ["library", "Library"],
  ["command", "Command"],
];
const LAYOUT_OPTIONS = [
  { value: "single", label: "Single", icon: Sparkles },
  { value: "transform_like", label: "Double-Faced", icon: Layers3 },
  { value: "split", label: "Split", icon: SquareSplitHorizontal },
];

function blankFace(label = "Custom Card") {
  return {
    name: label,
    manaCost: "",
    colorIndicator: [],
    supertypes: [],
    cardTypes: ["Creature"],
    subtypes: [],
    oracleText: "",
    power: "2",
    toughness: "2",
    loyalty: "",
    defense: "",
  };
}

function blankDraft() {
  return {
    layout: "single",
    hasFuse: false,
    faces: [blankFace()],
  };
}

function cloneDraft(draft) {
  return {
    layout: draft?.layout || "single",
    hasFuse: draft?.hasFuse === true,
    faces: Array.isArray(draft?.faces)
      ? draft.faces.map((face) => ({
        name: face?.name || "",
        manaCost: face?.manaCost || "",
        colorIndicator: Array.isArray(face?.colorIndicator) ? [...face.colorIndicator] : [],
        supertypes: Array.isArray(face?.supertypes) ? [...face.supertypes] : [],
        cardTypes: Array.isArray(face?.cardTypes) ? [...face.cardTypes] : [],
        subtypes: Array.isArray(face?.subtypes) ? [...face.subtypes] : [],
        oracleText: face?.oracleText || "",
        power: face?.power || "",
        toughness: face?.toughness || "",
        loyalty: face?.loyalty != null ? String(face.loyalty) : "",
        defense: face?.defense != null ? String(face.defense) : "",
      }))
      : [blankFace()],
  };
}

function normalizeDraftForApi(draft) {
  return {
    layout: draft.layout,
    hasFuse: draft.hasFuse,
    faces: draft.faces.map((face) => ({
      name: String(face.name || "").trim(),
      manaCost: String(face.manaCost || "").trim() || null,
      colorIndicator: face.colorIndicator,
      supertypes: face.supertypes,
      cardTypes: face.cardTypes,
      subtypes: face.subtypes,
      oracleText: String(face.oracleText || ""),
      power: String(face.power || "").trim() || null,
      toughness: String(face.toughness || "").trim() || null,
      loyalty: face.loyalty === "" ? null : Number(face.loyalty),
      defense: face.defense === "" ? null : Number(face.defense),
    })),
  };
}

function faceTabLabel(layout, index) {
  if (layout === "split") return index === 0 ? "Left Half" : "Right Half";
  if (layout === "transform_like") return index === 0 ? "Front Face" : "Back Face";
  return "Card";
}

function joinedSubtypes(face) {
  return Array.isArray(face?.subtypes) ? face.subtypes.join(", ") : "";
}

function setFromCsv(raw) {
  return raw
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

function toggleChoice(values, item) {
  return values.includes(item)
    ? values.filter((value) => value !== item)
    : [...values, item];
}

function LayoutPicker({ value, onChange }) {
  return (
    <div className="grid gap-2 sm:grid-cols-3">
      {LAYOUT_OPTIONS.map((option) => {
        const Icon = option.icon;
        const active = value === option.value;
        return (
          <button
            key={option.value}
            type="button"
            className={cn("card-forge-choice", active && "is-active")}
            onClick={() => onChange(option.value)}
          >
            <span className="flex items-center gap-2">
              <Icon className="size-4" />
              {option.label}
            </span>
          </button>
        );
      })}
    </div>
  );
}

function ToggleChipGroup({ values, options, onToggle }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {options.map((option) => (
        <button
          key={option}
          type="button"
          className={cn("card-forge-chip", values.includes(option) && "is-active")}
          onClick={() => onToggle(option)}
        >
          {option}
        </button>
      ))}
    </div>
  );
}

function ColorToggleGroup({ values, onToggle }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {COLOR_CODES.map((color) => (
        <button
          key={color}
          type="button"
          className={cn("card-forge-color", values.includes(color) && "is-active")}
          onClick={() => onToggle(color)}
        >
          {color}
        </button>
      ))}
    </div>
  );
}

function PreviewCard({ face, layout, hasFuse, busy, error }) {
  const statLabel = face?.power && face?.toughness
    ? `${face.power}/${face.toughness}`
    : face?.loyalty != null
      ? `L${face.loyalty}`
      : face?.defense != null
        ? `D${face.defense}`
        : null;

  return (
    <section className="card-forge-preview-card">
      <div className="card-forge-preview-aurora" />
      <div className="card-forge-preview-frame">
        <div className="card-forge-preview-header">
          <div className="min-w-0">
            <div className="card-forge-preview-name">{face?.name || "Custom Card"}</div>
            <div className="card-forge-preview-layout">
              {layout === "single" ? "Single-face" : layout === "split" ? "Split" : "Double-faced"}
              {hasFuse ? " • Fuse" : ""}
            </div>
          </div>
          <div className="card-forge-preview-cost">{face?.manaCost || "No cost"}</div>
        </div>

        <div className="card-forge-preview-type">{face?.typeLine || "Type line pending"}</div>

        <div className="card-forge-preview-text">
          {busy ? "Compiling preview..." : error ? error : face?.oracleText || "Rules text preview"}
        </div>

        <div className="card-forge-preview-footer">
          <div className="card-forge-preview-colors">
            {(face?.colorIndicator?.length || 0) > 0 ? face.colorIndicator.join(" ") : "No indicator"}
          </div>
          <div className="card-forge-preview-stat">{statLabel || "-"}</div>
        </div>
      </div>
    </section>
  );
}

function CompilePanel({ face, previewError, busy }) {
  if (previewError) {
    return (
      <section className="card-forge-panel">
        <div className="card-forge-panel-title">Compile Status</div>
        <div className="card-forge-status card-forge-status--error">{previewError}</div>
      </section>
    );
  }

  if (busy && !face) {
    return (
      <section className="card-forge-panel">
        <div className="card-forge-panel-title">Compile Status</div>
        <div className="card-forge-status">Preparing preview...</div>
      </section>
    );
  }

  return (
    <section className="card-forge-panel">
      <div className="card-forge-panel-title">Compile Status</div>
      <div className="card-forge-status card-forge-status--good">
        Ready
        {busy ? " • refreshing" : ""}
      </div>
      <div className="grid gap-3 lg:grid-cols-2">
        <div className="grid gap-1.5">
          <div className="card-forge-section-label">Compiled Text</div>
          <div className="card-forge-codeblock">
            {(face?.compiledText?.length || 0) > 0 ? face.compiledText.join("\n") : "No compiled spell text"}
          </div>
        </div>
        <div className="grid gap-1.5">
          <div className="card-forge-section-label">Compiled Abilities</div>
          <div className="card-forge-codeblock">
            {(face?.compiledAbilities?.length || 0) > 0
              ? face.compiledAbilities.join("\n")
              : "No compiled abilities"}
          </div>
        </div>
      </div>
    </section>
  );
}

export default function CreateCardForgeSheet({
  disabled = false,
  players = [],
  selectedPlayer = 0,
  onSelectPlayer,
  zone = "battlefield",
  onZoneChange,
  skipTriggers = false,
  onSkipTriggersChange,
  trigger = null,
}) {
  const { game, refresh, runWasmInteraction, setStatus } = useGame();
  const [open, setOpen] = useState(false);
  const [seedLoading, setSeedLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState("");
  const [seedDraft, setSeedDraft] = useState(null);
  const [draft, setDraft] = useState(blankDraft());
  const [preview, setPreview] = useState(null);
  const [activeFace, setActiveFace] = useState("face-0");

  const activeFaceIndex = Number(activeFace.replace("face-", "")) || 0;
  const previewFace = preview?.faces?.[activeFaceIndex] || null;
  const primaryName = preview?.faces?.[0]?.name || draft.faces[0]?.name || "Custom Card";
  const canCreate = !disabled && !submitting && Boolean(preview?.canCreate) && !previewError;

  useEffect(() => {
    if (activeFaceIndex >= draft.faces.length) {
      setActiveFace("face-0");
    }
  }, [activeFaceIndex, draft.faces.length]);

  useEffect(() => {
    if (!open || !game) return undefined;

    const timeoutId = window.setTimeout(async () => {
      setPreviewLoading(true);
      try {
        const nextPreview = await game.previewCustomCard(normalizeDraftForApi(draft));
        setPreview(nextPreview);
        setPreviewError("");
      } catch (error) {
        setPreview(null);
        setPreviewError(String(error?.message || error));
      } finally {
        setPreviewLoading(false);
      }
    }, 180);

    return () => window.clearTimeout(timeoutId);
  }, [draft, game, open]);

  const loadSeed = useCallback(async ({ reroll = false } = {}) => {
    setSeedLoading(true);
    try {
      if (!game || typeof game.sampleLoadedDeckSeed !== "function") {
        throw new Error("This WASM build does not expose sampleLoadedDeckSeed");
      }
      const seed = cloneDraft(await game.sampleLoadedDeckSeed(selectedPlayer));
      setSeedDraft(seed);
      setDraft(seed);
      setActiveFace("face-0");
      setPreviewError("");
      if (reroll) {
        setStatus(`Seeded forge with ${seed.faces?.[0]?.name || "a deck card"}`);
      }
    } catch {
      const fallback = blankDraft();
      setSeedDraft(fallback);
      setDraft(fallback);
      setActiveFace("face-0");
      setPreview(null);
      setPreviewError("");
      setStatus(
        `No loaded deck seed available for ${players.find((player) => player.id === selectedPlayer)?.name || "that player"}; starting from a blank card`
      );
    } finally {
      setSeedLoading(false);
    }
  }, [game, players, selectedPlayer, setStatus]);

  const handleOpenChange = useCallback((nextOpen) => {
    setOpen(nextOpen);
    if (nextOpen) {
      void loadSeed();
    }
  }, [loadSeed]);

  const updateFace = useCallback((index, patch) => {
    setDraft((current) => ({
      ...current,
      faces: current.faces.map((face, faceIndex) => (
        faceIndex === index ? { ...face, ...patch } : face
      )),
    }));
  }, []);

  const toggleFaceArrayValue = useCallback((index, key, value) => {
    setDraft((current) => ({
      ...current,
      faces: current.faces.map((face, faceIndex) => (
        faceIndex === index
          ? { ...face, [key]: toggleChoice(face[key], value) }
          : face
      )),
    }));
  }, []);

  const handleLayoutChange = useCallback((nextLayout) => {
    setDraft((current) => {
      const nextFaces = [...current.faces];
      if (nextLayout === "single") {
        return {
          layout: nextLayout,
          hasFuse: false,
          faces: [nextFaces[0] || blankFace()],
        };
      }
      while (nextFaces.length < 2) {
        nextFaces.push(blankFace(nextLayout === "split" ? "Right Half" : "Back Face"));
      }
      return {
        layout: nextLayout,
        hasFuse: nextLayout === "split" ? current.hasFuse : false,
        faces: nextFaces.slice(0, 2),
      };
    });
    setActiveFace("face-0");
  }, []);

  const resetToSeed = useCallback(() => {
    if (!seedDraft) return;
    setDraft(cloneDraft(seedDraft));
    setActiveFace("face-0");
  }, [seedDraft]);

  const handleCreate = useCallback(async () => {
    return runWasmInteraction(async () => {
      if (!game || typeof game.createCustomCard !== "function") {
        setStatus("This WASM build does not expose createCustomCard", true);
        return;
      }
      setSubmitting(true);
      try {
        await game.createCustomCard({
          draft: normalizeDraftForApi(draft),
          playerIndex: selectedPlayer,
          zoneName: zone,
          skipTriggers,
        });
        setOpen(false);
        await refresh(`Created ${primaryName}`);
      } catch (error) {
        setStatus(`Create card failed: ${String(error?.message || error)}`, true);
      } finally {
        setSubmitting(false);
      }
    });
  }, [draft, game, primaryName, refresh, runWasmInteraction, selectedPlayer, setStatus, skipTriggers, zone]);

  const faceTabs = useMemo(() => (
    draft.faces.map((face, index) => ({
      value: `face-${index}`,
      title: faceTabLabel(draft.layout, index),
      subtitle: face.name || faceTabLabel(draft.layout, index),
    }))
  ), [draft.faces, draft.layout]);
  const triggerNode = trigger && isValidElement(trigger)
    ? cloneElement(trigger, {
      disabled: trigger.props.disabled || disabled,
      onClick: (event) => {
        trigger.props.onClick?.(event);
        if (!event.defaultPrevented) {
          handleOpenChange(true);
        }
      },
    })
    : null;

  return (
    <>
      {triggerNode ?? (
        <button
          type="button"
          className="stone-pill inline-flex items-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45"
          disabled={disabled}
          onClick={() => handleOpenChange(true)}
        >
          Create Card
        </button>
      )}

      <Sheet open={open} onOpenChange={handleOpenChange}>
        <SheetContent
          side="center"
          className="card-forge-sheet fantasy-sheet max-h-[92vh] overflow-hidden p-0"
        >
          <SheetHeader className="fantasy-sheet-header card-forge-header pr-12">
            <div className="card-forge-eyebrow">Forge</div>
            <SheetTitle className="text-[24px] uppercase tracking-[0.18em] text-foreground">
              Create Card
            </SheetTitle>
            <SheetDescription className="card-forge-description max-w-[58ch] text-[13px] leading-5">
              Seeded from a random nonland card in the loaded deck. The sample is only a teaching
              aid, and every printed characteristic can be rewritten before the card enters this
              goldfishing session.
            </SheetDescription>
          </SheetHeader>

          <div className="card-forge-toolbar">
            <div className="card-forge-banner">
              {seedLoading ? "Loading deck sample..." : `Seed example: ${seedDraft?.faces?.[0]?.name || "Blank custom card"}`}
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={seedLoading}
                onClick={() => void runWasmInteraction(() => loadSeed({ reroll: true }))}
              >
                <RefreshCw className={cn("size-3.5", seedLoading && "animate-spin")} />
                New Sample
              </Button>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={!seedDraft}
                onClick={resetToSeed}
              >
                Reset Seed
              </Button>
            </div>
          </div>

          <div className="card-forge-grid">
            <div className="card-forge-left">
              <PreviewCard
                face={previewFace}
                layout={draft.layout}
                hasFuse={draft.hasFuse}
                busy={previewLoading}
                error={previewError}
              />
              <CompilePanel face={previewFace} previewError={previewError} busy={previewLoading} />
            </div>

            <div className="card-forge-right">
              <section className="card-forge-panel">
                <div className="card-forge-panel-title">Card Layout</div>
                <LayoutPicker value={draft.layout} onChange={handleLayoutChange} />
                {draft.layout === "split" ? (
                  <label className="toolbar-checkbox mt-2 flex items-center gap-2 text-[13px] uppercase tracking-wide">
                    <Checkbox
                      checked={draft.hasFuse}
                      onCheckedChange={(checked) => {
                        setDraft((current) => ({ ...current, hasFuse: checked === true }));
                      }}
                      className="h-3.5 w-3.5"
                    />
                    Fuse enabled
                  </label>
                ) : null}
              </section>

              <Tabs value={activeFace} onValueChange={setActiveFace}>
                <TabsList variant="line" className="card-forge-tabs-list">
                  {faceTabs.map((tab) => (
                    <TabsTrigger
                      key={tab.value}
                      value={tab.value}
                      className="card-forge-tab-trigger"
                    >
                      <span className="grid text-left">
                        <span>{tab.title}</span>
                        <span className="card-forge-tab-subtitle text-[10px] uppercase tracking-[0.2em]">
                          {tab.subtitle}
                        </span>
                      </span>
                    </TabsTrigger>
                  ))}
                </TabsList>

                {draft.faces.map((face, index) => (
                  <TabsContent key={`face-panel-${index}`} value={`face-${index}`}>
                    <section className="card-forge-panel">
                      <div className="card-forge-fields">
                        <label className="card-forge-field">
                          <span className="card-forge-section-label">Name</span>
                          <input
                            className="card-forge-input"
                            value={face.name}
                            onChange={(event) => updateFace(index, { name: event.target.value })}
                          />
                        </label>

                        <label className="card-forge-field">
                          <span className="card-forge-section-label">Mana Cost</span>
                          <input
                            className="card-forge-input"
                            placeholder="{2}{W}{U}"
                            value={face.manaCost}
                            onChange={(event) => updateFace(index, { manaCost: event.target.value })}
                          />
                        </label>

                        <div className="card-forge-field">
                          <span className="card-forge-section-label">Color Indicator</span>
                          <ColorToggleGroup
                            values={face.colorIndicator}
                            onToggle={(value) => toggleFaceArrayValue(index, "colorIndicator", value)}
                          />
                        </div>

                        <div className="card-forge-field">
                          <span className="card-forge-section-label">Supertypes</span>
                          <ToggleChipGroup
                            values={face.supertypes}
                            options={SUPER_TYPES}
                            onToggle={(value) => toggleFaceArrayValue(index, "supertypes", value)}
                          />
                        </div>

                        <div className="card-forge-field">
                          <span className="card-forge-section-label">Card Types</span>
                          <ToggleChipGroup
                            values={face.cardTypes}
                            options={CARD_TYPES}
                            onToggle={(value) => toggleFaceArrayValue(index, "cardTypes", value)}
                          />
                        </div>

                        <label className="card-forge-field card-forge-field--full">
                          <span className="card-forge-section-label">Subtypes</span>
                          <input
                            className="card-forge-input"
                            placeholder="Wizard, Human"
                            value={joinedSubtypes(face)}
                            onChange={(event) => updateFace(index, { subtypes: setFromCsv(event.target.value) })}
                          />
                        </label>

                        <div className="grid gap-3 md:grid-cols-4">
                          <label className="card-forge-field">
                            <span className="card-forge-section-label">Power</span>
                            <input
                              className="card-forge-input"
                              placeholder="2 or *"
                              value={face.power}
                              onChange={(event) => updateFace(index, { power: event.target.value })}
                            />
                          </label>
                          <label className="card-forge-field">
                            <span className="card-forge-section-label">Toughness</span>
                            <input
                              className="card-forge-input"
                              placeholder="2 or *+1"
                              value={face.toughness}
                              onChange={(event) => updateFace(index, { toughness: event.target.value })}
                            />
                          </label>
                          <label className="card-forge-field">
                            <span className="card-forge-section-label">Loyalty</span>
                            <input
                              className="card-forge-input"
                              type="number"
                              min={0}
                              value={face.loyalty}
                              onChange={(event) => updateFace(index, { loyalty: event.target.value })}
                            />
                          </label>
                          <label className="card-forge-field">
                            <span className="card-forge-section-label">Defense</span>
                            <input
                              className="card-forge-input"
                              type="number"
                              min={0}
                              value={face.defense}
                              onChange={(event) => updateFace(index, { defense: event.target.value })}
                            />
                          </label>
                        </div>

                        <label className="card-forge-field card-forge-field--full">
                          <span className="card-forge-section-label">Rules Text</span>
                          <textarea
                            className="card-forge-textarea"
                            placeholder="Write oracle-style rules text here..."
                            value={face.oracleText}
                            onChange={(event) => updateFace(index, { oracleText: event.target.value })}
                          />
                        </label>
                      </div>
                    </section>
                  </TabsContent>
                ))}
              </Tabs>

              <section className="card-forge-panel">
                <div className="card-forge-panel-title">Placement</div>
                <div className="grid gap-3 md:grid-cols-2">
                  <label className="card-forge-field">
                    <span className="card-forge-section-label">Player</span>
                    <select
                      className="card-forge-input"
                      value={selectedPlayer}
                      onChange={(event) => onSelectPlayer?.(Number(event.target.value))}
                    >
                      {players.map((player) => (
                        <option key={player.id} value={player.id}>
                          {player.name}
                        </option>
                      ))}
                    </select>
                  </label>

                  <label className="card-forge-field">
                    <span className="card-forge-section-label">Zone</span>
                    <select
                      className="card-forge-input"
                      value={zone}
                      onChange={(event) => onZoneChange?.(event.target.value)}
                    >
                      {ZONE_OPTIONS.map(([value, label]) => (
                        <option key={value} value={value}>
                          {label}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>

                <label className="toolbar-checkbox mt-2 flex items-center gap-2 text-[13px] uppercase tracking-wide">
                  <Checkbox
                    checked={skipTriggers}
                    onCheckedChange={(checked) => onSkipTriggersChange?.(checked === true)}
                    className="h-3.5 w-3.5"
                  />
                  Skip triggers
                </label>
              </section>
            </div>
          </div>

          <div className="card-forge-footer">
            <div className="card-forge-footer-note">
              Live preview is compiled by the engine, so the created card uses the same runtime path
              as built-in cards.
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                onClick={() => setOpen(false)}
              >
                Cancel
              </Button>
              <Button
                type="button"
                size="sm"
                className="card-forge-submit stone-pill"
                disabled={!canCreate}
                onClick={() => void handleCreate()}
              >
                {submitting ? "Creating..." : `Create ${primaryName}`}
              </Button>
            </div>
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}
