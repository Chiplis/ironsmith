import { useEffect, useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { ChevronDown, ChevronRight } from "lucide-react";
import { scryfallImageUrl } from "@/lib/scryfall";

function CollapsibleSection({ title, defaultOpen = false, children }) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger className="flex items-center gap-1 text-[10px] uppercase tracking-wider text-muted-foreground font-bold cursor-pointer hover:text-foreground">
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        {title}
      </CollapsibleTrigger>
      <CollapsibleContent>{children}</CollapsibleContent>
    </Collapsible>
  );
}

function StackAbilityView({ entry }) {
  return (
    <div className="grid gap-1.5 text-[11px] pr-1">
      <div className="font-bold text-[13px]">{entry.name}</div>
      <div className="text-muted-foreground italic">{entry.ability_kind} ability</div>
      {entry.ability_text && (
        <pre className="whitespace-pre-wrap text-[11px] text-[#c0d8f0] bg-[#0a1118] border border-[#1e3044] p-1.5 font-[inherit]">
          {entry.ability_text}
        </pre>
      )}
      {entry.effect_text && (
        <pre className="whitespace-pre-wrap text-[11px] text-[#a8c8e4] bg-[#0a1118] border border-[#1e3044] p-1.5 font-[inherit]">
          {entry.effect_text}
        </pre>
      )}
    </div>
  );
}

/** Parse a mana cost string like "{2}{W}{U}" into an array of symbol codes. */
function parseManaSymbols(costStr) {
  if (!costStr) return [];
  const matches = costStr.match(/\{([^}]+)\}/g);
  if (!matches) return [];
  return matches.map((m) => m.slice(1, -1)); // strip braces
}

/** Render an inline mana cost as Scryfall SVG icons. */
function ManaCostIcons({ cost }) {
  const symbols = parseManaSymbols(cost);
  if (!symbols.length) return null;
  return (
    <span className="inline-flex items-center gap-px">
      {symbols.map((sym, i) => (
        <img
          key={i}
          src={`https://svgs.scryfall.io/card-symbols/${encodeURIComponent(sym)}.svg`}
          alt={`{${sym}}`}
          className="w-3 h-3 inline-block"
          loading="lazy"
        />
      ))}
    </span>
  );
}

function ObjectView({ details }) {
  const counters = (details.counters || []).length
    ? details.counters.map((c) => `${c.amount} ${c.kind}`).join(" \u2022 ")
    : null;

  // Text metadata (mana cost excluded — rendered as icons at the end)
  const meta = [
    details.type_line,
    details.power_toughness && `${details.power_toughness}`,
    details.zone,
    details.controller != null && `P${details.controller}`,
    details.tapped && "Tapped",
    counters,
  ].filter(Boolean);

  const artUrl = scryfallImageUrl(details.name, "art_crop");

  return (
    <div className="grid gap-1.5 text-[11px] pr-1">
      <div className="relative overflow-hidden rounded border border-[#1e3044]">
        {artUrl && (
          <img
            className="absolute inset-0 w-full h-full object-cover opacity-40 z-0 pointer-events-none"
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        )}
        <div className="relative z-1 bg-[rgba(10,17,24,0.75)] p-1.5 grid gap-1.5">
          <div className="flex items-center gap-1.5 flex-wrap">
            <span className="font-bold text-[13px] text-shadow-[0_1px_2px_rgba(0,0,0,0.9)]">{details.name || "Unknown"}</span>
            {meta.length > 0 && (
              <span className="text-[10px] text-muted-foreground text-shadow-[0_1px_2px_rgba(0,0,0,0.9)]">{meta.join(" · ")}</span>
            )}
            {details.mana_cost && (
              <>
                {meta.length > 0 && <span className="text-[10px] text-muted-foreground">·</span>}
                <ManaCostIcons cost={details.mana_cost} />
              </>
            )}
          </div>
          <pre className="whitespace-pre-wrap text-[11px] text-[#c0d8f0] m-0 font-[inherit] min-h-[40px] text-shadow-[0_1px_2px_rgba(0,0,0,0.9)]">
            {details.oracle_text || "No oracle text."}
          </pre>
        </div>
      </div>

      {details.compiled_text && (
        <CollapsibleSection title="Compiled Text">
          <pre className="whitespace-pre-wrap text-[11px] text-[#a8c8e4] bg-[#0a1118] border border-[#1e3044] p-1.5 mt-0.5 font-[inherit]">
            {details.compiled_text}
          </pre>
        </CollapsibleSection>
      )}

      {details.abilities && details.abilities.length > 0 && (
        <CollapsibleSection title={`Compiled Abilities (${details.abilities.length})`}>
          <div className="grid gap-1 mt-0.5">
            {details.abilities.map((ab, i) => (
              <div key={i} className="bg-[#0a1118] border border-[#1e3044] p-1 text-[11px]">
                {typeof ab === "string" ? ab : ab.text || ab.kind || `Ability ${i + 1}`}
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {details.raw_compilation && (
        <CollapsibleSection title="Raw Compilation">
          <pre className="whitespace-pre-wrap text-[10px] text-[#8a9eb8] bg-[#0a1118] border border-[#1e3044] p-1.5 mt-0.5 font-[inherit] max-h-[200px] overflow-auto">
            {details.raw_compilation}
          </pre>
        </CollapsibleSection>
      )}
    </div>
  );
}

export default function InspectorPanel({ selectedObjectId }) {
  const { game, state } = useGame();
  const [details, setDetails] = useState(null);
  const [stackEntry, setStackEntry] = useState(null);

  const loadDetails = useCallback(
    async (id) => {
      if (!game || !id) {
        setDetails(null);
        setStackEntry(null);
        return;
      }

      // Check if this is a stack ability — render from stack data directly
      const numId = Number(id);
      const stackObjects = state?.stack_objects || [];
      const entry = stackObjects.find((e) => Number(e.id) === numId);
      if (entry && entry.ability_kind) {
        setStackEntry(entry);
        setDetails(null);
        return;
      }

      setStackEntry(null);
      try {
        const d = await game.objectDetails(BigInt(id));
        setDetails(d);
      } catch (err) {
        console.warn("objectDetails failed:", err);
        setDetails(null);
      }
    },
    [game, state?.stack_objects]
  );

  useEffect(() => {
    loadDetails(selectedObjectId);
  }, [selectedObjectId, loadDetails]);

  return (
    <section className="p-2 border-b border-game-line-2 flex flex-col gap-1.5 min-h-0 overflow-hidden">
      {stackEntry ? (
        <ScrollArea className="min-h-0 flex-1">
          <StackAbilityView entry={stackEntry} />
        </ScrollArea>
      ) : details ? (
        <ScrollArea className="min-h-0 flex-1">
          <ObjectView details={details} />
        </ScrollArea>
      ) : (
        <div className="text-muted-foreground text-[12px] italic">
          Click an object to inspect it
        </div>
      )}
    </section>
  );
}
