import {expandAbilityActionPrograms} from './reminder-ability-action-programs.js';
// Typed operands for inline custom actions. This module only substitutes a
// declared layout; strict native compilation remains the semantic validator.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const ABILITY = '(?:first strike|double strike|deathtouch|defender|flying|haste|hexproof|indestructible|lifelink|menace|reach|trample|vigilance)';
const PARAMETER = /\{(object|ability|name|player|subject)\}/i;
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
// Explicit nested program delimiters keep sibling source instructions outside
// macro scope. Only outer sentence separators become process separators.
function boundedProgramBody(program) {
  let quoted=false,depth=0;
  const body=[...program].map(char=>{
    if(/["“”]/.test(char)){quoted=!quoted;return char;}
    if(quoted)return char;
    if(char==='['){depth+=1;return char;}
    if(char===']'){depth-=1;return char;}
    return depth===0&&/[.!?]/.test(char)?';':char;
  }).join('').replace(/[;\s]+$/,'');
  return `[${body}]`;
}
export const hasActionParameters = definition => PARAMETER.test(String(definition.reminder || '')) || /\{(?:object|ability|name|player|subject)\}/i.test(String(definition.template || ''));

export function validateActionTemplate(keyword, template, reminder) {
  const layout = String(template || '');
  const names = [...layout.matchAll(/\{([^{}]+)\}/g)].map(match => match[1].toLowerCase());
  if (!names.length || names.some(name => !['object', 'ability', 'name', 'player', 'subject'].includes(name))) throw new Error(`"${keyword}"'s action layout needs {Object}, {Ability}, {Name}, and/or {Player}.`);
  if (names.includes('subject') && !(names.length === 1 || names.length === 2 && names.includes('object'))) throw new Error(`"${keyword}"'s subject action layout must bind one subject operand.`);
  if (names.includes('player') && names.length !== 1) throw new Error(`"${keyword}"'s player action layout must bind one player operand.`);
  if (names.includes('object') && names.includes('name')) throw new Error(`"${keyword}"'s action layout cannot ambiguously combine {Object} and {Name}.`);
  if (new Set(names).size !== names.length) throw new Error(`"${keyword}"'s action layout repeats an operand.`);
  // Mana symbols in the effect body are literals, not declaration operands.
  const used = [...String(reminder).matchAll(/\{(object|ability|name|player|subject)\}/gi)].map(match => match[1].toLowerCase());
  if (used.some(name => !names.includes(name)) || names.some(name => !used.includes(name))) throw new Error(`"${keyword}"'s action layout and reminder must use the same operands.`);
  if (/[^a-z{}\s—–-]/i.test(layout)) throw new Error(`"${keyword}"'s action layout may contain only operands, words, spaces and dashes.`);
  if (/\}\{/.test(layout)) throw new Error(`"${keyword}"'s action operands need a space between them.`);
}
function actionPatternSource(definition) {
  let rest = String(definition.template || '').trim();
  if (!rest) return null;
  // A line-start match must not absorb a modal bullet into its subject.
  // Leave that delimiter for the explicit bullet-prefix alternative below.
  const recipientAction = rest.match(/^\{subject\}([^{}]+)\{object\}$/i);
  if (recipientAction) {
    const verb = escape(recipientAction[1].trim()).replace(/\s+/g, '[ \\t]+');
    return `(^|[,.:;•]\\s*|\\bthen\\s+|\\byou may\\s+)(?<subject>[^.!?;,:•\\n"“”()]+?)[ \\t]+${verb}[ \\t]+(?<object>[^.!?;:\\n"“”()]+?)(?<unless>[ \\t]+unless[ \\t]+[^.!?;\\n]+)?(?=[ \\t]*(?:[.;\\n]|,\\s*then\\b|$))`;
  }
  const subjectLayout = rest.match(/^\{subject\}([^{}]*)$/i);
  if (subjectLayout) {
    const tail = escape(subjectLayout[1]).replace(/\s+/g, '[ \\t]+');
    // Inflect the verb head, preserving a declared multiword action's object.
    const [head, ...words] = String(definition.keyword).split(/\s+/);
    const verb = `${escape(head)}(?:s|es)?${words.length ? '[ \\t]+' + words.map(escape).join('[ \\t]+') : ''}`;
    const repeated = `(?:[ \\t]*,[ \\t]*then[ \\t]+you may have[ \\t]+it[ \\t]+${escape(definition.keyword)}[ \\t]+again)`;
    return `(^|[,.:;•]\\s*|\\bthen\\s+)(?<optionalSubject>(?<optionalActor>you|they|that player|its controller|its owner) may have[ \\t]+)?(?<subject>[^.!?;,:•\\n"“”()]+?)[ \\t]+(${verb})${tail}(?<repeatedSubject>${repeated})?(?=[ \\t]*(?:[.;\\n]|,\\s*then\\b|and\\s+(?:gains?|gets?|has|have)\\b|$))(?<subjectContinuation>[ \\t]+and[ \\t]+(?=(?:gains?|gets?|has|have)\\b))?(?<subjectResult>[.;][ \\t]*When it does,[^\\n]*)?`;
  }
  let source = '', position = 0;
  for (const match of rest.matchAll(/\{(object|ability|name|player|subject)\}/gi)) {
    source += escape(rest.slice(position, match.index)).replace(/\s+/g, '[ \\t]+');
    const name = match[1].toLowerCase();
    source += name === 'player' ? '(?<player>any number of target players|target player|that player|each player|each opponent|yourself|you|its controller|its owner|that (?:creature|permanent|artifact|enchantment|land|planeswalker|battle)\'s (?:controller|owner))'
      : name === 'ability' ? `(?<ability>${ABILITY})`
      : name === 'name' ? '(?<name>"[^"\\n]+"|“[^”\\n]+”|[^.!?;,:\\n"“”()]+?)'
      : '(?<object>[^.!?;:\\n"“”()]+?)';
    position = match.index + match[0].length;
  }
  source += escape(rest.slice(position)).replace(/\s+/g, '[ \\t]+');
  const actor="each player|that player|target (?:nonland )?(?:creature|permanent|artifact|enchantment|land)'s (?:controller|owner)";
  return `(^|[,.:;•]\\s*|\\b(?:then|and|instead)\\s+|\\byou(?: may)?\\s+)(?:(?<actor>${actor})[ \\t]+(?<actorMay>may[ \\t]+)?)?(${escape(definition.keyword)}s?)[ \\t]+${source}(?:[ \\t]+(?<repetition>once|twice|(?:one|two|three|four|five|six|seven|eight|nine|ten|\\d+)[ \\t]+times))?(?<resolvingInstead>[ \\t]+instead[ \\t]+of[ \\t]+putting[ \\t]+it[ \\t]+into[ \\t]+your[ \\t]+graveyard[ \\t]+as[ \\t]+it[ \\t]+resolves)?(?<instead>[ \\t]+instead)?(?=[ \\t]*(?:[.;:\\n]|,\\s*then\\b|as an additional cost\\b|rather than pay\\b|and[ \\t]+(?:each (?:player|opponent)|you|they|that player)[ \\t]+(?:loses?|gains?|draws?|discards?|sacrifices?|creates?|exiles?|returns?|puts?|taps?|untaps?)[ \\t]+|$))(?<then>[ \\t]*,[ \\t]*then[ \\t]+)?`;
}
export function actionParameterPattern(definition) {
  return new RegExp(actionPatternSource(definition) || '(?!)', 'gim');
}
function validObject(value, sourceName) {
  const selector = /^(?:it|them|one of them|one of those creatures|the other|the iterated (?:card|object)|enchanted (?:creature|permanent|artifact|enchantment|land)|each (?:attacking )?creature you control|all creature cards in graveyards|the top card of (?:your|their) library|a card from your hand|a creature you control|this (?:creature|permanent|artifact|enchantment|land|source)|that (?:creature|permanent|artifact|enchantment|land|token)|target (?:creature|permanent|artifact|enchantment|land)(?: you (?:control|don't control))?)$/i;
  const cardSelector = /^(?:that card|a card from among (?:them|those cards)|target card in (?:your|an opponent's|a player's) (?:graveyard|hand|exile))$/i.test(value);
  value = value.replace(/^any number of up to X target /i, 'up to X target ');
  // Card selections are ordinary native selector operands, including counts,
  // disjoint quality conjunctions, randomness, and bounded relative filters.
  const cardSelection = /^(?:an?|(?:up to )?(?:one|two|three|four|five|six|seven|eight|nine|ten|\d+))\s+(?:[a-z][a-z'-]*[ ,]+){0,18}cards?(?:\s+(?:with mana value \d+ or less|of that color|at random|that was put into your graveyard this turn))?$/i.test(value)
    && !/\b(?:if|unless|then|whenever|may|choose|create|draw|sacrifice)\b/i.test(value);
  const targetSelector = /^(?:(?:(?:up to )?(?:one|two|three|four|five|six|seven|eight|nine|ten|x|\d+)|another|other)\s+)?(?:other\s+)?target\s+/i.test(value)
    // A capitalized subtype can stand alone as the native object noun
    // (target Knight or Soldier); native complete-filter admission validates
    // the declared type rather than a frontend list of creature identities.
    && (/\b(?:creatures?|permanents?|artifacts?|enchantments?|lands?|cards?|spells?)\b/i.test(value)
      || /\b[Tt]arget\s+[A-Z][a-z'-]+(?:\s+(?:or|and)\s+[A-Z][a-z'-]+)?(?:\s|$)/.test(value))
    && value.split(/\s+/).length <= 24
    && !/\b(?:if|unless|then|while|until|where|when|whenever|may|choose|create|draw|put|sacrifice)\b/i.test(value);
  // Resolution-time card selections are typed operands. The native choice
  // grammar retains their qualities/count; no target is invented here.
  const cardChoice=/^(?:a|an|one|any number of) (?:(?:nonland|permanent|creature|artifact|enchantment|instant|sorcery|land) )?cards? from your (?:hand|graveyard)$/i.test(value);
  const objectWords=/\b(?:cards?|creatures?|permanents?|artifacts?|enchantments?|lands?|spells?|planeswalkers?|battles?)\b/i.test(value);
  const nonAction=!/\b(?:if|unless|then|while|until|where|when|whenever|may|choose|create|draw|draws|put|sacrifice|loses?|gains?|deals?|returns?|discards?)\b/i.test(value);
  const boundedSelector=value.split(/\s+/).length<=40 && objectWords && nonAction
    && /^(?:the (?:top|bottom)|those|each|all|another|other|a |an |up to (?:one|two|three|four|five|six|seven|eight|nine|ten|\d+) )/i.test(value);
  return /^(?:that|this) token$/i.test(value) || selector.test(value) || cardSelector || cardSelection || targetSelector || cardChoice || boundedSelector || (sourceName && [sourceName,sourceName.split(',')[0].trim()].some(name=>value.toLowerCase()===name.toLowerCase()));
}
export function expandActionParameterDeclarations(text, definition, sourceName = '') {
  text=expandAbilityActionPrograms(text,definition,invocation=>expandActionParameterDeclarations(invocation,definition,sourceName));
  // An authored action may give its final actual result a surface noun.
  // Bind that noun only after an actual valid invocation, within the same
  // ability line, and never inside quoted rules text.
  if (definition.resultReference) {
    const input = String(text), invocations = [...input.matchAll(actionParameterPattern(definition))];
    const reference = new RegExp(`\\b${escape(definition.resultReference)}\\b`, 'gi');
    for (const invocation of invocations.reverse()) {
      if (!outsideQuotation(input, invocation.index)) continue;
      const sentenceStart = Math.max(input.lastIndexOf('\n', invocation.index - 1), input.lastIndexOf('.', invocation.index - 1)) + 1;
      if (/\b(?:as (?:an? )?(?:additional )?cost|costs? to (?:cast|activate))\b/i.test(input.slice(sentenceStart, invocation.index))) continue;
      const operand = invocation.groups?.object || invocation.groups?.subject;
      if (operand && !validObject(operand.trim(), sourceName)) continue;
      const start = invocation.index + invocation[0].length;
      const newline = String(text).indexOf('\n', start);
      const end = newline < 0 ? String(text).length : newline;
      const tail = String(text).slice(start, end).replace(reference, (match, offset, whole) =>
        outsideQuotation(whole, offset) ? 'it' : match);
      text = String(text).slice(0, start) + tail + String(text).slice(end);
    }
  }
  return String(text).replace(actionParameterPattern(definition), (...args) => {
    const groups = args.at(-1), original = args[0], prefix = args[1];
    const offset = args.at(-3);
    if (!outsideQuotation(String(text), offset)) return original;
    // An action used as a casting/activation cost is not an effect macro.
    const keywordOffset = offset + prefix.length;
    const sentencePrefix = String(text).slice(Math.max(String(text).lastIndexOf('\n', keywordOffset - 1), String(text).lastIndexOf('.', keywordOffset - 1)) + 1, keywordOffset);
    const leadingPayment = /\b(?:as (?:an? )?(?:additional )?cost|costs? to (?:cast|activate))\b/i.test(sentencePrefix);
    const lineStart = String(text).lastIndexOf('\n', keywordOffset - 1) + 1;
    const lineEnd = String(text).indexOf('\n', keywordOffset);
    const line = String(text).slice(lineStart, lineEnd < 0 ? undefined : lineEnd);
    let quoted = false, costColon = -1;
    for (let index = 0; index < line.length; index++) {
      if (/["“”]/.test(line[index])) quoted = !quoted;
      else if (line[index] === ':' && !quoted) { costColon = lineStart + index; break; }
    }
    const activationCost = costColon >= keywordOffset;
    // The cost meaning is declared separately. A resolving effect macro may
    // never silently become a payment just because it precedes a colon.
    const remaining = String(text).slice(offset + original.length);
    const trailingPayment = /^\s+(?:as an additional cost to cast|rather than pay)\b/i.test(remaining);
    const paymentContext = activationCost || leadingPayment || trailingPayment;
    if (paymentContext && !definition.paymentReminder) return original;
    const reminder = paymentContext ? definition.paymentReminder : definition.actionProgram === true ? (definition.expansion || definition.reminder) : definition.reminder;
    if (groups.subjectContinuation && !/^Choose \{Subject\}\./i.test(reminder)) return original;
    const subject = groups.subject?.trim().replace(/^any number of up to X target /i, 'up to X target ');
    if (subject && !validObject(subject, sourceName)) return original;
    const objectOperand = groups.object?.trim();
    // Only an immediate first body action can inherit a loop's implicit
    // object. Earlier choices or effects may establish another `it`.
    const loop=sentencePrefix.match(/\bfor each (card|object)\b[^,.!?;\n]*,\s*(?:you may\s+)?$/i);
    const object = objectOperand?.toLowerCase()==='it' && loop ? `the iterated ${loop[1].toLowerCase()}` : objectOperand;
    if (object && !validObject(object, sourceName)) return original;
    const name = groups.name?.trim();
    if (name && (name.length > 200 || (!/^["“]/.test(name) && /\b(?:if|unless|then|while|until|where)\b/i.test(name)))) return original;
    if (subject && object) {
      const selects = /^(?:target |a |an |up to )/i.test(subject);
      const kind = subject.match(/\b(creature|permanent|artifact|enchantment|land|card|spell)\b/i)?.[1];
      if (selects && !kind) return original;
      const boundSubject = selects ? `that ${kind.toLowerCase()}` : subject;
      const prelude = selects ? `Choose ${subject}. ` : '';
      const program = prelude + String(definition.reminder).replace(/\{subject\}/gi, boundSubject).replace(/\{object\}/gi, object).replace(/[.!?]+$/, '');
      if (groups.unless) return prefix + `Unless ${groups.unless.trim().replace(/^unless\s+/i, '')}, do the following: ` + program.replace(/[.!?]/g, ';');
      if (/you may\s*$/i.test(prefix)) return prefix.replace(/you may\s*$/i, '') + 'You may do the following: ' + program.replace(/[.!?]/g, ';');
      return prefix + program;
    }
    if (groups.repetition && !groups.player) return original;
    const countWords = {once:1,twice:2,one:1,two:2,three:3,four:4,five:5,six:6,seven:7,eight:8,nine:9,ten:10};
    const repetition = groups.repetition?.toLowerCase().replace(/\s+times$/, '');
    const count = groups.repeatedSubject ? 2 : repetition ? (countWords[repetition] ?? Number(repetition)) : 1;
    if (!Number.isInteger(count) || count < 1 || count > 32) return original;
    const player = groups.player?.trim().toLowerCase();
    const quantified = /^each /.test(player || '');
    const countedPlayers = player === 'any number of target players';
    const recipient = object || subject;
    const targeted = /^(?:(?:(?:up to (?:[a-z]+|\d+)|another|other)\s+)?(?:other\s+)?target)\s+/i.test(recipient || '');
    const pluralTarget = definition.collectiveObject !== true && targeted && /\btarget(?:\s+(?:attacking|nontoken|other|nonland|tapped|untapped|artifact|enchantment|creature))*\s+(creatures|permanents|artifacts|enchantments|lands|cards|spells)\b/i.exec(recipient);
    const collection=definition.collectiveObject !== true && recipient?.match(/^(?:each|all) (.+)$/i);
    const cardSelection=recipient && /^(?:a|an|one|any number of) (?:(?:nonland|permanent|creature|artifact|enchantment|instant|sorcery|land) )?cards? from your (?:hand|graveyard)$/i.test(recipient);
    const multipleCardSelection=cardSelection && /^any number of /i.test(recipient);
    const collectionKind = /\bcards?\b/i.test(recipient || '') ? 'card' : /\bspells?\b/i.test(recipient || '') ? 'spell' : definition.perObjectProgram ? 'object' : 'permanent';
    const operand = subject ? 'subject' : 'object';
    const repeated = (String(reminder).match(new RegExp(`\\{${operand}\\}`, 'gi')) || []).length > 1;
    const declarationBody=String(reminder).trim().replace(/[.!?]+$/, '');
    let declarationQuoted=false, compound=false;
    for(const char of declarationBody){if(/["“”]/.test(char))declarationQuoted=!declarationQuoted;else if(!declarationQuoted&&/[.!?;\n]/.test(char))compound=true;}
    // Keep the existing one-clause target foreach surface. Whole programs
    // and subject predicates select once before iterating the exact group.
    // An optional announced target may select no recipient. Keep the complete
    // declared program (including choices, draws and action receipts) inside
    // that selected set instead of running its object-free instructions once.
    const optionalTargetProgram = definition.collectiveObject !== true && targeted
      && /^up to\s+/i.test(recipient) && (compound || definition.actionProgram === true);
    const wholeGroup = definition.perObjectProgram || multipleCardSelection || optionalTargetProgram || (pluralTarget && (Boolean(subject) || compound));
    const objectPrelude = recipient && (definition.perObjectProgram || cardSelection || (targeted && (wholeGroup || (!pluralTarget && repeated)))) ? `Choose ${recipient}. ` : '';
    const boundRecipient = definition.perObjectProgram ? 'the iterated object' : cardSelection ? 'that card' : collection ? `that ${collectionKind}` : wholeGroup ? `that ${collectionKind}` : pluralTarget ? `that ${pluralTarget[1].slice(0,-1).toLowerCase()}` : objectPrelude && /\bor\b/i.test(recipient) ? `that ${collectionKind}` : objectPrelude ? `that ${(recipient.match(/\b(creature|permanent|artifact|enchantment|land|card|spell)\b/i)?.[1] || 'permanent').toLowerCase()}` : recipient;
    const values = { subject: subject ? boundRecipient : subject, object: object ? boundRecipient : object, ability: groups.ability?.trim(), name, player: player === 'target player' || quantified || countedPlayers ? 'that player' : player === 'yourself' ? 'you' : player };
    let expanded = String(reminder).replace(/\{(object|ability|name|player|subject)\}/gi, (_, name) => values[name.toLowerCase()]);
    if (values.player === 'you') {
      const verbs = {loses:'lose', gains:'gain', draws:'draw', discards:'discard', sacrifices:'sacrifice', creates:'create', exiles:'exile', returns:'return', puts:'put', taps:'tap', untaps:'untap'};
      expanded = expanded.replace(/^you (loses|gains|draws|discards|sacrifices|creates|exiles|returns|puts|taps|untaps)\b/i, (_, verb) => `you ${verbs[verb.toLowerCase()]}`)
        .replace(/\bthey\b/g, 'you').replace(/\btheir\b/g, 'your');
    }
    const body = expanded.trim().replace(/[.!?]+$/, '');
    // Quantified bodies stay inside one participant's scope. A multi-sentence
    // body needs a native scope boundary before it can be repeated safely.
    if (quantified && /[.!?;\n]/.test(body)) return original;
    const optionalPayment = /^You may pay the cost ["“]/i.test(body);
    // A subject-result pronoun belongs to this declared payment, inside the
    // same bound recipient scope. Declining payment cannot become completion
    // merely because an enclosing 'may have' was accepted.
    if (groups.subjectResult && !optionalPayment) return original;
    const result = groups.subjectResult?.replace(/^[.;]\s*When it does,/i, 'When you do,');
    const completeBody = result ? `${body}. ${result}` : body;
    const iterations = Array.from({length:count}, () => completeBody);
    const program = countedPlayers
      ? `For each of ${player}, do the following: ${boundedProgramBody(iterations.join('. '))}`
      : quantified
      ? `For ${player}, ${iterations.join(', then ')}`
      : wholeGroup ? `For each ${collectionKind} chosen this way, do the following: ${boundedProgramBody(iterations.join('. '))}`
      : collection ? `For each ${collection[1].replace(/\b(creature|permanent|artifact|enchantment|land|card|spell)s\b/gi, '$1')}, do the following: ${boundedProgramBody(iterations.join('. '))}`
      : pluralTarget ? `For each of ${recipient}, ${iterations.join('. ')}`
      : player === 'target player' ? `For each target player, do the following: ${boundedProgramBody(iterations.join('. '))}`
      : iterations.join('. ');
    let actionActor=groups.actor;
    if(!actionActor&&definition.actionProgram===true&&/^then\s+/i.test(prefix)){
      const before=String(text).slice(0,offset);
      const clause=before.slice(Math.max(before.lastIndexOf('\n'),before.lastIndexOf('.'))+1);
      // An omitted actor in a coordinated action retains the explicitly
      // named preceding creator, not the controller of the resolving spell.
      if(/^\s*(?:that (?:creature|permanent|artifact|enchantment|land|card)'s (?:controller|owner)|its (?:controller|owner)|that player) creates?\b[\s\S]*,\s*$/i.test(clause))actionActor='that player';
    }
    if (actionActor) {
      const actor=actionActor.toLowerCase();
      const actorTarget=actor.match(/^(target (?:nonland )?(?:creature|permanent|artifact|enchantment|land))'s (controller|owner)$/);
      const actorPrelude=actorTarget?`Choose ${actorTarget[1]}. `:'';
      const person=actor==='each player'?'that player':actorTarget?`that permanent's ${actorTarget[2]}`:actor;
      if(definition.actionProgram===true){
        // A declared complete operation owns one actor scope. Let native
        // NamedActionProgram execute its whole choice/branch/cost program
        // under that actor, rather than inflecting selected child verbs.
        const nativeActor=actorTarget?`its ${actorTarget[2]}`:person;
        // Announced object targets stay outside the participant's resolving
        // instruction. Two distinct newly-targeted actor/recipient objects
        // need an explicit captured-player alias rather than rebinding its.
        if(actorTarget&&objectPrelude)return original;
        const match=program.match(/^(Perform the [^:]+ action(?:[^:]*)?:)/i);
        if(!match)return original;
        const scoped=program.replace(/^Perform\b/i,`${nativeActor} performs`);
        const operation=groups.actorMay?`${nativeActor} may do the following: ${boundedProgramBody(scoped)}`:scoped;
        const result=actor==='each player'?`For each player, do the following: ${boundedProgramBody(operation)}`:operation;
        return prefix+actorPrelude+objectPrelude+result+(groups.then?'. Then ':'');
      }
      const verbs={look:'looks',exile:'exiles',create:'creates',perform:'performs'};
      const authored=objectPrelude+program;
      // Quoted token abilities belong to the token's eventual controller.
      // Only the enclosing program receives the explicit action actor.
      const pieces=authored.split(/("[^"\n]*"|“[^”\n]*”)/g);
      const bound=pieces.map((piece,index)=>index%2?piece:piece
        .replace(/\byour\b/gi,'their')
        .replace(/(^|[.;:\[\],]\s*)(Look|Exile|Create|Perform)\b/gi,(_,start,verb)=>`${start}${person} ${verbs[verb.toLowerCase()]}`)).join('');
      const operation=groups.actorMay?`That player may do the following: ${boundedProgramBody(bound)}`:bound;
      const scoped=actor==='each player'?`For each player, do the following: ${boundedProgramBody(operation)}`:operation;
      return prefix+actorPrelude+scoped+(groups.then?'. Then ':'');
    }
    const optionalObject=Boolean(object)&&/\byou may\s*$/i.test(prefix);
    const expansionPrefix=optionalObject?prefix.replace(/\byou may\s*$/i,''):prefix.replace(/\byou\s*$/i,'');
    const optionalBody=boundedProgramBody(program);
    // Existing single-scope Subject declarations keep their accepted surface;
    // newly nested/grouped scopes require explicit brackets.
    const optionalSubject = groups.optionalSubject && !optionalPayment;
    const payload=optionalSubject&&!wholeGroup&&!collection&&!optionalObject?optionalBody.slice(1,-1):optionalBody;
    const actor = groups.optionalActor || 'you';
    // An explicit quoted price already owns the entire resource program.
    // The source permission supplies its sole optional offer and payer.
    const quotedPrice = /^pay the cost\s+["“]/i.test(program);
    // Other optional actor programs need their own explicit participant scope.
    // Only quoted prices currently declare an authoritative resource payer.
    if (groups.optionalActor && actor.toLowerCase() !== 'you' && !quotedPrice) return original;
    const boundedProgram=optionalSubject&&quotedPrice
      ? `${actor} may ${program}`
      : optionalSubject||optionalObject
        ? `${actor} may do the following: ${payload}` : program;
    if (groups.resolvingInstead) {
      return expansionPrefix + `Instead of putting the triggering spell into your graveyard as it resolves, do the following: ${boundedProgramBody(objectPrelude + program)}` + (groups.then ? '. Then ' : '');
    }
    if (groups.instead) {
      return expansionPrefix + `do the following instead: ${boundedProgramBody(objectPrelude + program)}` + (groups.then ? '. Then ' : '');
    }
    // A resolution choice belongs inside an optional action; announced
    // target preludes retain their existing announcement semantics.
    if (optionalObject && (cardSelection || (definition.perObjectProgram && !targeted))) {
      return expansionPrefix + `You may do the following: ${boundedProgramBody(objectPrelude + program)}` + (groups.then ? '. Then ' : '');
    }
    const continuationKind = /\bcreature\b|\btoken\b/i.test(subject || '') ? 'creature'
      : subject?.match(/\b(permanent|artifact|enchantment|land|card|spell)\b/i)?.[1] || 'permanent';
    const continuation = groups.subjectContinuation ? `. The ${continuationKind} chosen this way ` : '';
    return expansionPrefix + objectPrelude + boundedProgram + continuation + (groups.then ? '. Then ' : '');
  });
}
