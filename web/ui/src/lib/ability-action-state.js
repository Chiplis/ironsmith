export const ABILITY_INK = { available: '#1763a6', selected: '#236a32', unavailable: '#b52c28' };

export function abilityActionState(action, selected = false) {
  if (selected) return 'selected';
  if (!action || action.payment_pending) return 'pending';
  if (action.available === false || action.legal === false || action.mana_payment_available === false) return 'unavailable';
  if (Object.hasOwn(action, 'mana_payment_available') && action.mana_payment_available == null) return 'pending';
  return 'available';
}

export function abilityActionReason(action) {
  if (abilityActionState(action) === 'pending') return 'Checking availability…';
  return action?.unavailable_reason || action?.disabled_reason
    || (action?.mana_payment_available === false ? 'Cannot pay this ability’s cost.' : 'This ability cannot be activated right now.');
}

// One hit area per ability; distinct mana choices keep their own regions.
export function abilityHitRegions(regions) {
  const grouped = new Map();
  for (const region of regions) {
    const key = `${region.lineIndex}:${region.option || ''}`;
    if (region.option || region.angle) { grouped.set(`${key}:${grouped.size}`, region); continue; }
    const prior = grouped.get(key);
    if (!prior) grouped.set(key, {...region});
    else {
      const bottom = Math.max(prior.top + prior.height, region.top + region.height);
      const right = Math.max(prior.left + prior.width, region.left + region.width);
      prior.top = Math.min(prior.top, region.top); prior.left = Math.min(prior.left, region.left);
      prior.height = bottom - prior.top; prior.width = right - prior.left;
    }
  }
  return [...grouped.values()].map(region => region.option || region.angle ? region : {
    ...region, left: 30, width: 428, top: region.top - 3, height: region.height + 6,
  });
}
