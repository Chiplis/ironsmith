import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { ChevronUp, ChevronDown } from "lucide-react";
import { SymbolText } from "@/lib/mana-symbols";
import { normalizeDecisionText } from "./decisionText";

function OptionButton({ opt, canAct, onClick, isHighlighted, onMouseEnter, onMouseLeave }) {
  const disabled = !canAct || opt.legal === false;

  return (
    <Button
      variant="ghost"
      size="sm"
      className={
        "h-auto min-h-7 py-1 text-[13px] justify-start px-2 whitespace-normal text-left text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.1)] hover:shadow-[0_0_8px_rgba(100,169,255,0.15)]" +
        (isHighlighted ? " text-foreground bg-[rgba(240,206,97,0.08)] shadow-[0_0_10px_rgba(240,206,97,0.25)]" : "")
      }
      disabled={disabled}
      onPointerDown={(e) => {
        if (disabled || e.button !== 0) return;
        // Trigger as early as possible so option picks are not lost to
        // document-level pointerup handlers used by hand-drag interactions.
        e.preventDefault();
        onClick?.();
      }}
      onClick={(e) => {
        // Keep keyboard activation working while avoiding double-dispatch
        // after pointerdown-triggered selection.
        if (disabled || e.detail !== 0) return;
        onClick?.();
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <SymbolText text={normalizeDecisionText(opt.description)} />
    </Button>
  );
}

function SubmitButton({ canAct, disabled, onClick, children }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className="group h-auto min-h-7 py-1.5 text-[14px] font-bold justify-start px-3 whitespace-normal text-left transition-all duration-200 shrink-0"
      style={{
        color: "#8ec4ff",
        border: "1px solid rgba(142,196,255,0.35)",
        boxShadow: "0 0 6px 1px rgba(142,196,255,0.2), 0 0 14px 3px rgba(142,196,255,0.08)",
      }}
      disabled={!canAct || disabled}
      onClick={onClick}
    >
      <span className="inline-block transition-transform duration-200 group-hover:translate-x-0.5">
        {children}
      </span>
    </Button>
  );
}

function SectionHeader({ text }) {
  return (
    <h4 className="text-[12px] uppercase tracking-wider text-[#8ec4ff] font-bold px-1 py-0.5 m-0">
      {text}
    </h4>
  );
}

function Description({ text }) {
  if (!text) return null;
  return (
    <div className="text-[13px] text-muted-foreground px-1 leading-snug">
      <SymbolText text={normalizeDecisionText(text)} />
    </div>
  );
}

export default function SelectOptionsDecision({ decision, canAct }) {
  const reason = (decision.reason || "").toLowerCase();

  // Dispatch to sub-type based on decision metadata
  if (reason.includes("order")) {
    return <OrderingDecision decision={decision} canAct={canAct} />;
  }
  if (decision.distribute || reason.includes("distribut")) {
    return <DistributeDecision decision={decision} canAct={canAct} />;
  }
  if (decision.counter_type || reason.includes("counter")) {
    return <CountersDecision decision={decision} canAct={canAct} />;
  }
  const hasRepeatableOption = (decision.options || []).some((opt) => opt.repeatable);
  if (decision.repeatable || hasRepeatableOption) {
    return <RepeatableDecision decision={decision} canAct={canAct} />;
  }

  const min = decision.min ?? 1;
  const max = decision.max ?? 1;

  if (min === 1 && max === 1) {
    return <SingleSelectDecision decision={decision} canAct={canAct} />;
  }

  return <MultiSelectDecision decision={decision} canAct={canAct} />;
}

function SingleSelectDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const options = decision.options || [];

  return (
    <div className="flex h-full min-h-0 flex-col gap-1">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <Description text={decision.description} />
          <div className="flex flex-col gap-0.5">
            {options.map((opt) => {
              const objId = opt.object_id != null ? String(opt.object_id) : null;
              return (
                <OptionButton
                  key={opt.index}
                  opt={opt}
                  canAct={canAct}
                  isHighlighted={objId != null && hoveredObjectId === objId}
                  onClick={() =>
                    dispatch(
                      { type: "select_options", option_indices: [opt.index] },
                      opt.description
                    )
                  }
                  onMouseEnter={() => objId && hoverCard(objId)}
                  onMouseLeave={clearHover}
                />
              );
            })}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

function MultiSelectDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const options = decision.options || [];
  const [selected, setSelected] = useState(new Set());
  const min = decision.min ?? 0;
  const max = decision.max ?? options.length;

  const toggle = (index) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else if (next.size < max) next.add(index);
      return next;
    });
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-1">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <Description text={decision.description} />
          <SectionHeader text={`Select ${min === max ? min : `${min}–${max}`}`} />
          <div className="flex flex-col gap-0.5">
            {options.map((opt) => {
              const objId = opt.object_id != null ? String(opt.object_id) : null;
              const isHighlighted = objId != null && hoveredObjectId === objId;
              const isSelected = selected.has(opt.index);
              return (
                <label
                  key={opt.index}
                  className={`flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm cursor-pointer transition-all ${
                    opt.legal !== false ? "text-muted-foreground hover:text-foreground hover:bg-[rgba(100,169,255,0.1)] hover:shadow-[0_0_8px_rgba(100,169,255,0.15)]" : "opacity-50"
                  } ${isSelected ? "text-foreground bg-[rgba(100,169,255,0.08)] shadow-[0_0_6px_rgba(100,169,255,0.2)]" : ""}${
                    isHighlighted ? " text-foreground bg-[rgba(240,206,97,0.08)] shadow-[0_0_10px_rgba(240,206,97,0.25)]" : ""
                  }`}
                  onMouseEnter={() => objId && hoverCard(objId)}
                  onMouseLeave={clearHover}
                >
                  <Checkbox
                    checked={isSelected}
                    onCheckedChange={() => opt.legal !== false && toggle(opt.index)}
                    disabled={!canAct || opt.legal === false}
                    className="h-3.5 w-3.5"
                  />
                  <SymbolText text={normalizeDecisionText(opt.description)} />
                </label>
              );
            })}
          </div>
        </div>
      </ScrollArea>
      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <SubmitButton
          canAct={canAct}
          disabled={selected.size < min || selected.size > max}
          onClick={() =>
            dispatch(
              { type: "select_options", option_indices: Array.from(selected) },
              `Selected ${selected.size} option(s)`
            )
          }
        >
          Submit ({selected.size})
        </SubmitButton>
      </div>
    </div>
  );
}

function OrderingDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const [order, setOrder] = useState(
    () => (decision.options || []).map((opt) => opt.index)
  );
  const options = decision.options || [];

  const move = (position, direction) => {
    const newPos = position + direction;
    if (newPos < 0 || newPos >= order.length) return;
    setOrder((prev) => {
      const next = [...prev];
      [next[position], next[newPos]] = [next[newPos], next[position]];
      return next;
    });
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-1">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <Description text={decision.description} />
          <SectionHeader text="Order" />
          <div className="flex flex-col gap-0.5">
            {order.map((optIndex, pos) => {
              const opt = options.find((o) => o.index === optIndex);
              if (!opt) return null;
              return (
                <div key={optIndex} className="flex items-center gap-1.5 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]">
                  <span className="text-[11px] text-[#8ec4ff] font-bold w-4 text-center shrink-0">{pos + 1}</span>
                  <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 w-5 p-0 text-[13px]"
                    disabled={!canAct || pos === 0}
                    onClick={() => move(pos, -1)}
                  >
                    <ChevronUp className="size-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 w-5 p-0 text-[13px]"
                    disabled={!canAct || pos === order.length - 1}
                    onClick={() => move(pos, 1)}
                  >
                    <ChevronDown className="size-3.5" />
                  </Button>
                </div>
              );
            })}
          </div>
        </div>
      </ScrollArea>
      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <SubmitButton
          canAct={canAct}
          onClick={() =>
            dispatch({ type: "select_options", option_indices: order.slice() }, "Order submitted")
          }
        >
          Submit Order
        </SubmitButton>
      </div>
    </div>
  );
}

function DistributeDecision({ decision, canAct }) {
  const { dispatch, setStatus } = useGame();
  const options = decision.options || [];
  const total = Number(decision.max || 0);
  const [counts, setCounts] = useState(() =>
    Object.fromEntries(options.map((opt) => [opt.index, 0]))
  );

  const assigned = Object.values(counts).reduce((a, b) => a + b, 0);

  const expandOptionCounts = (countsByIndex) => {
    const expanded = [];
    Object.entries(countsByIndex).forEach(([idx, count]) => {
      for (let i = 0; i < Math.max(0, Math.floor(Number(count) || 0)); i++) {
        expanded.push(Number(idx));
      }
    });
    return expanded;
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-1">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <Description text={decision.description} />
          <SectionHeader text={`Distribute ${total} total`} />
          <div className="flex flex-col gap-0.5">
            {options.map((opt) => (
              <label key={opt.index} className="flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]">
                <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
                <Input
                  type="number"
                  className="h-6 w-16 text-[13px] bg-transparent text-center"
                  min={0}
                  max={Number(opt.max_count ?? total)}
                  value={counts[opt.index] || 0}
                  onChange={(e) =>
                    setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
                  }
                  disabled={!canAct || opt.legal === false}
                />
              </label>
            ))}
          </div>
        </div>
      </ScrollArea>
      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <SubmitButton
          canAct={canAct}
          disabled={assigned !== total}
          onClick={() => {
            if (assigned !== total) {
              setStatus(`Must assign exactly ${total} (currently ${assigned})`, true);
              return;
            }
            dispatch(
              { type: "select_options", option_indices: expandOptionCounts(counts) },
              "Distribution submitted"
            );
          }}
        >
          Submit ({assigned}/{total})
        </SubmitButton>
      </div>
    </div>
  );
}

function CountersDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const options = decision.options || [];
  const maxTotal = Number(decision.max || 0);
  const [counts, setCounts] = useState(() =>
    Object.fromEntries(options.map((opt) => [opt.index, 0]))
  );

  const total = Object.values(counts).reduce((a, b) => a + b, 0);

  const expandOptionCounts = (countsByIndex) => {
    const expanded = [];
    Object.entries(countsByIndex).forEach(([idx, count]) => {
      for (let i = 0; i < Math.max(0, Math.floor(Number(count) || 0)); i++) {
        expanded.push(Number(idx));
      }
    });
    return expanded;
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-1">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <Description text={decision.description} />
          <SectionHeader text="Counters" />
          <div className="flex flex-col gap-0.5">
            {options.map((opt) => (
              <label key={opt.index} className="flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]">
                <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
                <Input
                  type="number"
                  className="h-6 w-16 text-[13px] bg-transparent text-center"
                  min={0}
                  max={Number(opt.max_count ?? maxTotal)}
                  value={counts[opt.index] || 0}
                  onChange={(e) =>
                    setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
                  }
                  disabled={!canAct || opt.legal === false}
                />
              </label>
            ))}
          </div>
        </div>
      </ScrollArea>
      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <SubmitButton
          canAct={canAct}
          disabled={total > maxTotal}
          onClick={() =>
            dispatch(
              { type: "select_options", option_indices: expandOptionCounts(counts) },
              "Counter choice submitted"
            )
          }
        >
          Submit Counters ({total}/{maxTotal})
        </SubmitButton>
      </div>
    </div>
  );
}

function RepeatableDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const options = decision.options || [];
  const maxTotal = Number(decision.max || 0);
  const [counts, setCounts] = useState(() =>
    Object.fromEntries(options.map((opt) => [opt.index, 0]))
  );

  const total = Object.values(counts).reduce((a, b) => a + b, 0);

  const expandOptionCounts = (countsByIndex) => {
    const expanded = [];
    Object.entries(countsByIndex).forEach(([idx, count]) => {
      for (let i = 0; i < Math.max(0, Math.floor(Number(count) || 0)); i++) {
        expanded.push(Number(idx));
      }
    });
    return expanded;
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-1">
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <Description text={decision.description} />
          <SectionHeader text="Repeat" />
          <div className="flex flex-col gap-0.5">
            {options.map((opt) => (
              <label key={opt.index} className="flex items-center gap-2 text-[13px] py-1 px-2 rounded-sm text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.06)]">
                <span className="flex-1 min-w-0"><SymbolText text={normalizeDecisionText(opt.description)} /></span>
                <Input
                  type="number"
                  className="h-6 w-16 text-[13px] bg-transparent text-center"
                  min={0}
                  max={Number(opt.max_count ?? maxTotal)}
                  value={counts[opt.index] || 0}
                  onChange={(e) =>
                    setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
                  }
                  disabled={!canAct || opt.legal === false}
                />
              </label>
            ))}
          </div>
        </div>
      </ScrollArea>
      <div className="shrink-0 border-t border-game-line-2/70 pt-1">
        <SubmitButton
          canAct={canAct}
          disabled={total < (decision.min || 0) || total > maxTotal}
          onClick={() =>
            dispatch(
              { type: "select_options", option_indices: expandOptionCounts(counts) },
              `Selected ${total} option(s)`
            )
          }
        >
          Submit ({total})
        </SubmitButton>
      </div>
    </div>
  );
}
