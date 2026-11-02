// Recipient-first custom actions bind a declared scalar body, independently
// of the action's spelling. The native reader owns all resulting operations.
import { reminderParameters, parseParameterDeclaration, bindReminderParameters } from './reminder-costs.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const eligible = definition => (reminderParameters(definition.reminder).length === 0
  || (reminderParameters(definition.reminder).length === 1 && reminderParameters(definition.reminder)[0] === 'number'))
  && /^(?:it|that creature|they) becomes?\b/i.test(definition.reminder);
const validRecipient = (value, name) => /^(?:it|them|those creatures|this (?:creature|permanent)|that creature|(?:another )?target (?:attacking )?(?:creature|artifact|permanent)(?: you control)?|any number of target creatures you control)$/i.test(value)
  || (!!name && value.toLowerCase() === name.toLowerCase());
export function expandRecipientActions(text, definition, sourceName = '') {
  const before = String(text);
  if (!eligible(definition)) return before;
  const keyword = escape(definition.keyword);
  const pattern = new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(?:(you may have)\\s+([^,.!?;\\n]+?)|([^,.!?;\\n]+?)\\s+each)\\s+${keyword}\\s+([XYZ]|\\d+)(?:\\s+until end of turn)?(\\s+and gain (?:flying|haste|menace|vigilance|trample|lifelink|deathtouch|reach|first strike|double strike|hexproof|indestructible) until end of turn)?(?=\\s*(?:[.;\\n]|,\\s*where\\b|$))`, 'gim');
  return before.replace(pattern, (whole, prefix, optional, singular, plural, number, gain, offset) => {
    if (!outsideQuotation(before, offset + prefix.length)) return whole;
    const recipient = (singular || plural).trim();
    if (!validRecipient(recipient, sourceName)) return whole;
    const values = parseParameterDeclaration(definition.reminder, definition.keyword, `${definition.keyword} ${number}`, definition.template);
    if (!values && reminderParameters(definition.reminder).length) return whole;
    if (!reminderParameters(definition.reminder).length && !new RegExp(`\\b${escape(number)}/${escape(number)}\\b`, 'i').test(definition.reminder)) return whole;
    let body = (values ? bindReminderParameters(definition.reminder, definition.keyword, values) : definition.reminder).trim().replace(/[.!?]+$/, '');
    const verb = body.match(/^(?:it|that creature|they) (becomes?)\s+/i);
    if (!verb) return whole;
    body = body.slice(verb[0].length);
    if (gain) body = body.replace(/ until end of turn$/i, '') + gain;
    // Optionality belongs to the entire animation, including the recipient.
    return prefix + (optional ? `you may have ${recipient} become ` : `${recipient} each become `) + body;
  });
}
export const hasRecipientAction = (text, definition, sourceName = '') => expandRecipientActions(text, definition, sourceName) !== String(text);
