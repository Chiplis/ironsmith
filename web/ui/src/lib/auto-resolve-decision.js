import { isOrderingDecision, isPaymentDecision, isSpellCastFlowDecision } from "./decision-semantics.js";

export function tryBuildAutoResolveCommand(decision) {
  if (!decision) return null;

  // No legal blocker means there is no assignment for the player to make.
  if (decision.kind === "blockers" && Array.isArray(decision.blocker_options)
      && decision.blocker_options.every(option => Array.isArray(option.valid_blockers) && option.valid_blockers.length === 0)) {
    return { cmd: { type: "declare_blockers", declarations: [] }, label: "Auto: no legal blockers" };
  }

  // Likewise, an attack declaration with no creature able to attack.
  if (decision.kind === "attackers" && Array.isArray(decision.attacker_options) && decision.attacker_options.length === 0) {
    return { cmd: { type: "declare_attackers", declarations: [] }, label: "Auto: no possible attackers" };
  }

  if (
    decision.kind === "select_options" &&
    decision.min === 1 &&
    decision.max === 1 &&
    !isOrderingDecision(decision)
  ) {
    const isManaColorChoice = /^choose\b.*\bmana\b/i.test(decision.description || "")
      && (decision.options || []).every((option) =>
        /^(white|blue|black|red|green|colorless)$/i.test(option.description || ""));
    if ((!isManaColorChoice && isPaymentDecision(decision)) || isSpellCastFlowDecision(decision)) {
      return null;
    }
    const legal = (decision.options || []).filter((o) => o.legal !== false);
    if (legal.length === 1 && (legal[0].point_cost ?? 1) === 1 && (legal[0].max_count ?? 1) >= 1) {
      return {
        cmd: { type: "select_options", option_indices: [legal[0].index] },
        label: `Auto: ${legal[0].description}`,
      };
    }
  }

  if (decision.kind === "number" && decision.min === decision.max) {
    return {
      cmd: { type: "number_choice", value: decision.min },
      label: `Auto: ${decision.min}`,
    };
  }

  if (decision.kind === "targets") {
    const reqs = decision.requirements || [];
    if (
      reqs.length > 0 &&
      reqs.every((req) => {
        const maxT =
          req.max_targets === null || req.max_targets === undefined
            ? req.legal_targets.length
            : Number(req.max_targets);
        return (
          req.legal_targets.length > 0 &&
          req.legal_targets.length === req.min_targets &&
          req.min_targets === maxT
        );
      })
    ) {
      const targets = reqs.flatMap((req) =>
        req.legal_targets.map((t) =>
          t.kind === "player"
            ? { kind: "player", player: Number(t.player) }
            : { kind: "object", object: Number(t.object) }
        )
      );
      return { cmd: { type: "select_targets", targets }, label: "Auto: targets selected" };
    }
  }

  return null;
}

