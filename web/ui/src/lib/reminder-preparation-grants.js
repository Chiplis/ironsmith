// A static grant can derive each recipient card's preparation payment while
// preserving its ordinary delayed followup. Keyword identity is presentation.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function expandGrantedPreparationAbilities(text, definitions) {
  let result = String(text);
  for (const definition of definitions) {
    const program = String(definition.expansion || definition.reminder || '');
    if (!/^You may pay \{Cost\} and exile this card from your hand as a sorcery\. /i.test(program)
      || !/When you next resolve a spell, (?:you may )?cast this card from exile without paying its mana cost\.$/i.test(program)
      || /["“”\r\n]/.test(program)) continue;
    const keyword = escape(definition.keyword);
    const pattern = new RegExp(`\\b(has|have)\\s+${keyword}\\.\\s+(?:Its|The) ${keyword} cost is equal to its mana cost reduced by (\\{\\d+\\})\\.`, 'gi');
    result = result.replace(pattern, (original, verb, reduction, offset) => {
      if ((result.slice(0, offset).match(/["“”]/g) || []).length % 2) return original;
      const bound = program.replace(/\{Cost\}/gi, `its mana cost reduced by ${reduction}`);
      return `${verb} "${bound} This is the ${definition.keyword} preparation ability.".`;
    });
  }
  return result;
}
