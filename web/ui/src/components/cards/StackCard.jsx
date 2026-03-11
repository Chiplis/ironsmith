import { useEffect, useLayoutEffect, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import { getPlayerAccent, playerAccentVars } from "@/lib/player-colors";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";
import AnimatedCircuitFrame from "@/components/cards/AnimatedCircuitFrame";
import { ArrowLeft, ArrowRight } from "lucide-react";

const STACK_CARD_CIRCUIT_PATH = "M9.5 2.5H90.5 M9.5 93.5H90.5";

export default function StackCard({
  entry,
  isNew = false,
  isActive = false,
  isLeaving = false,
  className = "",
  onClick,
  reorderControls = null,
}) {
  const { state } = useGame();
  const name = entry.name || `Object#${entry.id}`;
  const artUrl = scryfallImageUrl(name, "art_crop");
  const scryfallUrl = scryfallImageUrl(name);
  const isCastEntry = !entry.ability_kind;
  const kindLabel = isCastEntry
    ? "Spell"
    : `${entry.ability_kind || "Ability"} ability`;
  const pt = entry.power_toughness
    || (entry.power != null && entry.toughness != null
      ? `${entry.power}/${entry.toughness}`
      : null);
  const stackAccent = getPlayerAccent(state?.players || [], entry?.controller);
  const stackAccentStyle = stackAccent
    ? {
      ...playerAccentVars(stackAccent),
      "--glow-rgb": stackAccent.rgb,
    }
    : undefined;
  const subtitle = String(entry?.__subtitle || "").trim();
  const hasReorderControls = !!reorderControls;
  const rootRef = useRef(null);
  const motionRef = useRef(null);

  useLayoutEffect(() => {
    const node = rootRef.current;
    if (!node) return undefined;

    cancelMotion(motionRef.current);
    motionRef.current = null;
    node.style.opacity = "";
    node.style.transform = "";

    if (isLeaving) {
      motionRef.current = createTimeline({ autoplay: true }).add(node, {
        opacity: [1, 0],
        y: [0, -14],
        scale: [1, 0.97],
        duration: 360,
        ease: "out(2)",
      });
    } else if (isNew) {
      motionRef.current = createTimeline({ autoplay: true }).add(node, {
        opacity: [0, 1],
        y: [18, 0],
        scale: [0.92, 1],
        duration: 380,
        ease: uiSpring({ duration: 380, bounce: 0.18 }),
        onComplete: () => {
          node.style.opacity = "";
          node.style.transform = "";
        },
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
        "game-card stack-card stack-card-circuit w-full min-w-0 min-h-[96px] overflow-hidden",
        onClick ? "cursor-pointer" : "cursor-default",
        isActive && "stack-card-active",
        hasReorderControls && "stack-card-reorderable",
        isLeaving && "pointer-events-none",
        className
      )}
      data-object-id={entry.id}
      data-card-name={name}
      onClick={() => onClick?.(entry.inspect_object_id ?? entry.id, {
        source: "stack",
        stackEntry: entry,
      })}
      style={stackAccentStyle}
    >
      {!isLeaving && (
        <AnimatedCircuitFrame
          seed={`${entry.id}:${entry.controller}:${name}`}
          path={STACK_CARD_CIRCUIT_PATH}
          viewBox="0 0 100 96"
          overlayClassName="stack-circuit-overlay"
        />
      )}

      {scryfallUrl && !hasReorderControls && (
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

      {hasReorderControls && (
        <>
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-left"
            disabled={!reorderControls.canMoveLeft}
            onClick={(event) => {
              event.stopPropagation();
              reorderControls.onMoveLeft?.();
            }}
            aria-label={reorderControls.leftLabel || `Move ${name} toward the top of the stack`}
            title={reorderControls.leftTitle || "Move toward the top of the stack"}
          >
            <ArrowLeft className="size-3.5" />
          </button>
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-right"
            disabled={!reorderControls.canMoveRight}
            onClick={(event) => {
              event.stopPropagation();
              reorderControls.onMoveRight?.();
            }}
            aria-label={reorderControls.rightLabel || `Move ${name} toward the bottom of the stack`}
            title={reorderControls.rightTitle || "Move toward the bottom of the stack"}
          >
            <ArrowRight className="size-3.5" />
          </button>
        </>
      )}

      <div className={cn(
        "stack-card-body relative z-2 flex min-h-[96px] flex-col py-2",
        hasReorderControls ? "px-12" : "px-2.5"
      )}>
        <div className="flex items-start gap-2">
          <div className="relative h-7 w-7 shrink-0 overflow-hidden rounded-md bg-[#0b121b]">
            {artUrl && (
              <img
                className="h-full w-full object-cover opacity-90"
                src={artUrl}
                alt=""
                loading="lazy"
                referrerPolicy="no-referrer"
              />
            )}
          </div>
          <div className="min-w-0 flex-1">
            <div className="stack-card-title break-words leading-[1.08] text-[#edf5ff]">
              {name}
            </div>
            <div className="mt-1 flex items-center gap-2 text-[11px] uppercase tracking-[0.12em] text-[#8ec4ff]">
              <span>{kindLabel}</span>
            </div>
            {subtitle && (
              <div className="stack-card-effect mt-1 text-[11px] font-semibold uppercase leading-[1.2] tracking-[0.08em] text-[#91cdfc]">
                {subtitle}
              </div>
            )}
          </div>
          {isCastEntry && entry.mana_cost && (
            <span className="shrink-0 pt-0.5">
              <ManaCostIcons cost={entry.mana_cost} />
            </span>
          )}
        </div>
        <div className="mt-auto flex items-center gap-2 pt-2">
          {pt && (
            <span className="shrink-0 rounded-sm border border-[#f5d08b]/35 bg-[rgba(245,208,139,0.08)] px-1.5 py-0.5 text-[12px] font-bold leading-none tracking-wide text-[#f5d08b]">
              {pt}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
