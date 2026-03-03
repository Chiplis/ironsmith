import { useEffect, useRef, useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { ChevronDown, ChevronRight } from "lucide-react";
import { scryfallImageUrl } from "@/lib/scryfall";
import { SymbolText, ManaCostIcons } from "@/lib/mana-symbols";

function CollapsibleSection({ title, defaultOpen = false, children }) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger className="flex items-center gap-1 text-[13px] uppercase tracking-wider text-muted-foreground font-bold cursor-pointer hover:text-foreground">
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        {title}
      </CollapsibleTrigger>
      <CollapsibleContent>{children}</CollapsibleContent>
    </Collapsible>
  );
}

function StackAbilityView({ entry }) {
  return (
    <div className="grid gap-1.5 text-[13px] pr-1">
      <div className="font-bold text-[15px]">{entry.name}</div>
      <div className="text-muted-foreground italic">{entry.ability_kind} ability</div>
      {entry.ability_text && (
        <SymbolText text={entry.ability_text} className="text-[13px] text-[#c0d8f0] bg-[#0a1118] p-1.5 rounded font-[inherit] block" />
      )}
      {entry.effect_text && (
        <SymbolText text={entry.effect_text} className="text-[13px] text-[#a8c8e4] bg-[#0a1118] p-1.5 rounded font-[inherit] block" />
      )}
    </div>
  );
}

// SymbolText and ManaCostIcons imported from @/lib/mana-symbols

function CardView({ details }) {
  return (
    <div className="grid gap-1.5 content-start auto-rows-min text-[13px]">
      <SymbolText
        text={details.oracle_text || "No oracle text."}
        className="text-[13px] text-[#c0d8f0] m-0 font-[inherit] block"
      />

      {details.compiled_text && (
        <CollapsibleSection title="Compiled Text">
          <SymbolText
            text={details.compiled_text}
            className="text-[13px] text-[#a8c8e4] bg-[#0a1118] p-1.5 mt-0.5 rounded font-[inherit] block"
          />
        </CollapsibleSection>
      )}
    </div>
  );
}

const tabCls = "flex items-center gap-0.5 px-1 py-0.5 text-[11px] uppercase tracking-wider font-bold cursor-pointer rounded-sm transition-colors";
const tabActive = tabCls + " bg-[rgba(68,117,168,0.3)] text-foreground";
const tabInactive = tabCls + " text-muted-foreground hover:text-foreground";

export default function InspectorPanel({ selectedObjectId, pinnedObjectId }) {
  const { game, state } = useGame();
  const [details, setDetails] = useState(null);
  const [stackEntry, setStackEntry] = useState(null);
  const [view, setView] = useState("card"); // "card" | "abilities" | "raw"
  const cacheRef = useRef(new Map()); // id → { details, stackEntry }

  const toggleView = useCallback((v) => {
    setView((prev) => (prev === v ? "card" : v));
  }, []);

  // Reset view only when the user clicks a new object (not on hover changes)
  useEffect(() => {
    setView("card");
  }, [pinnedObjectId]);

  // Invalidate cache when game state changes (new actions resolved, etc.)
  const turnKey = `${state?.turn_number}|${state?.phase}|${state?.step}|${state?.stack_size}`;
  const prevTurnKeyRef = useRef(turnKey);
  if (prevTurnKeyRef.current !== turnKey) {
    prevTurnKeyRef.current = turnKey;
    cacheRef.current.clear();
  }

  const loadDetails = useCallback(
    async (id) => {
      if (!game || !id) {
        setDetails(null);
        setStackEntry(null);
        return;
      }

      const key = String(id);
      const cached = cacheRef.current.get(key);
      if (cached) {
        setDetails(cached.details);
        setStackEntry(cached.stackEntry);
        return;
      }

      // Try card details first (works for permanents, spells on stack, etc.)
      try {
        const d = await game.objectDetails(BigInt(id));
        if (d) {
          cacheRef.current.set(key, { details: d, stackEntry: null });
          setDetails(d);
          setStackEntry(null);
          return;
        }
      } catch (_) {
        // objectDetails failed — fall through to stack entry check
      }

      // Fall back to stack entry for ability-only stack objects
      const numId = Number(id);
      const stackObjects = state?.stack_objects || [];
      const entry = stackObjects.find((e) => Number(e.id) === numId);
      if (entry && entry.ability_kind) {
        cacheRef.current.set(key, { details: null, stackEntry: entry });
        setStackEntry(entry);
        setDetails(null);
        return;
      }

      setDetails(null);
      setStackEntry(null);
    },
    [game, state?.stack_objects]
  );

  useEffect(() => {
    loadDetails(selectedObjectId);
  }, [selectedObjectId, loadDetails]);

  const hasAbilities = details?.abilities?.length > 0;
  const hasRaw = !!details?.raw_compilation;
  const showTabs = details && (hasAbilities || hasRaw);

  const artUrl = details ? scryfallImageUrl(details.name, "art_crop") : null;

  const hasContent = details || stackEntry;

  return (
    <div className="h-full flex flex-col overflow-hidden relative">
      {/* Card art — bleeds into container, dissolves at edges */}
      {artUrl && (
        <div className="absolute inset-x-[-12px] top-[-6px] h-[160px] z-0 pointer-events-none">
          <img
            className="w-full h-full object-cover"
            style={{
              maskImage: "linear-gradient(to bottom, black 40%, transparent 88%), linear-gradient(to right, transparent 0%, black 12%, black 88%, transparent 100%)",
              maskComposite: "intersect",
              WebkitMaskImage: "linear-gradient(to bottom, black 40%, transparent 88%), linear-gradient(to right, transparent 0%, black 12%, black 88%, transparent 100%)",
              WebkitMaskComposite: "source-in",
            }}
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        </div>
      )}

      {/* Header: card name + mana cost */}
      {details && (
        <div className="relative z-1 shrink-0 min-w-0 px-1.5 pt-[110px] pb-1"
          style={{
            background: artUrl ? "linear-gradient(to bottom, transparent 0%, rgba(11,17,26,0.6) 30%, rgba(11,17,26,0.92) 60%)" : undefined,
          }}
        >
          <div className="flex items-start gap-1">
            <span className="font-bold text-[13px] leading-tight min-w-0 break-words flex-1">{details.name || "Unknown"}</span>
            {details.mana_cost && <span className="shrink-0 mt-px"><ManaCostIcons cost={details.mana_cost} /></span>}
          </div>
          {(() => {
            const counters = (details.counters || []).length
              ? details.counters.map((c) => `${c.amount} ${c.kind}`).join(" \u2022 ")
              : null;
            const meta = [details.type_line, details.zone,
              details.controller != null && `P${details.controller}`,
              details.tapped && "Tapped", counters].filter(Boolean);
            const pt = details.power != null && details.toughness != null
              ? `${details.power}/${details.toughness}` : null;
            return (meta.length > 0 || pt) ? (
              <div className="flex items-center mt-0.5">
                <span className="text-[11px] text-muted-foreground leading-tight break-words">{meta.join(" · ")}</span>
                {pt && <span className="ml-auto text-[#f5d08b] text-[13px] font-bold tracking-wide shrink-0">{pt}</span>}
              </div>
            ) : null;
          })()}
          {showTabs && (
            <div className="flex items-center gap-0.5 mt-1">
              {hasAbilities && (
                <button className={view === "abilities" ? tabActive : tabInactive} onClick={() => toggleView("abilities")}>
                  {view === "abilities" && <ChevronDown className="size-3" />}
                  Abilities
                </button>
              )}
              {hasRaw && (
                <button className={view === "raw" ? tabActive : tabInactive} onClick={() => toggleView("raw")}>
                  {view === "raw" && <ChevronDown className="size-3" />}
                  Raw
                </button>
              )}
            </div>
          )}
        </div>
      )}

      {/* Content */}
      <ScrollArea className="relative z-1 flex-1 min-h-0">
        <div className="px-1.5 pb-1.5 pt-1">
          {stackEntry ? (
            <StackAbilityView entry={stackEntry} />
          ) : details ? (
            view === "abilities" && hasAbilities ? (
              <div className="grid gap-1 content-start auto-rows-min text-[13px]">
                {details.abilities.map((ab, i) => (
                  <div key={i} className="bg-[#0a1118] rounded p-1">
                    <SymbolText text={typeof ab === "string" ? ab : ab.text || ab.kind || `Ability ${i + 1}`} />
                  </div>
                ))}
              </div>
            ) : view === "raw" && hasRaw ? (
              <SymbolText text={details.raw_compilation} className="text-[12px] text-[#8a9eb8] font-[inherit] block" />
            ) : (
              <CardView details={details} />
            )
          ) : (
            <div className="text-muted-foreground text-[13px] italic p-1">
              Hover or click to inspect
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
