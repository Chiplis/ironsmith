import { useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { ChevronUp, ChevronDown } from "lucide-react";

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
  // Route to RepeatableDecision if any individual option is repeatable.
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
  const options = decision.options || [];

  return (
    <div className="flex flex-col gap-1">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground mb-1">{decision.description}</div>
      )}
      {options.map((opt) => (
        <Button
          key={opt.index}
          variant="outline"
          size="sm"
          className="h-7 text-[14px] justify-start px-2 whitespace-normal text-left"
          disabled={!canAct || !opt.legal}
          onClick={() =>
            dispatch(
              { type: "select_options", option_indices: [opt.index] },
              opt.description
            )
          }
        >
          {opt.description}
        </Button>
      ))}
    </div>
  );
}

function MultiSelectDecision({ decision, canAct }) {
  const { dispatch } = useGame();
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
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      <div className="text-[14px] text-muted-foreground">
        Select {min === max ? min : `${min}-${max}`}
      </div>
      <div className="flex flex-col gap-1">
        {options.map((opt) => (
          <label
            key={opt.index}
            className={`flex items-center gap-2 text-[14px] p-1 border rounded-sm cursor-pointer ${
              opt.legal ? "border-game-line-2 hover:border-game-line" : "opacity-50"
            } ${selected.has(opt.index) ? "border-primary bg-primary/10" : ""}`}
          >
            <Checkbox
              checked={selected.has(opt.index)}
              onCheckedChange={() => opt.legal && toggle(opt.index)}
              disabled={!canAct || !opt.legal}
              className="h-3.5 w-3.5"
            />
            {opt.description}
          </label>
        ))}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
        disabled={!canAct || selected.size < min || selected.size > max}
        onClick={() =>
          dispatch(
            { type: "select_options", option_indices: Array.from(selected) },
            `Selected ${selected.size} option(s)`
          )
        }
      >
        Submit ({selected.size})
      </Button>
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
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      <div className="flex flex-col gap-1">
        {order.map((optIndex, pos) => {
          const opt = options.find((o) => o.index === optIndex);
          if (!opt) return null;
          return (
            <div key={optIndex} className="flex items-center gap-1.5 text-[14px] p-1 border border-game-line-2 rounded-sm">
              <span className="flex-1">{pos + 1}. {opt.description}</span>
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
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
        disabled={!canAct}
        onClick={() =>
          dispatch({ type: "select_options", option_indices: order.slice() }, "Order submitted")
        }
      >
        Submit Order
      </Button>
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
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      <div className="text-[14px] text-muted-foreground">
        Distribute {total} total
      </div>
      <div className="flex flex-col gap-1">
        {options.map((opt) => (
          <label key={opt.index} className="flex items-center gap-2 text-[14px] p-1 border border-game-line-2 rounded-sm">
            <span className="flex-1">{opt.description}</span>
            <Input
              type="number"
              className="h-6 w-14 text-[14px] bg-transparent text-center"
              min={0}
              max={Number(opt.max_count ?? total)}
              value={counts[opt.index] || 0}
              onChange={(e) =>
                setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
              }
              disabled={!canAct || !opt.legal}
            />
          </label>
        ))}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
        disabled={!canAct || assigned !== total}
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
      </Button>
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
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      <div className="flex flex-col gap-1">
        {options.map((opt) => (
          <label key={opt.index} className="flex items-center gap-2 text-[14px] p-1 border border-game-line-2 rounded-sm">
            <span className="flex-1">{opt.description}</span>
            <Input
              type="number"
              className="h-6 w-14 text-[14px] bg-transparent text-center"
              min={0}
              max={Number(opt.max_count ?? maxTotal)}
              value={counts[opt.index] || 0}
              onChange={(e) =>
                setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
              }
              disabled={!canAct || !opt.legal}
            />
          </label>
        ))}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
        disabled={!canAct || total > maxTotal}
        onClick={() =>
          dispatch(
            { type: "select_options", option_indices: expandOptionCounts(counts) },
            "Counter choice submitted"
          )
        }
      >
        Submit Counters ({total}/{maxTotal})
      </Button>
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
    <div className="flex flex-col gap-2">
      {decision.description && (
        <div className="text-[16px] text-muted-foreground">{decision.description}</div>
      )}
      <div className="flex flex-col gap-1">
        {options.map((opt) => (
          <label key={opt.index} className="flex items-center gap-2 text-[14px] p-1 border border-game-line-2 rounded-sm">
            <span className="flex-1">{opt.description}</span>
            <Input
              type="number"
              className="h-6 w-14 text-[14px] bg-transparent text-center"
              min={0}
              max={Number(opt.max_count ?? maxTotal)}
              value={counts[opt.index] || 0}
              onChange={(e) =>
                setCounts((prev) => ({ ...prev, [opt.index]: Number(e.target.value) || 0 }))
              }
              disabled={!canAct || !opt.legal}
            />
          </label>
        ))}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
        disabled={!canAct || total < (decision.min || 0) || total > maxTotal}
        onClick={() =>
          dispatch(
            { type: "select_options", option_indices: expandOptionCounts(counts) },
            `Selected ${total} option(s)`
          )
        }
      >
        Submit ({total})
      </Button>
    </div>
  );
}
