import { independentAbilityPrograms } from './reminder-definition-programs.js';
// A validated parameterized activation can be one item of a static grant.
// The identity marker and complete activation program are separate abilities.
import { parseParameterDeclaration, bindReminderParameters, bindReminderValueFromDeclaration } from './reminder-costs.js';
import { customKeywordForms, officialSurfaceForms } from './keyword-forms.js';
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
const key=value=>String(value).normalize('NFKC').replace(/\s+/g,' ').trim().toLowerCase();
const official=new Set([...MTG_KEYWORD_RULES.flatMap(rule=>[rule.title,...rule.aliases]),...officialSurfaceForms()].map(key));
const escape=value=>value.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
// Keep recipient grammar bounded; unrelated predicates, quoted names and
// choice/control-flow tails remain verbatim for strict compilation.
const grant=/^(.+?\b(?:has|have|gains?|gain)\s+)([^\n]+)$/i;
export function expandGrantedParameterizedAbilities(text,definitions) {
  return expandNumericGrants(expandDerivedManaGrants(expandGrantedManaValueAbilities(String(text), definitions), definitions), definitions).split('\n').map(line=>{
    const head=line.match(grant);if(!head || /["“”]/.test(head[1]) || !/\b(?:creatures?|permanents?|artifacts?|cards?|spells?|it)\b/i.test(head[1]))return line;
    for(const def of definitions) {
      if(!/^\s*\{cost\}(?:\s*,[^:]+)?\s*:/i.test(def.reminder)||(def.template && def.template.toLowerCase()!=='{cost}'))continue;
      const declaration=new RegExp(`(?:^|,\\s*|\\band\\s+)(${escape(def.keyword)})\\s+((?:\\{[^{}\\s]+\\})+)\\s*\\.?$`,'i');
      const match=head[2].match(declaration);if(!match)continue;
      if(def.version!==1||typeof def.expansion!=='string'||official.has(key(def.keyword)))throw new Error('Granted parameter expansion requires a validated custom definition.');
      const values=parseParameterDeclaration(def.reminder,def.keyword,`${match[1]} ${match[2]}`,def.template);if(!values)continue;
      const program=bindReminderParameters(def.expansion,def.keyword,values,customKeywordForms(def));
      if(independentAbilityPrograms(program).some(body=>/[\r\n]/.test(body))||/\{(?:cost|number|type|object|ability|name|player)\}/i.test(program))continue;
      const quotedPrograms=independentAbilityPrograms(program).map(quoteGrantedRuleProgram);
      if(quotedPrograms.some(body=>!body))continue;
      let prefix=head[2].slice(0,match.index).trim().replace(/(?:,\s*)?\band\s*$/i,'').replace(/,\s*$/,'').trim();
      if(/["“”]/.test(prefix))continue;
      const marker=`custom keyword "${def.keyword}"`;
      const items=`${prefix?`${prefix}, `:''}${marker}, and ${quotedPrograms.join(', and ')}`;
      return `${head[1]}${items}.`;
    }
    return line;
  }).join('\n');
}

// Detection uses the same complete bounded declaration reader as expansion.
export function hasGrantedParameterizedAbility(text, definition) {
  return expandGrantedParameterizedAbilities(text, [definition]) !== String(text);
}

// Read numeric operands only after an explicit grant verb or token-with head.
// Sentence boundaries separating independent ability kinds remain structural.
function expandNumericGrants(text, definitions) {
  let result = text;
  for (const def of definitions) {
    if (!/\{number\}/i.test(def.reminder) || (def.template && def.template.toLowerCase() !== '{number}')) continue;
    const declaration = new RegExp(`\\b${escape(def.keyword)}\\s+(\\d+|X)(?:,\\s*where\\s+X\\s+is\\s+(their power|its power|its mana value))?(?=\\s*(?:[.]|until\\b|as\\s+you\\s+cast\\s+(?:it|them|that\\s+spell)(?=\\s*(?:[.]|$))|$))`, 'gi');
    result = result.replace(declaration, (match, number, expression, offset) => {
      const before = result.slice(0, offset);
      if ((before.match(/["“”]/g) || []).length % 2) return match;
      const head = before.split('\n').pop();
      const token = /\bcreate\b[^.!?]*\btokens?\s+with\s*$/i.test(head);
      if (!token && !/\b(?:has|have|gains?|gain)\s*$/i.test(head)) return match;
      let program = bindReminderParameters(def.expansion, def.keyword, {number}, customKeywordForms(def));
      if (expression?.toLowerCase()==='its mana value') {
        // A numeric copy exception is evaluated as the token is created;
        // ordinary ongoing grants do not authorize a creation-time sample.
        if (!/\bcreate\b[^.!?]*\btokens?\b[^.!?]*\bcopy\b[^.!?]*\bexcept\b/i.test(head)) return match;
        program=program.replace(/\bX\b/g, 'the mana value of this permanent as it was created');
      } else if (expression) program = bindReminderValueFromDeclaration(program, `${def.keyword} X, where X is this creature's power.`).text;
      if (independentAbilityPrograms(program).some(body=>/[\r\n]/.test(body)) || /\{(?:cost|number|type|object|ability|name|player)\}/i.test(program)) return match;
      const abilities = independentAbilityPrograms(program).flatMap(body => body.split(/(?<=\.)\s+(?=(?:When|Whenever|At)\b)/));
      const quotedAbilities=abilities.map(quoteGrantedRuleProgram);
      if(quotedAbilities.some(ability=>!ability))return match;
      return [`custom keyword "${def.keyword}"`, ...quotedAbilities].join(', and ');
    });
  }
  return result;
}

// An explicit following price clause names the granted activation's source
// mana cost, independently of the outer grantor or selected-object tags.
function expandDerivedManaGrants(text,definitions) {
 let result=text;
 for(const def of definitions){
  if(!/\{cost\}/i.test(def.reminder) || (!/^\s*\{cost\}(?:\s*,[^:]+)?\s*:/i.test(def.reminder) && !/^As you cast\b/i.test(def.reminder) && !/^You may pay\s+\{Cost\}\s+rather than\b/i.test(def.reminder)))continue;
  const keyword=escape(def.keyword);
  const pattern=new RegExp(`\\b(has|have|gains?)\\s+${keyword}(?:\\s+(until (?:end of turn|the beginning of your next end step)))?\\.\\s+(?:Their ${keyword} costs are|(?:The|Its) ${keyword} cost is) equal to (?:their|its|the card['’]s) mana cost(?: (?:increased by|plus) ((?:\\{[^{}\\s]+\\})+))?\\.`,'gi');
  result=result.replace(pattern,(match,verb,duration,additional,offset)=>{
   if((result.slice(0,offset).match(/["“”]/g)||[]).length%2)return match;
   const activation=/^\s*\{cost\}/i.test(def.reminder);
   const cost=`${activation?'Pay ':''}this card's mana cost${additional?`, ${additional}`:''}`;
   let program=bindReminderParameters(def.expansion,def.keyword,{cost},customKeywordForms(def));
   const programs=independentAbilityPrograms(program).map(body=>
    /^As you cast\b/i.test(def.reminder) && /^As you cast\b/i.test(body) ? `${body} This text-adding cost is named ${def.keyword}.` : body);
   if(/^As you cast\b/i.test(def.reminder) && programs.filter(body=>/^As you cast\b/i.test(body)).length!==1)return match;
   if(programs.some(body=>/[\r\n]/.test(body)))return match;
   const bodies=programs.map(quoteGrantedRuleProgram);
   if(bodies.some(body=>!body))return match;
   return `${verb} custom keyword "${def.keyword}", and ${bodies.join(', and ')}${duration?` ${duration}`:''}.`;
  });
 }
 return result;
}

// One outer native quoted ability retains an independently parsed inner
// program. Apostrophes inside ordinary words remain ordinary characters.
export function quoteGrantedRuleProgram(program){
 const normalized=String(program).replace(/[“”]/g,'"');
 if(/[\r\n]/.test(normalized)||(normalized.match(/"/g)||[]).length%2||/(^|\s)'|'(?=\s|$)/.test(normalized))return null;
 return `"${normalized.replace(/"/g,"'")}"`;
}

// A finite recipient-relative generic price recipe. Only an explicitly
// declared X/mana-value relationship is consumed; quoted names remain opaque.
export function expandGrantedManaValueAbilities(text, definitions) {
  let result = String(text);
  for (const def of definitions) {
    if (def.template?.toLowerCase() !== '{cost}' || !/^As an additional cost to cast this spell, you may pay \{Cost\}\./i.test(def.expansion || '')) continue;
    const declaration = new RegExp(`\\b(has|have|gains?)\\s+${escape(def.keyword)}\\s+\\{X\\},\\s*where X is (?:its|that card['’]s) mana value(?=\\s*(?:[.]|$))\\.?`, 'gi');
    const expand = part => part.replace(declaration, (original, verb) => {
      let program = bindReminderParameters(def.expansion, def.keyword, {cost: "mana equal to this spell's mana value"}, customKeywordForms(def));
      program += ` This optional additional cost is named ${def.keyword}.`;
      const quoted = quoteGrantedRuleProgram(program);
      return quoted ? `${verb} custom keyword "${def.keyword}", and ${quoted}.` : original;
    });
    result = result.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part, index, parts) => {
      if (!(index % 2)) return expand(part);
      // A separately authored emblem/token rule is an executable nested grant.
      // It retains one outer quote, with bounded inner native ability quotes.
      const before = parts.slice(0,index).join('');
      if (!/\b(?:emblem|tokens?)\s+with\s*$/i.test(before) || part.includes("'")) return part;
      const body = part.slice(1,-1), expanded = expand(body);
      return expanded === body ? part : part[0] + expanded.replace(/"/g, "'") + part.at(-1);
    }).join('');
  }
  return result;
}
