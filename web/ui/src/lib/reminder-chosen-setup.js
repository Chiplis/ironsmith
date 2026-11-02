// A chosen outside-game card declaration carries its deck criterion on the
// declaration, while its independently supplied reminder defines the action.
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const normalize = value => String(value || '').normalize('NFKC').replace(/[“”]/g, '"').replace(/[‘’]/g, "'").replace(/\s+/g, ' ').trim();
export function expandChosenSetupDeclarations(text, definition, originalSource = null) {
  const keyword = normalize(definition.keyword);
  const reminder = normalize(definition.reminder);
  const expected = `If this card outside the game is your chosen ${keyword}, you may swap it with a card in your opening hand.`;
  if (reminder.toLowerCase() !== expected.toLowerCase()) return text;
  const prefix = new RegExp(`^\\s*${escape(keyword)}\\s*[—–]\\s*(.+)$`, 'i');
  return String(text).split('\n').map(line => {
    const matched = normalize(line).match(prefix);
    if (!matched) return line;
    const originals = originalSource == null ? [] : String(originalSource).split('\n')
      .map(line => normalize(line).replace(/\s*\*?\([^()]*\)\*?\s*$/, ''))
      .map(line => line.match(prefix)).filter(Boolean);
    // Printed-text operands remain opaque source evidence even if another
    // custom mechanic has an executable import alias with the same spelling.
    const criterion = originals.length === 1 ? originals[0][1] : matched[1];
    if (!/^Each nonland card in your starting deck has "[^"\n]+"(?: or "[^"\n]+")* in its printed rules text\.$/i.test(criterion)) return line;
    return `Chosen outside game card "${keyword}": ${criterion} You may exchange this card with a card in your opening hand.`;
  }).join('\n');
}
