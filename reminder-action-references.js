// Explicit event forms bind source syntax to immutable operation receipts.
// Only complete self-action trigger paragraphs own these body references.
import {customKeywordForms} from './keyword-forms.js';
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const nouns = 'creature|permanent|artifact|enchantment|land|planeswalker|battle|card|object';
function bindActionBody(body, forms, pluralObjects) {
  const past = escape(forms.participle);
  return body.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index) => {
    if (index % 2) return part;
    // Quantities read the event's completed object count; collection targets
    // read its retained objects rather than a new filter over the battlefield.
    part = part.replace(new RegExp(`\\bthe number of (?:${nouns})s it ${past}\\b`, 'gi'), 'that many');
    part = part.replace(new RegExp(`\\beach (${nouns}) it ${past}\\b`, 'gi'), (_,noun) => `each of those ${noun.toLowerCase()}s`);
    part = part.replace(new RegExp(`\\b(${nouns})s it ${past}\\b`, 'gi'), (_,noun) => `those ${noun.toLowerCase()}s`);
    // With plural action objects, singular pronouns still name the performer.
    // A singular Vision recipient keeps the ordinary card antecedent.
    if (pluralObjects) part = part.replace(/\bits\b/gi, "this permanent's").replace(/\bit\b/gi, 'this permanent');
    return part;
  }).join('');
}
export function expandNamedActionReferences(text, definitions, faceName = '') {
  let result = String(text);
  for (const definition of definitions) {
    const forms = customKeywordForms(definition), verb = forms.eventVerb;
    if (!verb) continue;
    const subjects = ['this permanent', 'this creature', 'this artifact', ...(faceName ? [faceName] : [])].map(escape).join('|');
    const pattern = new RegExp(`^(\\s*)(Whenever|When)\\s+(?:${subjects})\\s+${escape(verb)}s?\\s+([^,;\\n]+),([\\s\\S]*)$`, 'i');
    result = result.split('\n').map(line => {
      const cost = new RegExp(`((?:this spell|it) costs? [^.!?\\n]*?less to cast for each (${nouns})s?) ${escape(forms.participle)} this way`, 'gi');
      line = line.replace(cost, '$1 affected this way');
      const match = pattern.exec(line);
      if (!match) {
        // Player-action one-or-more clauses consume one complete packet. A
        // singular per-recipient trigger is deliberately a different reader.
        const player = new RegExp(`^(\\s*)(Whenever|When)\\s+(you|a player|an opponent)\\s+${escape(verb)}s?\\s+one or more\\s+((?:${nouns})s)(?:\\s+([^,;\\n]+))?,([\\s\\S]*)$`, 'i').exec(line);
        if (!player) return line;
        const [,space,intro,actor,noun,qualifier,body] = player;
        const person = actor.toLowerCase();
        const operation = person === 'you' ? 'perform' : 'performs';
        const objects = noun + (qualifier ? ` ${qualifier}` : '');
        return `${space}${intro} ${person} ${operation} the ${forms.eventLabel || definition.keyword} action with ${objects},${body}`;
      }
      const [,space,intro,objects,body] = match;
      const pluralObjects = new RegExp(`\\b(?:${nouns})s\\b`, 'i').test(objects);
      return `${space}${intro} this permanent performs the ${forms.eventLabel || definition.keyword} action with ${objects},${bindActionBody(body, forms, pluralObjects)}`;
    }).join('\n');
  }
  return result;
}
