// A completion event reads two independent facts from a reminder: the existing
// operation and its authored activation label. The effect body is not copied
// into a watcher, and a permanent's keyword marker is never event evidence.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
function operation(definition) {
  const reminder = String(definition.reminder || '');
  const labels = [...reminder.matchAll(/\bThis is the ([a-z][a-z0-9_-]*) activated ability\./gi)];
  if (labels.length !== 1) return null;
  const afterLabel=reminder.slice(labels[0].index+labels[0][0].length);
  const transform=/\bTransform this permanent\./i.exec(afterLabel);
  if(!transform || afterLabel.slice(0,transform.index).includes(':')) return null;
  // The operation must belong to an activation, rather than a spell/static
  // instruction carrying an unrelated marker in the same reminder.
  const colon = reminder.lastIndexOf(':', labels[0].index);
  if (colon < 0 || /[.\n]/.test(reminder.slice(colon + 1, labels[0].index))) return null;
  return { verb: 'transforms', label: labels[0][1].toLowerCase() };
}
export function expandNamedActivationEvents(text, definition, sourceName = '') {
  const relation = operation(definition);
  if (!relation) return String(text);
  const pattern = new RegExp(`(^|\\n)([ \\t]*(?:When|Whenever))[ \\t]+you[ \\t]+${escape(definition.keyword)}[ \\t]+([^,\\n]+)(?=,)`, 'gi');
  return String(text).replace(pattern, (whole, line, intro, authored, offset) => {
    if (!outsideQuotation(String(text), offset)) return whole;
    const object = authored.trim();
    let recipient;
    if ((sourceName && object.toLowerCase() === sourceName.toLowerCase()) || /^(?:this (?:creature|permanent|artifact|enchantment|land|planeswalker|battle)|it)$/i.test(object)) {
      recipient = 'this permanent';
    } else if (/^(?:a|an|another) (?:creature|permanent|artifact|enchantment|land|planeswalker|battle)$/i.test(object)) {
      recipient = `${object} you control`;
    } else return whole;
    return `${line}${intro} ${recipient} ${relation.verb} caused by a ${relation.label} activated ability`;
  });
}
export const hasNamedActivationEvent = (text, definition, sourceName = '') => expandNamedActivationEvents(text, definition, sourceName) !== String(text);
