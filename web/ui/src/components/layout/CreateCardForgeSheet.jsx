import CommunityFormatEditor from './CommunityFormatEditor';
import { emptyCommunityFormat } from '@/lib/community-format';
import { normalizeDraftForApi } from "@/lib/custom-card-draft";
import SandboxPanel from "./SandboxPanel";
import MechanicsPanel from "./MechanicsPanel";
import CustomCardArtEditor from "./CustomCardArtEditor";
import BuildCardSearch from "./BuildCardSearch";
import CardPrintingStage from "./CardPrintingStage";
import {
  isDefaultCardPresentation,
  normalizeCardPresentation,
  setCardPresentation,
  snapshotStableIds,
} from "@/lib/card-glint/printing-choices.mjs";
import CustomCardsButton from "./CustomCardsButton";
import { customCardImageUrl } from "@/lib/custom-card-assets";
import { tokenKey } from "@/lib/custom-tokens";
import { CUSTOM_TOKENS_CHANGED, deleteCustomToken, listCustomTokens, saveCustomCard, saveCustomToken, previewCustomDraft, prepareCustomDraftForCreation } from "@/lib/custom-card-library";
import useUiText from "@/i18n/useUiText";
import { cloneElement, isValidElement, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CircleHelp, Copy, Loader2, RefreshCw, Sparkles, SquareSplitHorizontal, Layers3 } from "lucide-react";

import { useGame } from "@/context/GameContext";
import { cn } from "@/lib/utils";
import { copyTextToClipboard } from "@/lib/clipboard";
import { hideEmptyDefinitionFields } from "@/lib/compiled-definition-display";
import {
  fetchCardBuildDraft,
  customCardArtUrl,
  resolveScryfallImageUrl,
  resolveScryfallPrintingMetadata,
  setCustomCardCounterOverrides,
  setCompiledCardNames,
  setCustomCardArtUrls,
} from "@/lib/scryfall";
import { counterDisplayLabel } from "@/lib/mana-assets";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Slider } from "@/components/ui/slider";
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
  { value: "meld", label: "Meld", icon: Layers3 },
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
    artUrl: "",
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
    faces: [{ ...blankFace(), manaCost: "{0}" }],
    presentation: normalizeCardPresentation(),
  };
}

// Printing choices alone never turn a catalog card into a custom compile.
function rulesSnapshot(draft) {
  return JSON.stringify({ ...draft, presentation: undefined });
}

// Keep this list aligned with the engine's CounterType enum. The showcase
// uses direct counter seeding, so it can inspect every supported badge without
// changing card rules or requiring a bespoke oracle-text sentence per type.
const COUNTER_SHOWCASE_COUNTER_KINDS = [
  "Plus One Plus One", "Minus One Minus One", "+1/+0", "+0/+1", "+1/+2", "+2/+2",
  "-0/-1", "-0/-2", "-2/-1", "-2/-2", "Deathtouch", "Decayed", "Double Strike",
  "First Strike", "Flying", "Haste", "Hexproof", "Indestructible", "Lifelink", "Menace",
  "Reach", "Trample", "Vigilance", "Loyalty", "Charge", "Age", "Aim", "Arrow",
  "Awakening", "Blood", "Brain", "Bounty", "Brick", "Corpse", "Credit", "Crystal",
  "Cube", "Currency", "Death", "Defense", "Depletion", "Despair", "Devotion", "Divinity",
  "Doom", "Dream", "Echo", "Egg", "Energy", "Enlightened", "Eon", "Experience",
  "Eyeball", "Fade", "Fate", "Feather", "Filibuster", "Finality", "Flame", "Flood",
  "Foreshadow", "Fungus", "Fuse", "Gem", "Glyph", "Gold", "Growth", "Hatchling",
  "Healing", "Hit", "Hoofprint", "Hour", "Hunger", "Ice", "Incarnation", "Infection",
  "Intervention", "Isolation", "Javelin", "Ki", "Keyword", "Knowledge", "Level", "Lore",
  "Luck", "Magnet", "Manifestation", "Mannequin", "Matrix", "Mine", "Mining", "Mire",
  "Music", "Muster", "Net", "Night", "Oil", "Omen", "Ore", "Page", "Pain",
  "Paralyzation", "Petal", "Petrification", "Phylactery", "Pin", "Plague", "Plot", "Polyp",
  "Poison", "Pressure", "Prey", "Pupa", "Quest", "Rad", "Scream", "Shield", "Silver",
  "Sleep", "Slime", "Slumber", "Soot", "Soul", "Spore", "Storage", "Strife", "Study",
  "Stun", "Void", "Task", "Theft", "Tide", "Time", "Tower", "Training", "Trap",
  "Treasure", "Unity", "Velocity", "Verse", "Vitality", "Volatile", "Voyage", "Wage",
  "Winch", "Wind", "Wish",
];

function counterShowcaseLabel(kind) {
  return counterDisplayLabel(kind)
    || String(kind).replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
}

const COUNTER_SHOWCASE_PRESETS = COUNTER_SHOWCASE_COUNTER_KINDS.map((kind, index) => {
  const label = counterShowcaseLabel(kind);
  const slug = label.replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "").toLowerCase();
  return {
    id: `counter-showcase-${index}-${slug}`,
    label,
    counter: { kind, amount: 1 },
    name: `Counter Showcase — ${label}`,
    cardTypes: ["Artifact"],
    oracleText: "",
  };
});

// A presentation-only mixed stack exercises the UI aggregation path without
// changing engine rules. The engine seeds the first source so the card is a
// real compiled object; the UI override supplies the complete source list for
// this visual lab card (4 × +1/+1, 1 × +1/+0, 1 × -1/-1 => net +4/+3).
const COUNTER_SHOWCASE_MIXED_PRESET = {
  id: "counter-showcase-mixed-power-toughness",
  label: "Mixed P/T (+4/+3)",
  counter: { kind: "Plus One Plus One", amount: 4 },
  overrideCounters: [
    { kind: "Plus One Plus One", amount: 4 },
    { kind: "+1/+0", amount: 1 },
    { kind: "Minus One Minus One", amount: 1 },
  ],
  name: "Mixed P/T Counters +4/+3",
  cardTypes: ["Artifact"],
  // Keep the engine-facing text empty: this card is a visual fixture and the
  // descriptive breakdown lives in the UI-only override below.
  oracleText: "",
};

const COUNTER_SHOWCASE_THREE_THREE_PRESET = {
  id: "counter-showcase-net-three-three",
  label: "Net P/T (+3/3)",
  counter: { kind: "Minus One Minus One", amount: 1 },
  overrideCounters: [
    { kind: "Minus One Minus One", amount: 1 },
    { kind: "+2/+2", amount: 2 },
  ],
  name: "Combined P/T Counters +3/+3",
  cardTypes: ["Artifact"],
  oracleText: "",
};

const COUNTER_SHOWCASE_POSITIVE_STACK_PRESET = {
  id: "counter-showcase-positive-stack",
  label: "Positive P/T (+5/5)",
  counter: { kind: "Plus One Plus One", amount: 1 },
  overrideCounters: [
    { kind: "Plus One Plus One", amount: 1 },
    { kind: "+2/+2", amount: 2 },
  ],
  name: "Stacked P/T Counters +5/+5",
  cardTypes: ["Artifact"],
  oracleText: "",
};

// A real creature fixture makes it possible to verify the complete battlefield
// treatment in one click: the card is printed 0/0, enters with a fractional
// +1/+1 counter, and therefore renders its base attack/defense beside the
// separate counter chip instead of misleadingly looking like a 1/1 base card.
// The counter is seeded directly by the counter showcase path so this remains a
// deterministic visual test even when trigger execution is disabled.
const COUNTER_SHOWCASE_CREATURE_PRESET = {
  id: "counter-showcase-creature-plus-one-plus-one",
  label: "Creature 0/0 +1/+1 (1/1)",
  counter: { kind: "Plus One Plus One", amount: 1 },
  name: "Creature Base 0/0 with +1/+1 Counter",
  cardTypes: ["Creature"],
  subtypes: ["Mutant"],
  oracleText: "This creature enters with a +1/+1 counter on it.",
  power: "0",
  toughness: "0",
};

const COUNTER_SHOWCASE_PRESET_LIST = [
  COUNTER_SHOWCASE_CREATURE_PRESET,
  ...COUNTER_SHOWCASE_PRESETS,
  COUNTER_SHOWCASE_MIXED_PRESET,
  COUNTER_SHOWCASE_THREE_THREE_PRESET,
  COUNTER_SHOWCASE_POSITIVE_STACK_PRESET,
];

function counterShowcaseDraft(preset) {
  const face = blankFace(preset.name);
  return {
    layout: "single",
    hasFuse: false,
    faces: [{
      ...face,
      name: preset.name,
      cardTypes: [...preset.cardTypes],
      subtypes: [...(preset.subtypes || [])],
      oracleText: preset.oracleText,
      power: preset.power || "",
      toughness: preset.toughness || "",
    }],
  };
}

function cloneDraft(draft) {
  return {
    customMechanics: structuredClone(draft?.customMechanics || []),
    ...(draft?.customTokens?.length ? { customTokens: structuredClone(draft.customTokens) } : {}),
    ...(draft?.token === true ? { token: true } : {}),
    layout: draft?.layout || "single",
    frameLayout: draft?.frameLayout,
    hasFuse: draft?.hasFuse === true,
    presentation: normalizeCardPresentation(draft?.presentation),
    faces: Array.isArray(draft?.faces)
      ? draft.faces.map((face) => ({
        name: face?.name || "",
        manaCost: face?.manaCost || "",
        colorIndicator: Array.isArray(face?.colorIndicator) ? [...face.colorIndicator] : [],
        supertypes: Array.isArray(face?.supertypes) ? [...face.supertypes] : [],
        cardTypes: Array.isArray(face?.cardTypes) ? [...face.cardTypes] : [],
        subtypes: Array.isArray(face?.subtypes) ? [...face.subtypes] : [],
        oracleText: face?.oracleText || "",
        artUrl: face?.artUrl || customCardArtUrl(face?.name),
        artSourceUrl: face?.artSourceUrl || "",
        artCrop: face?.artCrop ? { ...face.artCrop } : null,
        power: face?.power || "",
        toughness: face?.toughness || "",
        loyalty: face?.loyalty != null ? String(face.loyalty) : "",
        defense: face?.defense != null ? String(face.defense) : "",
      }))
      : [blankFace()],
  };
}


function faceTabLabel(layout, index) {
  if (layout === "meld") return ["First Card", "Second Card", "Meld Result"][index];
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
  const ui = useUiText();
  return (
    <div className="grid gap-2 sm:grid-cols-2">
      {LAYOUT_OPTIONS.map((option) => {
        const Icon = option.icon;
        const active = value === option.value;
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={active}
            className={cn("card-forge-choice", active && "is-active")}
            onClick={() => onChange(option.value)}
          >
            <span className="flex items-center gap-2">
              <Icon className="size-4" />
              {ui(option.label)}
            </span>
          </button>
        );
      })}
    </div>
  );
}

function ToggleChipGroup({ values, options, onToggle }) {
  const ui = useUiText();
  return (
    <div className="flex flex-wrap gap-1.5">
      {options.map((option) => (
        <button
          key={option}
          type="button"
          aria-pressed={values.includes(option)}
          className={cn("card-forge-chip", values.includes(option) && "is-active")}
          onClick={() => onToggle(option)}
        >
          {ui(option)}
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
          aria-pressed={values.includes(color)}
          className={cn("card-forge-color", values.includes(color) && "is-active")}
          onClick={() => onToggle(color)}
        >
          {color}
        </button>
      ))}
    </div>
  );
}

function CompilePanel({ face, previewError, busy }) {
  const ui = useUiText();
  if (previewError) {
    return (
      <section className="card-forge-panel card-forge-compile-panel">
        <div className="card-forge-panel-title">{ui("Compile Status")}</div>
        <div className="card-forge-status card-forge-status--error">{ui(previewError)}</div>
      </section>
    );
  }

  if (busy && !face) {
    return (
      <section className="card-forge-panel card-forge-compile-panel">
        <div className="card-forge-panel-title">{ui("Compile Status")}</div>
        <div className="card-forge-status">{ui("Preparing preview...")}</div>
      </section>
    );
  }

  return (
    <section className="card-forge-panel card-forge-compile-panel">
      <div className="card-forge-panel-title">{ui("Compile Status")}</div>
      <div className="card-forge-status card-forge-status--good">{ui("Ready")}{busy ? ui(" • refreshing") : ""}
      </div>
      {(face?.customMechanics || []).map(mechanic => (
        <div key={mechanic.keyword} className="card-forge-codeblock" role="note">
          <strong>{mechanic.keyword}</strong>: {mechanic.reminder}
          <div className="text-xs opacity-70">Compiled meaning: {mechanic.expansion}</div>
        </div>
      ))}
      <div className="card-forge-compile-grid">
        <div className="grid gap-1.5">
          <div className="card-forge-section-label">{ui("Compiled Text")}</div>
          <div className="card-forge-codeblock">
            {(face?.compiledText?.length || 0) > 0 ? face.compiledText.join("\n") : ui("No compiled spell text")}
          </div>
        </div>
      </div>
    </section>
  );
}

function CompiledAbilitiesPanel({ face, faces, previewError, busy }) {
  const ui = useUiText();
  const [showEmptyValues, setShowEmptyValues] = useState(false);
  const [copyStatus, setCopyStatus] = useState("");
  const rawCompilation = face?.rawCompilation || "";
  const fullDefinition = (faces || []).map((entry) => entry.rawCompilation || "").filter(Boolean).join("\n\n");
  const displayedDefinition = showEmptyValues
    ? rawCompilation
    : hideEmptyDefinitionFields(rawCompilation);
  return (
    <section className="card-forge-panel card-forge-abilities-panel">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="card-forge-section-label">{ui("Compiled Abilities")}</div>
        <Button type="button" variant="secondary" size="sm"
          disabled={!fullDefinition || busy || Boolean(previewError)}
          onClick={async () => {
            const copied = await copyTextToClipboard(fullDefinition);
            setCopyStatus(copied ? "Card definition copied" : "Could not copy card definition");
          }}>
          <Copy className="size-3.5" aria-hidden="true" />{ui("Copy card definition")}
        </Button>
        <span role="status" className="text-xs">{copyStatus ? ui(copyStatus) : ""}</span>
        <label className="flex cursor-pointer items-center gap-2 text-xs">
          <Checkbox
            checked={showEmptyValues}
            onCheckedChange={(checked) => setShowEmptyValues(checked === true)}
            aria-label={ui("Show empty values")}
          />
          {ui("Show empty values")}
        </label>
      </div>
      <div className="card-forge-codeblock">
        {previewError
          ? ui(previewError)
          : busy && !face
            ? ui("Preparing preview...")
            : displayedDefinition || ui("No compiled abilities")}
      </div>
    </section>
  );
}

function layoutLabel(layout) {
  if (layout === "meld") return "Meld";
  if (layout === "split") return "Split";
  if (layout === "transform_like") return "Double-faced";
  return "Single-face";
}

function InlineFaceEditor({ face, layout, hasFuse, token = false, busy, updateFace, toggleFaceArrayValue, layoutControls }) {
  const ui = useUiText();
  return (
    <section className="card-forge-inline-editor">
      <div className="card-forge-preview-aurora" />
      <div className="card-forge-inline-frame">
        <div className="card-forge-inline-header">
          <div className="min-w-0">
            <input
              className="card-forge-inline-name"
              aria-label={ui("Card name")}
              value={face.name}
              onChange={(event) => updateFace({ name: event.target.value })}
            />
            <div className="card-forge-preview-layout">
              {token ? ui("Token") : ui(layoutLabel(layout))}
              {hasFuse ? ui(" • Fuse") : ""}
              {busy ? ui(" • compiling") : ""}
            </div>
          </div>
          <div className="card-forge-inline-header-side">
            {token ? null : (
              <input
                className="card-forge-inline-cost"
                aria-label={ui("Mana cost")}
                placeholder="{2}{W}{U}"
                value={face.manaCost}
                onChange={(event) => updateFace({ manaCost: event.target.value })}
              />
            )}
          </div>
        </div>

        <div className="card-forge-layout-art-row">
          {layoutControls}
          <CustomCardArtEditor key={face.name} face={face} updateFace={updateFace} />
        </div>

        <div className="card-forge-inline-type">
          <div className="card-forge-inline-row">
            <span className="card-forge-section-label">{ui("Indicator")}</span>
            <ColorToggleGroup
              values={face.colorIndicator}
              onToggle={(value) => toggleFaceArrayValue("colorIndicator", value)}
            />
          </div>
          <div className="card-forge-inline-row">
            <span className="card-forge-section-label">{ui("Supertypes")}</span>
            <ToggleChipGroup
              values={face.supertypes}
              options={SUPER_TYPES}
              onToggle={(value) => toggleFaceArrayValue("supertypes", value)}
            />
          </div>
          <div className="card-forge-inline-row card-forge-inline-row--types">
            <span className="card-forge-section-label">{ui("Types")}</span>
            <ToggleChipGroup
              values={face.cardTypes}
              options={CARD_TYPES}
              onToggle={(value) => toggleFaceArrayValue("cardTypes", value)}
            />
          </div>
          <label className="card-forge-inline-subtypes">
            <span className="card-forge-section-label">{ui("Subtypes")}</span>
            <input
              className="card-forge-input card-forge-input--compact"
              placeholder={ui("Wizard, Human")}
              value={joinedSubtypes(face)}
              onChange={(event) => updateFace({ subtypes: setFromCsv(event.target.value) })}
            />
          </label>
        </div>

        <textarea
          className="card-forge-inline-rules"
          aria-label={ui("Rules text")}
          placeholder={ui("Write oracle-style rules text here...")}
          value={face.oracleText}
          onChange={(event) => updateFace({ oracleText: event.target.value })}
        />

        <div className="card-forge-inline-stats">
          <label className="card-forge-stat-field">
            <span>{ui("Power")}</span>
            <input
              className="card-forge-input card-forge-input--compact"
              placeholder={ui("2 or *")}
              value={face.power}
              onChange={(event) => updateFace({ power: event.target.value })}
            />
          </label>
          <label className="card-forge-stat-field">
            <span>{ui("Toughness")}</span>
            <input
              className="card-forge-input card-forge-input--compact"
              placeholder={ui("2 or *+1")}
              value={face.toughness}
              onChange={(event) => updateFace({ toughness: event.target.value })}
            />
          </label>
          <label className="card-forge-stat-field">
            <span>{ui("Loyalty")}</span>
            <input
              className="card-forge-input card-forge-input--compact"
              type="number"
              min={0}
              value={face.loyalty}
              onChange={(event) => updateFace({ loyalty: event.target.value })}
            />
          </label>
          <label className="card-forge-stat-field">
            <span>{ui("Defense")}</span>
            <input
              className="card-forge-input card-forge-input--compact"
              type="number"
              min={0}
              value={face.defense}
              onChange={(event) => updateFace({ defense: event.target.value })}
            />
          </label>
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
  const ui = useUiText();
  const { game, state, refresh, runWasmInteraction, setStatus, semanticThreshold, setSemanticThreshold, cardsMeetingThreshold } = useGame();
  const [open, setOpen] = useState(false);
  const [workspaceTab, setWorkspaceTab] = useState("card");
  const [customSetDraft, setCustomSetDraft] = useState(emptyCommunityFormat);
  const [customFormatDraft, setCustomFormatDraft] = useState(emptyCommunityFormat);
  const [sandboxBusy, setSandboxBusy] = useState(false);
  const [catalogCard, setCatalogCard] = useState(null);
  const [seedLoading, setSeedLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveMessage, setSaveMessage] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [counterShowcaseLoading, setCounterShowcaseLoading] = useState(false);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState("");
  const [seedDraft, setSeedDraft] = useState(null);
  const [draft, setDraft] = useState(blankDraft());
  const [preview, setPreview] = useState(null);
  const [activeFace, setActiveFace] = useState("face-0");
  // The real card a draft started from, whose printing "Current printing" shows.
  const [printingName, setPrintingName] = useState(null);
  const [currentPrinting, setCurrentPrinting] = useState(null);
  const selectionRequest = useRef(0);
  const [savedTokens, setSavedTokens] = useState([]);
  const isToken = draft.token === true;

  const cancelSelection = useCallback(() => {
    selectionRequest.current++;
    setSeedLoading(false);
  }, []);

  const activeFaceIndex = Number(activeFace.replace("face-", "")) || 0;
  const previewFace = preview?.faces?.[activeFaceIndex] || null;
  const primaryName = preview?.faces?.[0]?.name || draft.faces[0]?.name || "Custom Card";
  const unchangedCatalogCard = catalogCard && rulesSnapshot(draft) === catalogCard.snapshot;
  const canCompile = !disabled && !submitting && !seedLoading && (unchangedCatalogCard || (Boolean(preview?.canCreate) && !previewError && !previewLoading));

  useEffect(() => {
    if (activeFaceIndex >= draft.faces.length) {
      setActiveFace("face-0");
    }
  }, [activeFaceIndex, draft.faces.length]);

  useEffect(() => {
    if (!printingName) return undefined;
    let active = true;
    (async () => {
      const imageUrl = await resolveScryfallImageUrl(printingName, "normal");
      // Custom cards have local art but no Scryfall printing; a failed
      // printing lookup must not throw their art away with it.
      const printing = imageUrl
        ? await resolveScryfallPrintingMetadata(imageUrl).catch(() => null)
        : null;
      if (active) setCurrentPrinting({ name: printingName, imageUrl, printing });
    })().catch(() => { if (active) setCurrentPrinting(null); });
    return () => { active = false; };
  }, [printingName]);
  // Renaming the card leaves that printing behind.
  const draftPrinting = currentPrinting && currentPrinting.name === draft.faces[0]?.name ? currentPrinting : null;

  // Appearance edits do not change compiler input. Keep the debounce tied to
  // the rules snapshot so printing controls never queue another WASM compile.
  const previewInput = JSON.stringify(normalizeDraftForApi(draft));
  useEffect(() => {
    if (!open || !game) return undefined;

    let active = true;
    const timeoutId = window.setTimeout(async () => {
      setPreviewLoading(true);
      try {
        const { preview: nextPreview } = await previewCustomDraft(JSON.parse(previewInput), input => game.previewCustomCard(input));
        if (!active) return;
        setPreview(nextPreview);
        setPreviewError("");
      } catch (error) {
        if (!active) return;
        setPreview(null);
        setPreviewError(String(error?.message || error));
      } finally {
        if (active) setPreviewLoading(false);
      }
    }, 180);

    return () => { active = false; window.clearTimeout(timeoutId); };
  }, [previewInput, game, open]);

  const loadSeed = useCallback(async ({ reroll = false } = {}) => {
    setSeedLoading(true);
    setCatalogCard(null);
    try {
      if (!game || typeof game.sampleLoadedDeckSeed !== "function") {
        throw new Error("This WASM build does not expose sampleLoadedDeckSeed");
      }
      // Samples any nonland card in the decks at the table; a reroll skips the
      // card already shown so the button always visibly changes the sample.
      const excludeName = reroll ? seedDraft?.faces?.[0]?.name || null : null;
      const seed = cloneDraft(await game.sampleLoadedDeckSeed(excludeName));
      setSeedDraft(seed);
      setDraft(seed);
      setPrintingName(seed.faces[0]?.name || null);
      setActiveFace("face-0");
      setPreviewError("");
      if (reroll) {
        setStatus(`Loaded example ${seed.faces?.[0]?.name || "a deck card"}`);
      }
    } catch {
      if (reroll && seedDraft) {
        setStatus("No other deck card is available to sample");
        return;
      }
      const fallback = blankDraft();
      setSeedDraft(fallback);
      setDraft(fallback);
      setActiveFace("face-0");
      setPreview(null);
      setPreviewError("");
      setStatus("No deck cards at the table to sample; starting from a blank card");
    } finally {
      setSeedLoading(false);
    }
  }, [game, seedDraft, setStatus]);

  useEffect(() => {
    if (!open) return undefined;
    let active = true;
    const load = () => listCustomTokens()
      .then((tokens) => { if (active) setSavedTokens(tokens); })
      .catch(() => {});
    load();
    globalThis.addEventListener?.(CUSTOM_TOKENS_CHANGED, load);
    return () => { active = false; globalThis.removeEventListener?.(CUSTOM_TOKENS_CHANGED, load); };
  }, [open]);

  // A token is a single face with no mana cost; turning it off restores a card.
  const handleTokenToggle = useCallback((checked) => {
    cancelSelection();
    setCatalogCard(null);
    setDraft((current) => {
      if (!checked) {
        const { token: _token, ...card } = current;
        return card;
      }
      return {
        ...current,
        token: true,
        layout: "single",
        hasFuse: false,
        faces: [{ ...current.faces[0], manaCost: "" }],
      };
    });
    setActiveFace("face-0");
  }, [cancelSelection]);

  const loadSavedToken = useCallback((token) => {
    cancelSelection();
    const next = cloneDraft({ layout: "single", token: true, faces: [token.face] });
    setDraft(next);
    setSeedDraft(next);
    setCatalogCard(null);
    setPrintingName(null);
    setActiveFace("face-0");
    setSaveMessage("");
  }, [cancelSelection]);

  const removeSavedToken = useCallback(async (token) => {
    try {
      await deleteCustomToken(token.name);
      setSaveMessage(`Deleted token ${token.name}. Cards that already create it keep their copy.`);
    } catch (error) {
      setSaveMessage(`Delete failed: ${String(error?.message || error)}`);
    }
  }, []);

  const handleOpenChange = useCallback((nextOpen) => {
    if (sandboxBusy) return;
    cancelSelection();
    setOpen(nextOpen);
    if (nextOpen) {
      setSaveMessage("");

    }
  }, [cancelSelection, sandboxBusy]);

  const updateFace = useCallback((index, patch) => {
    cancelSelection();
    setDraft((current) => ({
      ...current,
      faces: current.faces.map((face, faceIndex) => (
        faceIndex === index ? { ...face, ...patch } : face
      )),
    }));
  }, [cancelSelection]);

  const toggleFaceArrayValue = useCallback((index, key, value) => {
    cancelSelection();
    setDraft((current) => ({
      ...current,
      faces: current.faces.map((face, faceIndex) => (
        faceIndex === index
          ? { ...face, [key]: toggleChoice(face[key], value) }
          : face
      )),
    }));
  }, [cancelSelection]);

  const handleLayoutChange = useCallback((nextLayout) => {
    cancelSelection();
    setDraft((current) => {
      const nextFaces = [...current.faces];
      if (nextLayout === "single") {
        return {
          layout: nextLayout,
          hasFuse: false,
          faces: [nextFaces[0] || blankFace()],
          presentation: current.presentation,
        };
      }
      const faceCount = nextLayout === "meld" ? 3 : 2;
      while (nextFaces.length < faceCount) {
        nextFaces.push(blankFace(faceTabLabel(nextLayout, nextFaces.length)));
      }
      return {
        layout: nextLayout,
        hasFuse: nextLayout === "split" ? current.hasFuse : false,
        faces: nextFaces.slice(0, faceCount),
        presentation: current.presentation,
      };
    });
    setActiveFace("face-0");
  }, [cancelSelection]);

  const resetToSeed = useCallback(() => {
    if (!seedDraft) return;
    cancelSelection();
    setDraft(cloneDraft(seedDraft));
    setActiveFace("face-0");
  }, [seedDraft, cancelSelection]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveMessage("");
    try {
      const savedDraft = normalizeDraftForApi(draft);
      savedDraft.faces.forEach((face, index) => {
        face.artUrl = draft.faces[index].artUrl || "";
        face.artSourceUrl = draft.faces[index].artSourceUrl || "";
        face.artCrop = draft.faces[index].artCrop || null;
      });
      // Validate this exact snapshot, even if the debounced preview is stale.
      // Printing choices are saved with the card but never sent to the engine.
      savedDraft.presentation = draft.presentation;
      if (savedDraft.token) {
        const token = await saveCustomToken(savedDraft, { compile: input => game.previewCustomCard(input) });
        setCustomCardArtUrls([{ name: token.name, artUrl: draft.faces[0].artUrl }]);
        setSaveMessage(`Saved token ${token.name}. Other cards can now create it by name, for example "Create two ${token.name} tokens." Saved on this browser; saving the same name updates it.`);
        return;
      }
      await saveCustomCard(savedDraft, { compile: input => game.previewCustomCard(input) });
      setSaveMessage(`Saved ${savedDraft.faces[0].name}. Find it with Include custom cards in Build. Saved on this browser; saving the same name updates it.`);
    } catch (error) {
      setSaveMessage(`Save failed: ${String(error?.message || error)}`);
    } finally {
      setSaving(false);
    }
  }, [draft, game]);

  const handleCompile = useCallback(async () => {
    return runWasmInteraction(async () => {
      if (!game || typeof game.createCustomCard !== "function") {
        setStatus("This WASM build does not expose custom card compilation", true);
        return;
      }
      setSubmitting(true);
      const existing = snapshotStableIds(state);
      try {
        if (unchangedCatalogCard) {
          const objectId = await game.addCardToZone(selectedPlayer, catalogCard.name, zone, skipTriggers);
          // Only this copy changes, and only when a printing choice was made.
          if (!isDefaultCardPresentation(draft.presentation)) {
            setCardPresentation(objectId, [catalogCard.name], draft.presentation, { existing });
          }
          setOpen(false);
          await refresh(`Added ${catalogCard.name}`);
          return;
        }
        const prepared = await prepareCustomDraftForCreation(normalizeDraftForApi(draft), input => game.previewCustomCard(input));
        const objectId = await game.createCustomCard({
          draft: prepared.input,
          playerIndex: selectedPlayer,
          // A token only exists on the battlefield (CR 111.7).
          zoneName: isToken ? "battlefield" : zone,
          skipTriggers,
        });
        setCardPresentation(objectId, [draft.faces.map((face) => face.name).join(" // "), ...draft.faces.map((face) => face.name)],
          draft.presentation, { existing, builderCreated: true });
        setCustomCardArtUrls([
          ...draft.faces.map((face) => ({ name: face.name, artUrl: face.artUrl })),
          // Tokens this card creates show the art saved with them.
          ...(prepared.draft.customTokens || []).filter((token) => token.face.artUrl)
            .map((token) => ({ name: token.name, artUrl: token.face.artUrl })),
        ]);
        // The compiled card owns its text from here on, whatever printing its
        // name resolves to.
        setCompiledCardNames(draft.faces.map((face) => face.name));
        setOpen(false);
        await refresh(`Compiled ${primaryName}`);
      } catch (error) {
        setStatus(`Compile card failed: ${String(error?.message || error)}`, true);
      } finally {
        setSubmitting(false);
      }
    });
  }, [draft, game, isToken, primaryName, refresh, runWasmInteraction, selectedPlayer, setStatus, skipTriggers, zone, unchangedCatalogCard, catalogCard, state]);

  const handleCounterShowcase = useCallback(async () => {
    const result = await runWasmInteraction(async () => {
      if (!game || typeof game.createCustomCard !== "function") {
        setStatus("This WASM build does not expose custom card compilation", true);
        return false;
      }

      setCounterShowcaseLoading(true);
      const created = [];
      const failed = [];
      try {
        for (const preset of COUNTER_SHOWCASE_PRESET_LIST) {
          try {
            await game.createCustomCard({
              draft: normalizeDraftForApi(counterShowcaseDraft(preset)),
              playerIndex: selectedPlayer,
              counterSeed: preset.counter,
              // Reuse the placement selector so the showcase can exercise
              // counters in every visible zone without authoring one rule
              // sentence per counter kind.
              zoneName: zone,
              // This is an explicit visual lab: seed the same runtime counter
              // snapshot directly and leave normal card rules untouched.
              skipTriggers: true,
            });
            created.push(preset.label);
          } catch (error) {
            failed.push(`${preset.label}: ${String(error?.message || error)}`);
          }
        }

        if (created.length === 0) {
          setStatus("Counter showcase could not compile any card", true);
          return false;
        }

        // Counter showcase cards intentionally have descriptive custom names, so the regular
        // name-based art resolver cannot find a printing for them. Apply any
        // already-known art immediately; resolving a fallback printing must
        // never hold the WASM operation open or delay the visible cards.
        const knownArtUrl = seedDraft?.faces?.[0]?.artUrl
          || draft.faces?.[0]?.artUrl
          || "";
        setCustomCardCounterOverrides({
          [COUNTER_SHOWCASE_MIXED_PRESET.name]: COUNTER_SHOWCASE_MIXED_PRESET.overrideCounters,
          [COUNTER_SHOWCASE_THREE_THREE_PRESET.name]: COUNTER_SHOWCASE_THREE_THREE_PRESET.overrideCounters,
          [COUNTER_SHOWCASE_POSITIVE_STACK_PRESET.name]: COUNTER_SHOWCASE_POSITIVE_STACK_PRESET.overrideCounters,
        });
        setCustomCardArtUrls(COUNTER_SHOWCASE_PRESET_LIST.map((preset) => ({
          name: preset.name,
          artUrl: knownArtUrl,
        })));
        setCompiledCardNames(COUNTER_SHOWCASE_PRESET_LIST.map((preset) => preset.name));

        setOpen(false);
        await refresh(
          `Counter showcase: ${created.length} cards in ${zone}${
            failed.length > 0 ? ` (${failed.length} skipped)` : ""
          }`
        );
        if (failed.length > 0) {
          console.warn("Counter showcase skipped presets:", failed);
        }
        // Resolve a real printing after the showcase is visible. This is
        // deliberately fire-and-forget so a slow/offline art service cannot
        // make the counter loader appear stuck.
        void (async () => {
          const sampledDraft = seedDraft?.faces?.[0]?.name
            ? seedDraft
            : (typeof game.sampleLoadedDeckSeed === "function"
              ? await game.sampleLoadedDeckSeed(null).catch(() => null)
              : null);
          const artSourceName = sampledDraft?.faces?.[0]?.name || draft.faces?.[0]?.name;
          if (!artSourceName) return;
          const showcaseArtUrl = await resolveScryfallImageUrl(artSourceName, "normal").catch(() => "");
          if (!showcaseArtUrl) return;
          setCustomCardArtUrls(COUNTER_SHOWCASE_PRESET_LIST.map((preset) => ({
            name: preset.name,
            artUrl: showcaseArtUrl,
          })));
          await refresh();
        })().catch(() => {});
        return true;
      } finally {
        setCounterShowcaseLoading(false);
      }
    });
    if (result === undefined) {
      setStatus("Counter showcase is waiting for another game action to finish", true);
    }
    return result;
  }, [draft.faces, game, refresh, runWasmInteraction, seedDraft, selectedPlayer, setStatus, zone]);

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

  const layoutControls = (
    <section className="card-forge-layout-controls">
      <div className="card-forge-panel-title">{ui("Card Layout")}</div>
      {isToken ? null : <LayoutPicker value={draft.layout} onChange={handleLayoutChange} />}
      <label className="toolbar-checkbox mt-2 flex items-center gap-2 text-[13px] uppercase tracking-wide">
        <Checkbox
          checked={isToken}
          onCheckedChange={(checked) => handleTokenToggle(checked === true)}
          className="h-3.5 w-3.5"
        />{ui("Token")}</label>
      {isToken ? (
        <p className="text-xs text-muted-foreground">{ui("Save a named token and other cards can create it by name, for example “Create two {0} tokens.” Tokens have no mana cost and are added to the battlefield.", { 0: draft.faces[0]?.name || "Bobble" })}</p>
      ) : null}
      {isToken && savedTokens.length ? (
        <div className="mt-2 flex flex-wrap gap-1.5" aria-label={ui("Saved tokens")}>
          {savedTokens.map((token) => (
            <span key={token.key} className="inline-flex items-center gap-0.5">
              <button
                type="button"
                className={cn("card-forge-chip", tokenKey(draft.faces[0]?.name) === token.key && "is-active")}
                onClick={() => loadSavedToken(token)}
              >{token.name}</button>
              <button
                type="button"
                className="card-forge-chip"
                aria-label={ui("Delete token {0}", { 0: token.name })}
                onClick={() => void removeSavedToken(token)}
              >×</button>
            </span>
          ))}
        </div>
      ) : null}
      {draft.layout === "meld" ? (
        <p className="text-xs text-muted-foreground">{ui("Edit both cards and their meld result. Add a meld ability to a component’s rules text using the exact partner and result names. Adding creates both component cards in the selected zone.")}</p>
      ) : null}
      {draft.layout === "split" ? (
        <label className="toolbar-checkbox mt-2 flex items-center gap-2 text-[13px] uppercase tracking-wide">
          <Checkbox
            checked={draft.hasFuse}
            onCheckedChange={(checked) => {
              setDraft((current) => ({ ...current, hasFuse: checked === true }));
            }}
            className="h-3.5 w-3.5"
          />{ui("Fuse enabled")}</label>
      ) : null}
    </section>
  );

  return (
    <>
      {triggerNode ?? (
        <button
          type="button"
          className="stone-pill inline-flex items-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45"
          disabled={disabled}
          onClick={() => handleOpenChange(true)}
        >{ui("Compile Card")}</button>
      )}

      <Sheet open={open} onOpenChange={handleOpenChange}>
        <SheetContent
          side="center"
          className="card-forge-sheet fantasy-sheet overflow-hidden p-0"
          style={{
            width: "100dvw",
            maxWidth: "none",
            height: "100dvh",
            maxHeight: "100dvh",
          }}
        >
          <SheetHeader className="fantasy-sheet-header card-forge-header pr-12">
            <SheetTitle>{ui("Build")}</SheetTitle>
            <SheetDescription className="card-forge-description max-w-[58ch] text-[13px] leading-5">{ui("Create cards and mechanics, collect cards into sets, and design custom formats for your games.")}</SheetDescription>
          </SheetHeader>

          <Tabs value={workspaceTab} onValueChange={setWorkspaceTab} className="flex min-h-0 flex-1 flex-col overflow-hidden">
            <TabsList variant="line" className="shrink-0 max-w-full justify-start overflow-x-auto px-4">
              <TabsTrigger value="card" disabled={sandboxBusy}>{ui("Card")}</TabsTrigger>
              <TabsTrigger value="mechanics" disabled={sandboxBusy}>{ui("Mechanics")}</TabsTrigger>
              <TabsTrigger value="sets" disabled={sandboxBusy}>{ui("Sets")}</TabsTrigger>
              <TabsTrigger value="formats" disabled={sandboxBusy}>{ui("Formats")}</TabsTrigger>
              <TabsTrigger value="sandbox" disabled={submitting}>{ui("Sandbox")}</TabsTrigger>
            </TabsList>
            <TabsContent forceMount value="sets" className="min-h-0 flex-1 overflow-y-auto data-[state=inactive]:hidden">
              <CommunityFormatEditor kind="set" active={open && workspaceTab === 'sets'} value={customSetDraft} onChange={setCustomSetDraft} />
            </TabsContent>
            <TabsContent forceMount value="formats" className="min-h-0 flex-1 overflow-y-auto data-[state=inactive]:hidden">
              <CommunityFormatEditor active={open && workspaceTab === 'formats'} value={customFormatDraft} onChange={setCustomFormatDraft} />
            </TabsContent>
            <TabsContent value="mechanics" className="min-h-0 flex-1 overflow-y-auto">
              <MechanicsPanel />
            </TabsContent>
            <TabsContent forceMount value="sandbox" className="min-h-0 flex-1 overflow-y-auto data-[state=inactive]:hidden">
              <SandboxPanel currentDraft={draft} onBusyChange={setSandboxBusy} onStarted={() => setOpen(false)} />
            </TabsContent>
            <TabsContent value="card" className="min-h-0 flex-1 overflow-hidden data-[state=active]:flex data-[state=active]:flex-col">
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <BuildCardSearch
            actions={<div className="build-card-toolbar-actions">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                onClick={() => handleOpenChange(false)}
              >{ui("Cancel")}</Button>
              <Button type="button" variant="secondary" size="sm" className="stone-pill"
                disabled={disabled || saving || submitting || seedLoading || !game || draft.faces.some(face => !face.name.trim())}
                onClick={() => void handleSave()}>{saving ? ui("Saving...") : ui("Save")}</Button>
              <Button
                type="button"
                size="sm"
                className="card-forge-submit stone-pill"
                disabled={!canCompile}
                onClick={() => void handleCompile()}
              >
                {submitting ? ui("Adding...") : ui("Add")}
              </Button>
                  <label className="card-forge-field">
                    <span className="card-forge-section-label">{ui("Player")}</span>
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
                    <span className="card-forge-section-label">{ui("Zone")}</span>
                    <select
                      className="card-forge-input"
                      value={isToken ? "battlefield" : zone}
                      disabled={isToken}
                      onChange={(event) => onZoneChange?.(event.target.value)}
                    >
                      {ZONE_OPTIONS.map(([value, label]) => (
                        <option key={value} value={value}>
                          {ui(label)}
                        </option>
                      ))}
                    </select>
                  </label>

            </div>}
            secondaryControls={<div className="build-card-toolbar-status">
              {saveMessage ? <p role="status" className="text-[12px] leading-4 text-foreground">{saveMessage}</p> : null}
                <label className="toolbar-checkbox flex items-center gap-2 text-[13px] uppercase tracking-wide">
                  <Checkbox
                    checked={skipTriggers}
                    onCheckedChange={(checked) => onSkipTriggersChange?.(checked === true)}
                    className="h-3.5 w-3.5"
                  />{ui("Skip triggers")}</label>
                <div className="build-card-fidelity">
                  <span className="build-card-fidelity-label text-[13px] uppercase tracking-wide">{ui("Fidelity")}</span>
                  <Slider
                    aria-label={ui("Card fidelity threshold")}
                    className="build-card-fidelity-slider"
                    min={0}
                    max={100}
                    step={1}
                    value={[Math.round(semanticThreshold)]}
                    onValueChange={([value]) => setSemanticThreshold(value)}
                  />
                  <span className="build-card-fidelity-value text-[12px] tabular-nums">
                    {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : ui("Off")}
                  </span>
                  <Popover>
                    <PopoverTrigger asChild>
                      <button type="button" className="build-card-fidelity-help" aria-label={ui("What does fidelity do?")}>
                        <CircleHelp size={15} />
                      </button>
                    </PopoverTrigger>
                    <PopoverContent side="top" data-ui-layer="tooltip" className="build-card-fidelity-help-content text-[13px] leading-5">
                      {ui("Fidelity is how closely our compiled version of a card matches its printed Oracle text. Search only shows cards at or above this threshold, so lower it to find cards that compile but read less like the original. Off shows every card. Currently {0} cards meet the threshold. This is the same setting as Card fidelity threshold in Settings.", { 0: cardsMeetingThreshold })}
                    </PopoverContent>
                  </Popover>
                </div>
            </div>}
            disabled={disabled || submitting} onSelect={async (option) => {
            const request = ++selectionRequest.current;
            setSeedLoading(true);
            setSaveMessage("");
            try {
              const next = cloneDraft(option.card?.draft || await fetchCardBuildDraft(option.name));
              if (request !== selectionRequest.current) return;
              // Custom cards have no Scryfall printing; paint their bundled art.
              if (option.card) {
                next.faces = next.faces.map((face) => ({
                  ...face, artUrl: face.artUrl || customCardImageUrl(face.name, "art_crop"),
                }));
              }
              setDraft(next);
              setSeedDraft(next);
              setCatalogCard(option.card ? null : { name: option.name, snapshot: rulesSnapshot(next) });
              setPrintingName(option.card ? null : next.faces[0]?.name || null);
              setActiveFace("face-0");
              setPreview(null);
              setPreviewError("");
            } catch (error) {
              if (request === selectionRequest.current) setSaveMessage(`Could not load card: ${String(error?.message || error)}`);
            } finally {
              if (request === selectionRequest.current) setSeedLoading(false);
            }
          }} />
          <div className="card-forge-toolbar">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
              <div className="card-forge-banner">
                {seedLoading ? ui("Loading deck sample...") : ui("Editing: {0}", { 0: draft.faces?.[0]?.name || "New card" })}
              </div>
              <p className="card-forge-printing-credit">
                {ui("Card frames rendered by")}{" "}
                <a href="https://github.com/vishvananda/card-glint" target="_blank" rel="noopener noreferrer">card-glint</a>
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="secondary" size="sm" className="stone-pill" disabled={submitting} onClick={() => { cancelSelection(); setDraft(blankDraft()); setSeedDraft(null); setCatalogCard(null); setPrintingName(null); setActiveFace("face-0"); }}>{ui("New card")}</Button>
              <CustomCardsButton className="stone-pill px-3 py-2" label={ui("Load custom cards")} />
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={seedLoading}
                onClick={() => void runWasmInteraction(() => loadSeed({ reroll: true }))}
              >
                <RefreshCw className={cn("size-3.5", seedLoading && "animate-spin")} />{ui("New Sample")}</Button>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={!seedDraft}
                onClick={resetToSeed}
              >{ui("Reset changes")}</Button>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={disabled || submitting || counterShowcaseLoading || seedLoading || previewLoading}
                onClick={() => void handleCounterShowcase()}
              >
                {counterShowcaseLoading ? (
                  <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
                ) : null}
                {counterShowcaseLoading ? ui("Loading counters...") : ui("Load counter showcase")}
              </Button>
            </div>
          </div>

          <div className="card-forge-grid">
            <div className="card-forge-main card-forge-workbench">
              <CardPrintingStage
                draft={draft}
                faceIndex={activeFaceIndex}
                printing={draftPrinting}
                onPresentationChange={(presentation) => setDraft((current) => ({ ...current, presentation }))}
              />
              <div className="card-forge-details">
              {draft.faces.length === 1 ? (
                <div className="card-forge-single-face">
                  <InlineFaceEditor
                    face={draft.faces[0]}
                    layout={draft.layout}
                    layoutControls={layoutControls}
                    hasFuse={draft.hasFuse}
                    token={draft.token === true}
                    busy={previewLoading}
                    updateFace={(patch) => updateFace(0, patch)}
                    toggleFaceArrayValue={(key, value) => toggleFaceArrayValue(0, key, value)}
                  />
                </div>
              ) : (
                <Tabs value={activeFace} onValueChange={setActiveFace} className="card-forge-face-tabs">
                  <TabsList variant="line" className="card-forge-tabs-list">
                    {faceTabs.map((tab) => (
                      <TabsTrigger
                        key={tab.value}
                        value={tab.value}
                        className="card-forge-tab-trigger"
                      >
                        <span className="grid text-left">
                          <span>{ui(tab.title)}</span>
                          <span className="card-forge-tab-subtitle text-[10px] uppercase tracking-[0.2em]">
                            {ui(tab.subtitle)}
                          </span>
                        </span>
                      </TabsTrigger>
                    ))}
                  </TabsList>

                  {draft.faces.map((face, index) => (
                    <TabsContent
                      key={`face-panel-${index}`}
                      value={`face-${index}`}
                      className="card-forge-face-content"
                    >
                      <InlineFaceEditor
                        face={face}
                        layout={draft.layout}
                        layoutControls={layoutControls}
                        hasFuse={draft.hasFuse}
                        busy={previewLoading}
                        updateFace={(patch) => updateFace(index, patch)}
                        toggleFaceArrayValue={(key, value) => toggleFaceArrayValue(index, key, value)}
                      />
                    </TabsContent>
                  ))}
                </Tabs>
              )}
              </div>
            </div>

            <div className="card-forge-side">
              <CompilePanel face={previewFace} previewError={previewError} busy={previewLoading} />
              <CompiledAbilitiesPanel face={previewFace} faces={preview?.faces} previewError={previewError} busy={previewLoading} />
            </div>

          </div>


          </div>
            </TabsContent>
          </Tabs>
        </SheetContent>
      </Sheet>
    </>
  );
}
