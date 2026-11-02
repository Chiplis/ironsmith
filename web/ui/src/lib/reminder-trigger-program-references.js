// Explicit source-defined class names bind external program references.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outside = (text, index) => (text.slice(0, index).match(/["“”]/g) || []).length % 2 === 0;
function replace(text, expression, transform) {
 return text.replace(expression, (...args) => outside(text, args.at(-2)) ? transform(...args) : args[0]);
}
export function expandTriggerProgramReferences(value, definition, sourceName = '') {
 const references = definition.triggerReferences;
 if (!references) return value;
 const noun = escape(references.noun), label = references.class;
 let text = String(value);
 const eventRule = event => `As ${event}, trigger each externally triggered ability named ${label} of the event source.`;
 text = replace(text, new RegExp(`(^|\\n)(?:${escape(sourceName)} and other )?permanents you control entering the battlefield cause their ${noun} to trigger\\.`, 'gim'), (_whole, prefix) => `${prefix}${eventRule('a permanent you control enters').replace('event source', 'event arrival')}`);
 text = replace(text, new RegExp(`(^|\\n)Creatures you control (dying|attacking) causes? their ${noun} to trigger\\.`, 'gim'), (_whole, prefix, action) => `${prefix}${eventRule(`a creature you control ${action.toLowerCase() === 'dying' ? 'dies' : 'attacks'}`)}`);
 text = replace(text, new RegExp(`(^|\\n)Opponents drawing a card during your turn causes one ${noun} of a permanent you control to trigger\\.`, 'gim'), (_whole, prefix) => `${prefix}As an opponent draws a card during your turn, trigger one externally triggered ability named ${label} from a permanent you control.`);
 text = replace(text, new RegExp(`enchanted player chooses a ${noun} ability of ([^.]+)\\. Trigger that ability\\.`, 'gi'), (_whole, sources) => `Trigger one ${label} triggered ability of ${sources}, chosen by enchanted player.`);
 text = replace(text, new RegExp(`trigger a ${noun} of any number of ([^.]+)\\.`, 'gi'), (_whole, sources) => `Choose any number of ${sources}. For each of those permanents, trigger one ${label} triggered ability of that permanent.`);
 text = replace(text, new RegExp(`trigger (?:a|one) ${noun} of that card`, 'gi'), () => `trigger one ${label} triggered ability of that card in your hand`);
 text = replace(text, new RegExp(`trigger up to one ${noun} of`, 'gi'), () => `trigger up to one ${label} triggered ability of`);
 text = replace(text, new RegExp(`trigger (?:a|one) ${noun} of`, 'gi'), () => `trigger one ${label} triggered ability of`);
 return text;
}
