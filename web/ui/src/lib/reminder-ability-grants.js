import { independentAbilityPrograms, unlabelledAbilityProgram } from './reminder-definition-programs.js';
import {quoteGrantedRuleProgram} from './reminder-granted-parameters.js';
// A complete triggered program can be granted as an ordinary quoted ability.
// Keep the recipient-relative self reference inside the granted program.
import { hasReminderParameters } from './reminder-costs.js';
import { hasActionParameters } from './reminder-action-parameters.js';
import { officialSurfaceForms } from './keyword-forms.js';
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
import { isStatusMechanic } from './custom-status-mechanics.js';
const official = new Set([...officialSurfaceForms(), ...MTG_KEYWORD_RULES.flatMap(rule => [rule.title, ...rule.aliases])].map(value => value.toLowerCase()));
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function expandGrantedTriggeredPrograms(text, definitions) {
  let result = String(text);
  for (const def of definitions) {
    if (def.template || hasReminderParameters(def.reminder, def.template)
      || hasActionParameters(def) || isStatusMechanic(def)
      || !/^(?:(?:When|Whenever|At)\b|This (?:creature|permanent|spell)\b|(?:Un)?tapped creatures\b|Damage dealt to this (?:creature|permanent)\b|For each generic mana in this (?:spell|ability)(?:'s|’s) total cost\b|You may cast\b|As an additional cost(?: named ["'][^"']+["'])? to cast this spell\b|If this (?:spell|card)\b)/i.test(unlabelledAbilityProgram(def.expansion || ''))
      || independentAbilityPrograms(def.expansion).some(body=>!quoteGrantedRuleProgram(body))) continue;
    // Membership is a recipient-relative predicate, independent of granting
    // the program below. Preserve its exact custom identity marker.
    const membership=new RegExp(`\\b(if|unless)\\s+(it|this spell|that spell)\\s+(doesn't have|does not have|has)\\s+${escape(def.keyword)}\\b`,'gi');
    result=result.replace(membership,(match,condition,subject,verb,offset)=>
      (result.slice(0,offset).match(/["“”]/g)||[]).length%2?match:`${condition} ${subject} is a spell ${verb.toLowerCase()==='has'?'with':'without'} custom keyword "${def.keyword}"`);
    // Historical membership is identity evidence, never an instruction to
    // grant or perform the keyword's program. An omitted repeated pronoun
    // belongs only to this bounded demonstrative predicate conjunction.
    const ellipticalPast = new RegExp(`(\\bif\\s+it\\s+was\\s+[^.!?\\n]+?\\s+or)\\s+had\\s+${escape(def.keyword)}(?=\\s*[,.;])`, 'gi');
    result = result.replace(ellipticalPast, (match, prefix, offset) =>
      (result.slice(0, offset).match(/["“”]/g) || []).length % 2 ? match : `${prefix} it had ${def.keyword}`);
    const pastMembership = new RegExp(`\\b(it|that creature|that permanent|that card|that spell)\\s+had\\s+${escape(def.keyword)}(?=\\s*[,.;]|\\s+(?:and|or)\\b)`, 'gi');
    result = result.replace(pastMembership, (match, subject, offset) =>
      (result.slice(0, offset).match(/["“”]/g) || []).length % 2 ? match : `${subject} had custom keyword "${def.keyword}"`);
    // An object with a keyword is a membership filter when its following
    // owner/controller qualifier still describes that object, not a grant.
    const objectMembership = new RegExp(`\\b(card|cards|spell|spells|permanent|permanents|creature|creatures|artifact|artifacts|enchantment|enchantments|aura|auras)\\s+(with|without)\\s+${escape(def.keyword)}(?=\\s+(?:you|an opponent|a player)\\s+(?:own|owns|owned|control|controls|controlled)\\b)`, 'gi');
    result = result.replace(objectMembership, (match, noun, relation, offset) =>
      (result.slice(0, offset).match(/["“”]/g) || []).length % 2 ? match : `${noun} ${relation} custom keyword "${def.keyword}"`);
    // Only explicit has/have/gain grants. Quoted text and names stay opaque.
    const pattern = new RegExp(`\\b(has|have|gains?|with)\\s+((?:(?:${[...official].sort((a,b)=>b.length-a.length).map(escape).join('|')})(?:\\s*,\\s*|\\s+and\\s+|\\s+or\\s+))*)${escape(def.keyword)}(?=\\s*(?:[.,]|until\\b|or\\b|as you cast (?:it|that spell)\\b|and\\s*$|$))`, 'gi');
    result = result.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part, index, parts) => {
      if (index % 2) {
        const body = part.slice(1, -1);
        const before = parts.slice(0, index).join('');
        // Native quoted-static grants use apostrophes for their inner rules.
        // Expand only an explicit recipient rule owned by a granted/token ability.
        if (!/\bcustom keyword\b/i.test(body) && !body.includes("'")
          && /\b(?:has|have|gains?|gain)\b/i.test(body)
          && /\b(?:has|have|gains?|gain|with)\s+(?:(?:[a-z]+)\s+and\s+)?$/i.test(before)) {
          const expanded = expandGrantedTriggeredPrograms(body, [def]);
          if (expanded !== body && !def.expansion.includes("'")) return part[0] + expanded.replace(/"/g, "'") + part.at(-1);
        }
        return part;
      }
      return part.replace(pattern, (original, verb, prefix, offset) => {
        if (prefix.trim() && !prefix.trim().replace(/(?:,\s*)?(?:and|or)\s*$/, '').split(/,\s*|\s+(?:and|or)\s+/).every(name => official.has(name.trim().toLowerCase()))) return original;
        if (verb.toLowerCase() === 'with' && !/\bcreate\b[^.!?]*\btokens?\s*$/i.test(part.slice(0, offset))) return original;
        // Spell-resolution programs cannot be guessed as intrinsic abilities
        // of a newly created permanent token merely sharing their name.
        if(verb.toLowerCase()==='with' && /^If this (?:spell|card)\b/i.test(def.expansion))return original;
        const choice=/\bor\s*$/i.test(prefix);
        const retained = prefix.trim().replace(/(?:,\s*)?(?:and|or)\s*$/, '').trim();
        const programs = independentAbilityPrograms(def.expansion).flatMap(body => /^(?:(?:When|Whenever|At)\b|As an additional cost(?: named ["'][^"']+["'])? to cast this spell\b)/i.test(unlabelledAbilityProgram(body)) ? [body]
          : body.split(/(?<=\.)\s+(?=(?:This (?:creature|permanent|spell)|(?:Un)?tapped creatures)\b)/));
        if (programs.some(body=>!quoteGrantedRuleProgram(body))) return original;
        return `${verb} ${retained ? `${retained}${choice ? ' or ' : ', '}` : ''}custom keyword "${def.keyword}", and ${programs.map(quoteGrantedRuleProgram).join(', and ')}`;
      });
    }).join('');
  }
  return result;
}
