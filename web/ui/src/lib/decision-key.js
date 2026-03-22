export function decisionKey(decision) {
  const metaKey = [
    decision.kind || "",
    decision.player ?? "",
    decision.source_id ?? "",
    decision.source_name || "",
    decision.reason || "",
    decision.description || "",
    decision.context_text || "",
    decision.consequence_text || "",
  ].join("|");
  if (decision.attacker_options) {
    return decision.attacker_options
      .map((opt) => {
        const targets = (opt.valid_targets || [])
          .map((target) => JSON.stringify(target))
          .join("+");
        return `${Number(opt.creature)}:${opt.must_attack ? 1 : 0}:${targets}`;
      })
      .join("|") + `|${metaKey}`;
  }
  if (decision.blocker_options) {
    return decision.blocker_options
      .map((opt) => {
        const blockers = (opt.valid_blockers || [])
          .map((blocker) => `${Number(blocker.id)}:${blocker.name || ""}`)
          .join("+");
        return `${Number(opt.attacker)}:${opt.min_blockers || 0}:${blockers}`;
      })
      .join("|") + `|${metaKey}`;
  }
  if (decision.candidates) {
    return decision.candidates.map((c) => c.id).join(",") + `|${metaKey}`;
  }
  if (decision.options) {
    return decision.options.map((o) => `${o.index}:${o.description}`).join(",") + `|${metaKey}`;
  }
  if (decision.requirements) {
    return decision.requirements
      .map((r) =>
        (r.legal_targets || [])
          .map((t) => (t.kind === "player" ? `p${t.player}` : `o${t.object}`))
          .join("+")
      )
      .join(",") + `|${metaKey}`;
  }
  return `|${metaKey}`;
}
