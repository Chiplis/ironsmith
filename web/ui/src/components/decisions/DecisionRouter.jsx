import PriorityDecision from "./PriorityDecision";
import TargetsDecision from "./TargetsDecision";
import AttackersDecision from "./AttackersDecision";
import BlockersDecision from "./BlockersDecision";
import SelectObjectsDecision from "./SelectObjectsDecision";
import SelectOptionsDecision from "./SelectOptionsDecision";
import NumberDecision from "./NumberDecision";
import TextInputDecision from "./TextInputDecision";
import { useGame } from "@/context/GameContext";
import { decisionKey } from "@/lib/decision-key";

export default function DecisionRouter({
  decision,
  canAct,
  selectedObjectId = null,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  combatInline = false,
  layout = "panel",
  showStripSummary = true,
  onCombatActionChange = null,
}) {
  const { state } = useGame();
  if (!decision) return null;

  const key = decisionKey(decision);
  const combatKey = `${key}|${state?.snapshot_id ?? ""}`;

  switch (decision.kind) {
    case "priority":
      return <PriorityDecision decision={decision} canAct={canAct} />;
    case "targets":
      return (
        <TargetsDecision
          key={key}
          decision={decision}
          canAct={canAct}
          inspectorOracleTextHeight={inspectorOracleTextHeight}
          inlineSubmit={inlineSubmit}
          onSubmitActionChange={onSubmitActionChange}
          hideDescription={hideDescription}
          layout={layout}
          showStripSummary={showStripSummary}
        />
      );
    case "attackers":
      return (
        <AttackersDecision
          key={combatKey}
          decision={decision}
          canAct={canAct}
          compact={combatInline}
          onCompactActionChange={onCombatActionChange}
        />
      );
    case "blockers":
      return (
        <BlockersDecision
          key={combatKey}
          decision={decision}
          canAct={canAct}
          compact={combatInline}
          onCompactActionChange={onCombatActionChange}
        />
      );
    case "select_objects":
      return (
        <SelectObjectsDecision
          key={key}
          decision={decision}
          canAct={canAct}
          inspectorOracleTextHeight={inspectorOracleTextHeight}
          inlineSubmit={inlineSubmit}
          onSubmitActionChange={onSubmitActionChange}
          hideDescription={hideDescription}
          layout={layout}
        />
      );
    case "select_options":
      return (
        <SelectOptionsDecision
          key={key}
          decision={decision}
          canAct={canAct}
          selectedObjectId={selectedObjectId}
          inspectorOracleTextHeight={inspectorOracleTextHeight}
          inlineSubmit={inlineSubmit}
          onSubmitActionChange={onSubmitActionChange}
          hideDescription={hideDescription}
          layout={layout}
        />
      );
    case "number":
      return (
        <NumberDecision
          key={key}
          decision={decision}
          canAct={canAct}
          inlineSubmit={inlineSubmit}
          onSubmitActionChange={onSubmitActionChange}
          hideDescription={hideDescription}
          layout={layout}
        />
      );
    case "text_input":
      return (
        <TextInputDecision
          key={key}
          decision={decision}
          canAct={canAct}
          inlineSubmit={inlineSubmit}
          onSubmitActionChange={onSubmitActionChange}
          hideDescription={hideDescription}
          layout={layout}
        />
      );
    default:
      return (
        <div className="text-muted-foreground text-[16px] italic p-2">
          Unknown decision type: {decision.kind}
        </div>
      );
  }
}
