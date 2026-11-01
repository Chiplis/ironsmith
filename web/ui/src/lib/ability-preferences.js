import { normalizeAbilityMatchText } from './inspector-ability-lines.js';

// Match canonical ability text, never use a translated label as a saved key.
export function findAbilityPreference(entries = [], texts = []) {
  const candidates = entries.filter(entry => texts.some(text => {
    const needle = normalizeAbilityMatchText(text);
    const ability = normalizeAbilityMatchText(entry.source_ability_text);
    return needle && ability && (needle === ability || needle.startsWith(`${ability} `));
  }));
  const identities = new Set(candidates.map(entry => entry.automation_identity));
  return identities.size === 1 ? candidates[0] : null;
}
