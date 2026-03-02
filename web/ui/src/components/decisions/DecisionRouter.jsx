import PriorityDecision from "./PriorityDecision";
import TargetsDecision from "./TargetsDecision";
import AttackersDecision from "./AttackersDecision";
import BlockersDecision from "./BlockersDecision";
import SelectObjectsDecision from "./SelectObjectsDecision";
import SelectOptionsDecision from "./SelectOptionsDecision";
import NumberDecision from "./NumberDecision";

export default function DecisionRouter({ decision, canAct }) {
  if (!decision) return null;

  switch (decision.kind) {
    case "priority":
      return <PriorityDecision decision={decision} canAct={canAct} />;
    case "targets":
      return <TargetsDecision decision={decision} canAct={canAct} />;
    case "attackers":
      return <AttackersDecision decision={decision} canAct={canAct} />;
    case "blockers":
      return <BlockersDecision decision={decision} canAct={canAct} />;
    case "select_objects":
      return <SelectObjectsDecision decision={decision} canAct={canAct} />;
    case "select_options":
      return <SelectOptionsDecision decision={decision} canAct={canAct} />;
    case "number":
      return <NumberDecision decision={decision} canAct={canAct} />;
    default:
      return (
        <div className="text-muted-foreground text-[12px] italic p-2">
          Unknown decision type: {decision.kind}
        </div>
      );
  }
}
