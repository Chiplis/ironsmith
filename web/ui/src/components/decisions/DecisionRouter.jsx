import PriorityDecision from "./PriorityDecision";
import TargetsDecision from "./TargetsDecision";
import AttackersDecision from "./AttackersDecision";
import BlockersDecision from "./BlockersDecision";
import SelectObjectsDecision from "./SelectObjectsDecision";
import SelectOptionsDecision from "./SelectOptionsDecision";
import NumberDecision from "./NumberDecision";

/** Derive a stable key so React remounts stateful decision components when the
 *  underlying decision changes (e.g. "discard a card" → "search library"). */
function decisionKey(decision) {
  if (decision.candidates) {
    return decision.candidates.map((c) => c.id).join(",");
  }
  if (decision.options) {
    return decision.options.map((o) => `${o.index}:${o.description}`).join(",");
  }
  if (decision.requirements) {
    return decision.requirements
      .map((r) =>
        (r.legal_targets || [])
          .map((t) => (t.kind === "player" ? `p${t.player}` : `o${t.object}`))
          .join("+")
      )
      .join(",");
  }
  return decision.description || "";
}

export default function DecisionRouter({ decision, canAct }) {
  if (!decision) return null;

  const key = decisionKey(decision);

  switch (decision.kind) {
    case "priority":
      return <PriorityDecision decision={decision} canAct={canAct} />;
    case "targets":
      return <TargetsDecision key={key} decision={decision} canAct={canAct} />;
    case "attackers":
      return <AttackersDecision key={key} decision={decision} canAct={canAct} />;
    case "blockers":
      return <BlockersDecision key={key} decision={decision} canAct={canAct} />;
    case "select_objects":
      return <SelectObjectsDecision key={key} decision={decision} canAct={canAct} />;
    case "select_options":
      return <SelectOptionsDecision key={key} decision={decision} canAct={canAct} />;
    case "number":
      return <NumberDecision key={key} decision={decision} canAct={canAct} />;
    default:
      return (
        <div className="text-muted-foreground text-[16px] italic p-2">
          Unknown decision type: {decision.kind}
        </div>
      );
  }
}
