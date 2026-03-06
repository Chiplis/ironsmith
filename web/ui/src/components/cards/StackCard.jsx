import { useEffect, useLayoutEffect, useRef } from "react";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";

export default function StackCard({
  entry,
  isNew = false,
  isActive = false,
  isLeaving = false,
  className = "",
  onClick,
}) {
  const name = entry.name || `Object#${entry.id}`;
  const artUrl = scryfallImageUrl(name, "art_crop");
  const scryfallUrl = scryfallImageUrl(name);
  const isCastEntry = !entry.ability_kind;
  const pt = entry.power_toughness
    || (entry.power != null && entry.toughness != null
      ? `${entry.power}/${entry.toughness}`
      : null);
  const rootRef = useRef(null);
  const motionRef = useRef(null);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node) return undefined;

    cancelMotion(motionRef.current);
    motionRef.current = null;

    if (isLeaving) {
      motionRef.current = animate(node, {
        opacity: [1, 0],
        y: [0, -14],
        scale: [1, 0.97],
        duration: 360,
        ease: "out(2)",
      });
    } else if (isNew) {
      motionRef.current = animate(node, {
        opacity: [0, 1],
        y: [18, 0],
        scale: [0.92, 1],
        duration: 380,
        ease: uiSpring({ duration: 380, bounce: 0.18 }),
      });
    }
  }, [isLeaving, isNew]);

  useEffect(() => () => {
    cancelMotion(motionRef.current);
    motionRef.current = null;
  }, []);

  return (
    <div
      ref={rootRef}
      className={cn(
        "game-card stack-card w-full min-w-0 min-h-[96px] cursor-pointer overflow-hidden",
        isActive && "ring-1 ring-[#8ec4ff] shadow-[0_0_0_1px_rgba(142,196,255,0.5),0_10px_22px_rgba(0,0,0,0.46)]",
        isLeaving && "pointer-events-none",
        className
      )}
      data-object-id={entry.id}
      data-card-name={name}
      onClick={() => onClick?.(entry.id)}
    >
      {artUrl && (
        <div className="stack-card-art absolute inset-y-0 left-0 z-0 w-[102px] overflow-hidden pointer-events-none">
          <img
            className="h-full w-full object-cover opacity-88 saturate-[1.08] contrast-[1.06]"
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        </div>
      )}

      {scryfallUrl && (
        <a
          className="absolute top-1 right-1 bg-[#0a1118] text-[#9ec3ea] no-underline uppercase text-[12px] tracking-wide px-1 py-px rounded-sm leading-tight z-2 opacity-0 hover:opacity-100 transition-opacity"
          href={scryfallUrl}
          target="_blank"
          rel="noopener noreferrer"
          onClick={(e) => e.stopPropagation()}
        >
          img
        </a>
      )}

      <div className="stack-card-body relative z-2 ml-[102px] flex min-h-[96px] flex-col px-2.5 py-2">
        <div className="flex items-start gap-2">
          <div className="min-w-0 flex-1">
            <div className="stack-card-title break-words leading-[1.08] text-[#edf5ff]">
              {name}
            </div>
            <div className="mt-1 flex items-center gap-2 text-[11px] uppercase tracking-[0.12em] text-[#8ec4ff]">
              <span>{isCastEntry ? "Spell" : "Ability"}</span>
              {entry.ability_kind && (
                <span className="text-[#c6dbf2]/80 normal-case tracking-normal">
                  {entry.ability_kind}
                </span>
              )}
            </div>
          </div>
          {isCastEntry && entry.mana_cost && (
            <span className="shrink-0 pt-0.5">
              <ManaCostIcons cost={entry.mana_cost} />
            </span>
          )}
        </div>
        <div className="mt-2 flex items-center gap-2">
          {pt && (
            <span className="shrink-0 rounded-sm border border-[#f5d08b]/35 bg-[rgba(245,208,139,0.08)] px-1.5 py-0.5 text-[12px] font-bold leading-none tracking-wide text-[#f5d08b]">
              {pt}
            </span>
          )}
          {entry.effect_text && !entry.ability_kind && (
            <span className="text-[11px] uppercase tracking-[0.12em] text-[#7fa8d2]">
              Effect
            </span>
          )}
        </div>

        {entry.ability_kind ? (
          <span className="stack-card-effect mt-2 block text-[12px] leading-[1.32] text-[#d4e5f7]">
            {entry.effect_text || `${entry.ability_kind} ability`}
          </span>
        ) : entry.effect_text ? (
          <span className="stack-card-effect mt-2 block text-[12px] leading-[1.32] text-[#d4e5f7]">
            {entry.effect_text}
          </span>
        ) : (
          <div className="mt-auto" />
        )}

        {!entry.effect_text && isCastEntry && (
          <div className="mt-auto pt-2 text-[11px] uppercase tracking-[0.12em] text-[#7fa8d2]">
            Waiting to resolve
          </div>
        )}
      </div>
    </div>
  );
}
