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
  const metaKey = `|${decision.description || ""}|${decision.context_text || ""}|${decision.consequence_text || ""}`;
  if (decision.attacker_options) {
    return decision.attacker_options
      .map((opt) => {
        const targets = (opt.valid_targets || [])
          .map((target) => JSON.stringify(target))
          .join("+");
        return `${Number(opt.creature)}:${opt.must_attack ? 1 : 0}:${targets}`;
      })
      .join("|") + metaKey;
  }
  if (decision.blocker_options) {
    return decision.blocker_options
      .map((opt) => {
        const blockers = (opt.valid_blockers || [])
          .map((blocker) => `${Number(blocker.id)}:${blocker.name || ""}`)
          .join("+");
        return `${Number(opt.attacker)}:${opt.min_blockers || 0}:${blockers}`;
      })
      .join("|") + metaKey;
  }
  if (decision.candidates) {
    return decision.candidates.map((c) => c.id).join(",") + metaKey;
  }
  if (decision.options) {
    return decision.options.map((o) => `${o.index}:${o.description}`).join(",") + metaKey;
  }
  if (decision.requirements) {
    return decision.requirements
      .map((r) =>
        (r.legal_targets || [])
          .map((t) => (t.kind === "player" ? `p${t.player}` : `o${t.object}`))
          .join("+")
      )
      .join(",") + metaKey;
  }
  return metaKey;
}

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
}) {
  if (!decision) return null;

  const key = decisionKey(decision);

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
      return <AttackersDecision key={key} decision={decision} canAct={canAct} compact={combatInline} />;
    case "blockers":
      return <BlockersDecision key={key} decision={decision} canAct={canAct} compact={combatInline} />;
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
    default:
      return (
        <div className="text-muted-foreground text-[16px] italic p-2">
          Unknown decision type: {decision.kind}
        </div>
      );
  }
}
