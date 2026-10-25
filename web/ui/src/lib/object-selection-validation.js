// Rules supplied by the engine describe complete alternatives, not just the
// enclosing count range. Keep IDs opaque (including redacted remote choices).
export function isObjectSelectionValid(decision, selected) {
  const candidates = decision.candidates || [];
  const ids = Array.from(selected, String);
  if (new Set(ids).size !== ids.length) return false;
  if (!ids.every(id => candidates.some(c => String(c.id) === id && c.legal !== false))) return false;
  if (ids.length > (decision.max ?? candidates.length)) return false;
  if (!decision.allow_partial_completion && ids.length < (decision.min ?? 0)) return false;
  const alternatives = decision.selection_alternatives || [];
  return alternatives.length === 0 || alternatives.some(rule => {
    const allowed = new Set(rule.candidate_indices.map(index => String(candidates[index]?.id)));
    return ids.length >= rule.min && ids.length <= rule.max && ids.every(id => allowed.has(id));
  });
}
