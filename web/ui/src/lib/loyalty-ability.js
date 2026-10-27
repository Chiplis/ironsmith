// Loyalty costs belong to abilities, including abilities gained by creatures.
export function splitLoyaltyAbility(text) {
  const match = String(text).match(/^\s*(?:\[([+−-]?(?:\d+|X))\]|([+−-]?(?:\d+|X)))\s*[:：]\s*(.+)$/iu);
  if (!match) return null;
  const cost = (match[1] || match[2]).replace('-', '−');
  return { cost, text: match[3], direction: cost.startsWith('+') ? 'up' : cost.startsWith('−') ? 'down' : 'zero' };
}
