import { useEffect, useMemo, useState } from "react";
import CardGlintLivePreview from "@/components/cards/CardGlintLivePreview";
import { Checkbox } from "@/components/ui/checkbox";
import useUiText from "@/i18n/useUiText";
import { cardGlintDraftFace } from "@/lib/card-glint/draft.mjs";
import {
  CARD_BORDER_COLORS,
  CARD_GLINT_FINISHES,
  CARD_RARITIES,
} from "@/lib/card-glint/printing-choices.mjs";
import { loadCardGlint } from "@/lib/card-glint/session.mjs";
import { cn } from "@/lib/utils";

function cyclePresentation(options, value) {
  const index = options.findIndex((option) => option.id === value);
  return options[(index + 1) % options.length].id;
}

function PresentationChoices({ label, options, value, onChange, translate = true }) {
  const ui = useUiText();
  return (
    <div className="card-forge-inline-row">
      <span className="card-forge-section-label">{label}</span>
      <div className="flex flex-wrap gap-1.5" role="group" aria-label={label}>
        {options.map((option) => (
          <button
            key={option.id ?? "default"}
            type="button"
            aria-pressed={value === option.id}
            disabled={option.disabled}
            title={option.reason}
            className={cn("card-forge-chip", value === option.id && "is-active")}
            onClick={() => onChange(option.id)}
          >
            {translate ? ui(option.label) : option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

// card-glint's style catalog, with each style's support for this card.
function usePrintingStyles(face) {
  const faceKey = face ? JSON.stringify(face) : null;
  const [api, setApi] = useState(null);
  useEffect(() => {
    let cancelled = false;
    loadCardGlint().then((value) => { if (!cancelled) setApi(value); }).catch(() => {});
    return () => { cancelled = true; };
  }, []);
  return useMemo(() => {
    if (!api?.printingStyles || !faceKey) return [];
    const { printingChoice: _choice, ...source } = JSON.parse(faceKey);
    return api.printingStyles.map((style) => {
      const frames = api.printingStyleOptions(source, style);
      const supported = frames.find((frame) => frame.supported);
      return { style, frames, disabled: !supported, reason: supported ? undefined : frames[0]?.reason };
    });
  }, [api, faceKey]);
}

// The live preview and printing controls. Clicking the card's border or set
// symbol cycles the border color or rarity.
export default function CardPrintingStage({ draft, faceIndex, printing, onPresentationChange }) {
  const ui = useUiText();
  const [previewEffects, setPreviewEffects] = useState(false);
  const presentation = draft.presentation;
  const face = useMemo(() => cardGlintDraftFace(draft, faceIndex, printing), [draft, faceIndex, printing]);
  // Style support depends on the card, not on the chosen style.
  const sourceFace = useMemo(() => cardGlintDraftFace({ ...draft, presentation: { ...presentation, style: "match", frame: null, borderColor: null } },
    faceIndex, printing), [draft, faceIndex, printing, presentation]);
  const styles = usePrintingStyles(sourceFace);
  const selected = styles.find((entry) => entry.style.id === presentation.style);
  const change = (patch) => onPresentationChange({ ...presentation, ...patch });
  return (
    <section className="card-forge-printing" aria-label={ui("Printing")}>
      <div className="card-forge-printing-card">
        <CardGlintLivePreview
          face={face}
          liveEnabled={previewEffects}
          presentation={presentation}
          onBorderClick={() => change({ borderColor: cyclePresentation(CARD_BORDER_COLORS, presentation.borderColor) })}
          onRarityClick={() => change({ rarity: cyclePresentation(CARD_RARITIES, presentation.rarity) })}
          labels={{
            border: ui("Change border color"),
            rarity: ui("Change rarity"),
            pending: ui("Preparing card preview..."),
            error: ui("Card preview unavailable. You can still edit the details."),
          }}
        />
      </div>
      <div className="card-forge-printing-controls">
        <label className="toolbar-checkbox flex items-center gap-2 text-[13px] uppercase tracking-wide"
          title={ui("Preview foil and print textures (uses more graphics resources)")}>
          <Checkbox checked={previewEffects} onCheckedChange={(checked) => setPreviewEffects(checked === true)}
            className="h-3.5 w-3.5" />{ui("Preview finish effects")}
        </label>
        <PresentationChoices label={ui("Finish")} options={CARD_GLINT_FINISHES} value={presentation.finish}
          onChange={(finish) => change({ finish })} />
        <label className="toolbar-checkbox flex items-center gap-2 text-[13px] uppercase tracking-wide"
          title={ui("Fine cardstock texture and irregular ink dots")}>
          <Checkbox
            checked={presentation.printEffects}
            onCheckedChange={(checked) => change({ printEffects: checked === true })}
            className="h-3.5 w-3.5"
          />{ui("Print effects")}</label>
        {styles.length ? (
          <PresentationChoices label={ui("Printing style")} translate={false}
            options={styles.map(({ style, disabled, reason }) => ({
              id: style.id, disabled, reason,
              label: style.id !== "match" ? style.label
                : printing?.printing?.set ? `${ui("Current printing")} · ${printing.printing.set.toUpperCase()}` : ui("Current printing"),
            }))}
            value={presentation.style}
            onChange={(style) => change({ style, frame: null })} />
        ) : null}
        {selected && selected.style.frames.length > 1 ? (
          <PresentationChoices label={ui("Frame")} translate={false}
            options={[{ id: null, label: ui("Automatic") },
              ...selected.frames.map((frame) => ({ id: frame.id, label: frame.label, disabled: !frame.supported, reason: frame.reason }))]}
            value={presentation.frame}
            onChange={(frame) => change({ frame })} />
        ) : null}
        <PresentationChoices label={ui("Border")} options={CARD_BORDER_COLORS} value={presentation.borderColor}
          onChange={(borderColor) => change({ borderColor })} />
        <PresentationChoices label={ui("Rarity")} options={CARD_RARITIES} value={presentation.rarity}
          onChange={(rarity) => change({ rarity })} />

      </div>
    </section>
  );
}
