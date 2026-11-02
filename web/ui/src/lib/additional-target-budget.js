// The optional allowance is shared by all target occurrences in this prompt.
export function withinAdditionalTargetBudget(requirements, selections) {
  let budget = 0;
  let used = 0;
  for (let index = 0; index < requirements.length; index += 1) {
    const requirement = requirements[index];
    const allowance = Math.max(0, Number(requirement.additional_target_budget || 0));
    budget = Math.max(budget, allowance);
    if (requirement.max_targets !== null && requirement.max_targets !== undefined) {
      const originalMax = Math.max(0, Number(requirement.max_targets) - allowance);
      used += Math.max(0, (selections[index]?.length || 0) - originalMax);
    }
  }
  return used <= budget;
}
