import { aliasSourceKeywordFaces, sourceDisplayText } from './source-keyword-aliases.mjs';
import { sourceSubtypeDefinitions } from './source-subtypes.mjs';
import { compileDraftInput } from '../../web/ui/src/lib/custom-mechanics.js';
import { applyCustomTokens, draftTokens } from '../../web/ui/src/lib/custom-tokens.js';

const MULTIFACE = new Set(['doubleface', 'transform', 'transform_like', 'transformLike',
  'modal_dfc', 'split', 'aftermath', 'flip', 'adventure', 'prepare']);

export function sourceFaces(card) {
  const faces = card.source_faces?.length ? card.source_faces : [{ name: card.name, original_text: card.original_text }];
  return faces.map(face => {
    if (face.name && typeof face.original_text === 'string') return face;
    // A frozen complete-source normal-face record already supplies each
    // independent printed field. Shape those fields without replacing the
    // evidence object or inventing a missing linked face.
    if (MULTIFACE.has(face.shape || card.source_shape) || typeof face.rulesText !== 'string'
      || typeof face.typeLine !== 'string' || !(face.cardName || face.fullName)) return face;
    const lines = [face.manaCost ? `Mana cost: ${face.manaCost}` : '', `Type: ${face.typeLine}`,
      face.power !== '' && face.power != null && face.toughness !== '' && face.toughness != null ? `Power/Toughness: ${face.power}/${face.toughness}` : '',
      face.loyalty !== '' && face.loyalty != null ? `Loyalty: ${face.loyalty}` : '',
      face.defense !== '' && face.defense != null ? `Defense: ${face.defense}` : '', face.rulesText];
    return { ...face, name: face.cardName || face.fullName, original_text: lines.filter(Boolean).join('\n') };
  });
}

export function cohortLayout(card) {
  return card.source_layout || card.source_shape || 'normal';
}

export function needsMissingFaces(card) {
  return MULTIFACE.has(cohortLayout(card)) && sourceFaces(card).length < 2;
}

// Source exports use bracketed printed statistics and level bands. Materialize
// those independent layout fields into the compiler's ordinary metadata and
// level-block grammar, without modifying the authoritative source faces.
export function materializeSourceLevelLayout(metadata, lines) {
  const output = [];
  let baseStats = null;
  let inLevel = false;
  const creature = metadata.some(line => /^Type:.*\bCreature\b/i.test(line));
  const hasBaseStats = metadata.some(line => /^Power\/Toughness:/i.test(line));
  const hasNumberedNonLevelSections = lines.some(line => /^\[(?!level\b)[a-z]+\s+\d+]/i.test(line));
  for (const original of lines) {
    let line = original;
    // Printed lore symbols are chapter layout, independent of a keyword or
    // card identity. Expand a closed Roman range into ordinary chapter symbols.
    const chapter=line.match(/^\[Lore\s+([IVX]+)(?:\s*[-–]\s*([IVX]+))?]\s*(.+)$/i);
    if(chapter) {
      const romans=['I','II','III','IV','V','VI','VII','VIII','IX','X'];
      const first=romans.indexOf(chapter[1].toUpperCase()),last=romans.indexOf((chapter[2]||chapter[1]).toUpperCase());
      if(first<0||last<first)throw Error('Printed lore symbols require an ascending Roman chapter range.');
      output.push(`${romans.slice(first,last+1).join(', ')} — ${chapter[3]}`);continue;
    }

    const level = line.match(/^\[level\s+(\d+(?:\s*[-–]\s*\d+|\s*\+)?)]\s*(.*)$/i);
    if (level) { output.push(`LEVEL ${level[1].replace('–','-')}`); line=level[2]; inLevel=true; }
    const stats = line.match(/\s*\[([+-]?\d+|[★*])\s*\/\s*([+-]?\d+|[★*])]\s*$/);
    if (stats && creature && !hasNumberedNonLevelSections) {
      line=line.slice(0,stats.index).trimEnd();
      if (line.trim()) output.push(line);
      if (inLevel) output.push(`${stats[1]}/${stats[2]}`);
      else if (!hasBaseStats) baseStats=`Power/Toughness: ${stats[1]}/${stats[2]}`;
    } else if (line.trim() || !level) output.push(line);
  }
  return { metadata: baseStats ? [...metadata,baseStats] : metadata, lines: output };
}

// Related token display blocks are source attachments, not additional rules of
// the referring card. Only a complete named token appendix is separated.
export function separateSourceTokenAppendices(text) {
  const pieces=text.split(/\n---\s*\n/);
  if(pieces.length<2)return {text,auxiliary:[]};
  const auxiliary=[];
  for(const block of pieces.slice(1)) {
    const lines=block.split('\n');
    const name=lines[0]?.trim().match(/^\*\*([^*]+)\*\*$/)?.[1];
    const type=lines[1]?.trim().match(/^Token\s+(.+)$/i)?.[1];
    const stats=lines.at(-1)?.trim().match(/^\*\*([+-]?\d+)\/([+-]?\d+)\*\*$/);
    if(!name||!type||!stats)return {text,auxiliary:[]};
    auxiliary.push({name,original_text:block,executable_text:`Type: ${type}\nPower/Toughness: ${stats[1]}/${stats[2]}\n${lines.slice(2,-1).join('\n')}`,token:true});
  }
  return {text:pieces[0],auxiliary};
}

function separateInput(face) {
  if (!face.name || typeof face.original_text !== 'string') throw Error('Source face requires name and original_text');
  const metadata = [], oracle = [];
  const attached=separateSourceTokenAppendices(face.original_text);
  let inAppendix = false;
  for (const line of attached.text.split('\n')) {
    // Unrecognized attachments remain complete source evidence. Do not turn
    // their fields into host metadata or normalize their display markup.
    if (/^---\s*$/.test(line)) inAppendix = true;
    if (inAppendix) { oracle.push(line); continue; }
    const stats = line.match(/^(?:Power\/Toughness|P\/T):\s*(.+)$/);
    if (stats) { metadata.push(`Power/Toughness: ${stats[1]}`); continue; }
    if (/^(?:Mana cost|Type|Power\/Toughness|Loyalty|Defense|Color indicator):/.test(line) && !oracle.length) metadata.push(line);
    else oracle.push(line);
  }
  // Carry explicit per-face source colors through the same native metadata
  // channel as supplied token colors, without editing frozen source prose.
  if (face.color_indicator != null) {
    const allowed=new Set(['White','Blue','Black','Red','Green']);
    if(!Array.isArray(face.color_indicator)||face.color_indicator.some(color=>!allowed.has(color)))throw Error('Source face has an invalid typed color indicator');
    if(face.color_indicator.length && !metadata.some(line=>line.startsWith('Color indicator:')))metadata.push(`Color indicator: ${face.color_indicator.join(', ')}`);
  }
  const appendixStart = oracle.findIndex(line => /^---\s*$/.test(line));
  const hostLines = appendixStart < 0 ? oracle : oracle.slice(0,appendixStart);
  const appendixLines = appendixStart < 0 ? [] : oracle.slice(appendixStart);
  const shaped = materializeSourceLevelLayout(metadata, hostLines);
  const hostText = shaped.lines.join('\n').replace(/\*\(/g, '(').replace(/\)\*/g, ')').replace(/\*([^*\n]+)\*/g, '$1');
  let oracleText = [hostText,...appendixLines].join('\n');
  // Explicit reviewed corrections repair a source ellipsis only for this
  // exact face. They never mutate the frozen source evidence or invent a
  // global text substitution keyed to a card name.
  for (const correction of face.reviewed_source_corrections || []) {
    const { original, replacement, source_context, evidence, review_status } = correction;
    if (review_status !== 'source_context_reviewed' || typeof original !== 'string' || !original
      || typeof replacement !== 'string' || !replacement || replacement === original
      || typeof source_context !== 'string' || !source_context.includes(original)
      || typeof evidence !== 'string' || !evidence.trim()) throw Error('Source correction needs exact reviewed context and evidence.');
    if (!face.original_text.includes(source_context) || oracleText.split(original).length !== 2) throw Error('Reviewed source correction does not match one exact source occurrence.');
    oracleText = oracleText.replace(original, replacement);
  }
  return { name: face.name, metadata: shaped.metadata, oracleText, auxiliaryTokens: attached.auxiliary,
    typeLine: shaped.metadata.find(line=>line.startsWith('Type:'))?.slice(5).trim() || '' };
}

// Source-only migration metadata: official-name collisions are renamed before
// the strict custom-mechanic validator sees executable card text. Original
// source faces remain evidence and are never mutated.
export function renameSourceKeywords(text, definitions) {
  return aliasSourceKeywordFaces([{name:'Source import',oracleText:text}],definitions)[0].oracleText;
}

// Reuse reviewed source layout/corrections for editable browser imports.
export function sourceDraftFaces(card) {
  return sourceFaces(card).map(separateInput);
}

export function materializeCard(card, definitions, scope = card.scope) {
  if (needsMissingFaces(card)) throw Error('The source layout requires all linked faces before expansion');
  const sources = sourceDraftFaces(card);
  // Browser drafts and source fixtures carry the same scoped token inputs
  // under their respective public spelling. Never discard one silently.
  if (card.custom_tokens && card.customTokens && JSON.stringify(card.custom_tokens) !== JSON.stringify(card.customTokens)) {
    throw Error('Conflicting source and draft supplied token definitions');
  }
  const suppliedTokens = card.custom_tokens ?? card.customTokens ?? [];
  const draft = { suppliedCardDefinitions:card.supplied_card_definitions || [], customMechanics: definitions, customTokens: suppliedTokens, layout: cohortLayout(card), token: !!card.token,
    faces: aliasSourceKeywordFaces(sources.map(({name,oracleText,typeLine}) => ({name,oracleText,typeLine,originalPrintedRulesText:oracleText})),definitions) };
  // Named token rewriting/face transport follows mechanic expansion in the ordinary frontend pipeline.
  const expandedMechanics = compileDraftInput(draft, definitions);
  const expanded = applyCustomTokens(expandedMechanics, draftTokens({...draft, faces: expandedMechanics.faces}));
  const customSubtypes = sourceSubtypeDefinitions(card, expanded, scope);
  const customTokenDefinitions = (expanded.customTokenFaces || []).map(token => {
    const typeLine = token.typeLine || [...(token.supertypes || []), ...(token.cardTypes || []),
      ...(token.subtypes?.length ? ['—', ...token.subtypes] : [])].join(' ');
    const metadata = [`Type: ${typeLine}`];
    if (token.power != null && token.toughness != null) metadata.push(`Power/Toughness: ${token.power}/${token.toughness}`);
    if (token.loyalty != null && token.loyalty !== '') metadata.push(`Loyalty: ${token.loyalty}`);
    if (token.colorIndicator?.length) metadata.push(`Color indicator: ${token.colorIndicator.join(' ')}`);
    return {name: token.name, token: true, executable_text: [...metadata, token.oracleText || ''].join('\n'),
      custom_subtypes: customSubtypes, custom_keyword_identities: token.customKeywordIdentities || [],
      custom_keyword_payments: token.customKeywordPayments || [],
      custom_keyword_counter_definitions: token.customKeywordCounterDefinitions || []};
  });
  return expanded.faces.map((face, index) => ({
    name: face.name || sources[index].name,
    ...(sourceFaces(card)[index].color_indicator != null ? {color_indicator:[...sourceFaces(card)[index].color_indicator]} : {}),
    executable_text: [...sources[index].metadata, face.oracleText].join('\n'),
    custom_keyword_identities: face.customKeywordIdentities || [],
    custom_keyword_payments: face.customKeywordPayments || [],
    supplied_card_definitions: face.suppliedCardDefinitions || card.supplied_card_definitions || [],
    custom_subtypes: customSubtypes,
    custom_token_definitions: customTokenDefinitions,
    ...(suppliedTokens.length ? { custom_tokens: suppliedTokens } : {}),
    custom_keyword_counter_definitions: face.customKeywordCounterDefinitions || card.custom_keyword_counter_definitions || [],
    ...(sources[index].auxiliaryTokens.length ? {source_auxiliary_tokens:sources[index].auxiliaryTokens} : {}),
    original_display_text: sourceDisplayText(sources[index].oracleText),
    original_printed_rules_text: face.originalPrintedRulesText,
  }));
}
