import {expandCardCreationPrograms,hasCardCreationProgram,suppliedCardDefinitions} from './reminder-card-creation.js';
import {expandNamedActivatedReferences,hasNamedActivatedReference} from './reminder-named-activated-references.js';
import { bindSelfTransformActivationLabel, expandSelfTransformActionReferences } from './reminder-transform-actions.js';
import { expandOperationPaymentAlternatives, hasOperationPaymentAlternative, expandOperationTokenGrants } from './reminder-operation-prices.js';
import {hasConditionalKeywordSharingReference,expandConditionalKeywordSharing} from './reminder-conditional-keyword-sharing.js';
import {normalizeNamedObjectReferences,expandDeclaredObjectReferences,hasDeclaredObjectReference} from './reminder-declared-object-references.js';
import {expandExileRelationshipReferences} from './reminder-exile-relationships.js';
import { expandTriggerProgramReferences } from './reminder-trigger-program-references.js';
import {normalizeResolutionProgramMultiplicity,bindResolutionProgramMultiplicity} from './reminder-resolution-recipe-policy.js';
import { expandNumberedCharacteristics } from './reminder-numbered-characteristics.js';
import { expandPermanentLinkReferences } from './reminder-permanent-links.js';
import {expandTapPaymentReferences,bindNamedTapPaymentDefinition} from './reminder-tap-payment-references.js';
import { expandGrantedPreparationAbilities } from './reminder-preparation-grants.js';
import { bindLinkedEntryExileReference } from './linked-entry-exile-references.js';
import {expandRepeatedBareDeclarations} from './reminder-repeated-declarations.js';
import { expandSubmoduleBarProgram, expandSubmoduleActivationReferences } from './reminder-submodule-bars.js';
import { expandWorkBarProgram } from './reminder-work-bars.js';
import { independentAbilityPrograms } from './reminder-definition-programs.js';
import { normalizeObjectRelation, expandSourceObjectRelationReferences } from './reminder-object-relations.js';
import { normalizeCollectionReferences, expandCollectionReferences } from './reminder-collection-references.js';
import {expandNamedTapPaymentReferences} from './reminder-named-tap-payments.js';
import { expandNamedStatusCardReferences, expandPersistentPlayerActionPrograms } from './reminder-persistent-player-programs.js';
import { expandPlayerCounterRuleDeclarations } from './reminder-player-counter-rules.js';
import {expandNamedActivationEvents, hasNamedActivationEvent} from './reminder-named-activation-events.js';
import { expandNamedActionReferences } from './reminder-action-references.js';
import {expandNamedActionState,hasNamedActionState} from './reminder-named-action-state.js';
import { namedActionProgramDefinition, expandNamedActionReplacementReferences } from './reminder-named-action-programs.js';
import {hasParticipantActionReference, expandParticipantActionReferences} from './reminder-participant-actions.js';
import { expandCompoundCastingActionGrants, bindSourceCastingPriceReferences } from './reminder-compound-prices.js';
import { bindSourcePaymentTiming } from './reminder-payment-timing.js';
import { expandChosenSetupDeclarations } from './reminder-chosen-setup.js';
import { hasTokenSizeParameters, validateTokenSizeTemplate, expandTokenSizeParameters, expandDeclaredGroupReferences } from './reminder-token-sizes.js';
import { splitReminderPriceDeclaration } from './reminder-costs.js';
import {counterAbilityDefinitions,stripCounterAbilityLabels} from './reminder-counter-definitions.js';
import { hasAbilityWordDeclaration, expandAbilityWordPrograms } from './reminder-ability-words.js';
import { expandCostProgramDeclarations, costProgramReferences, hasCostProgramDeclaration, hasCostProgramReference } from './reminder-cost-programs.js';
import { hasPrintedCastingCharacteristics, expandPrintedCastingCharacteristics } from './reminder-casting-characteristics.js';
import { expandAbilityBarProgram } from './reminder-ability-bars.js';
import { mentionsCustomKeywordPredicate, expandCustomKeywordPredicates } from './custom-keyword-predicates.js';
import { expandCounterMenus } from './reminder-counter-menus.js';
import { expandNestedCostPrograms } from './reminder-nested-costs.js';
import { expandFollowingAbilities, bindFollowingSourceAbilityReferences } from './reminder-following-abilities.js';
import { expandRecipientActions, hasRecipientAction } from './reminder-recipient-actions.js';
import { hasPrefixParameter, hasPrefixDeclaration, expandPrefixParameters } from './reminder-prefix-parameters.js';
import { expandGrantedParameterizedAbilities, hasGrantedParameterizedAbility } from './reminder-granted-parameters.js';
import { expandGrantedTriggeredPrograms } from './reminder-ability-grants.js';
import { listCustomSubtypes, withDraftSubtypes, canonicalSubtypeText } from './custom-subtypes.js';
import { expandKnownReminderDefinition } from './official-reminder-definitions.js';
import { hasReminderMaterialParameter, bindReminderMaterials, hasExileCastDefinition, bindExileCastObject, hasNextUntapCostDefinition, nextUntapCostText, hasReminderCardParameter, bindReminderCard, isActivationReminder, bindActivationReminder, bindReminderCost, hasReminderCostReference, bindReminderActivationLabel, bindReminderValueFromDeclaration, hasRepeatedCopyPayment, expandRepeatedCopyPayment, bindReminderCostFromDeclaration, hasReminderParameters, parseParameterDeclaration, bindReminderParameters, parameterExample, reminderParameters, templateParameters } from './reminder-costs.js';
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
import { OFFICIAL_KEYWORD_FORMS, officialSurfaceForms, customKeywordForms, normalizeFormOverrides, surfaceForms, deriveParticiple } from './keyword-forms.js';
import { applyCustomTokens, draftTokens } from './custom-tokens.js';
import { mentionsStatus, expandStatusActions, expandStatusGrants, expandStatusReferences } from './custom-status-mechanics.js';
import { hasActionParameters, validateActionTemplate, actionParameterPattern, expandActionParameterDeclarations } from './reminder-action-parameters.js';
import { hasInlineParameters, inlineParameterPattern, hasControlFlowOperand, repeatedActionPattern, expandRepeatedActionDeclarations, isSinglePlayerAction, playerActionPattern, expandSimplePlayerActionDeclarations } from './reminder-inline-parameters.js';
import { hasLiteralTemplate, literalTemplatePattern, hasLiteralDeclaration, expandLiteralDeclarations } from './reminder-literal-templates.js';
import { expandTypeParameterPrograms, hasTypeParameterProgram } from './reminder-parameter-programs.js';

export const mechanicKey = value => String(value || '').normalize('NFKC').replace(/[‘’]/g, "'").replace(/\s+/g, ' ').trim().toLowerCase();
export const reminderKey = value => mechanicKey(value).replace(/[“”]/g, '"');
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const official = new Set([...MTG_KEYWORD_RULES.flatMap(rule => [rule.title, ...rule.aliases]), ...officialSurfaceForms(), ...MTG_KEYWORD_RULES.filter(rule => rule.kind === 'keywordAction').map(rule => deriveParticiple(rule.title))].map(mechanicKey));
// Words a custom form may not take: an official form would make a reference
// such as "if it was kicked" ambiguous between the two keywords.
const reservedForms = new Set([...official, ...OFFICIAL_KEYWORD_FORMS.map(forms => forms.costNoun).filter(Boolean), 'mana', 'additional', 'alternative']);

function separateKeywordDeclarations(text, definitions) {
  const bare = new Set(definitions.filter(def => !['ability-word','card-creation'].includes(def.declarationKind) && !def.template
    && !hasReminderParameters(def.reminder, def.template)
    && !hasActionParameters(def)).map(def => mechanicKey(def.keyword)));
  return text.split('\n').map(line => {
    const parts = line.split(',').map(part => part.trim());
    // Only a complete list of bare declarations is a keyword line. Commas
    // inside prose, quoted grants, parameters and trigger heads stay intact.
    if (parts.length < 2 || !parts.some(part => bare.has(mechanicKey(part)))
      || !parts.every(part => bare.has(mechanicKey(part)) || official.has(mechanicKey(part)))) return line;
    return parts.join('\n');
  }).join('\n');
}

// Bounded, deterministic normalization only. Unsupported meanings still fail
// strict compilation; this is not a general natural-language code generator.
export function mechanicExpansion(reminder, declaration = '') {
  return independentAbilityPrograms(String(expandKnownReminderDefinition(reminder, declaration)).trim().replace(
    /\b(spells you cast) cost (\{?\d+\}?) less to cast until end of turn\b/gi,
    (_, subject, amount) => `${subject} this turn cost {${amount.replace(/[{}]/g, '')}} less to cast`,
  ).replace(
    // "You may pay {Cost} to draw a card" means paying buys the effect; the
    // compiler reads that as "You may pay {Cost}. If you do, draw a card."
    /\b(you may pay) (\{cost\}|(?:\{[^{}]+\})+|its [a-z][a-z '-]* cost) to (?!cast\b|take a special action\b)([^.!?]+)([.!?]|$)/gi,
    (_, may, price, effect) => `${may} ${price}. If you do, ${effect}.`,
  )).join('\n');
}

// The name may carry the card line's layout after the keyword ("Behold a
// {Type}", "Forge {Number} {Cost}"): the keyword is the text before the first
// placeholder, less an article that belongs to the layout; the rest is its
// declaration template.
export function splitMechanicName(name) {
  const text = String(name || '').normalize('NFKC').replace(/\s+/g, ' ').trim();
  const brace = text.indexOf('{');
  if (brace < 0) return { keyword: mechanicKey(text), template: null };
  const head = text.slice(0, brace).replace(/[\s—–-]+$/, '').replace(/\s+an?$/i, '');
  return { keyword: mechanicKey(head), template: text.slice(head.length).trim() || null };
}

function validateTemplate(keyword, template, reminder) {
  if (hasTokenSizeParameters({template,reminder})) return validateTokenSizeTemplate(keyword,template,reminder);
  if (hasLiteralTemplate({ template }) && /[\r\n]/.test(template)) throw new Error('A literal declaration layout must fit on one line.');
  if (hasActionParameters({ template, reminder })) return validateActionTemplate(keyword, template, reminder);
  if (hasLiteralTemplate({ template })) {
    const used = reminderParameters(reminder);
    if (used.length) throw new Error(`"${keyword}"'s literal layout cannot bind reminder parameters.`);
    return;
  }
  const named = templateParameters(template);
  const unknown = String(template).match(/\{(?!(?:cost|number|type)\})([^{}]*)\}/i);
  if (unknown) throw new Error(`"{${unknown[1]}}" is not a parameter. Use {Cost}, {Number} or {Type} in "${keyword}"'s layout.`);
  const repeated = named.find((name, index) => named.indexOf(name) !== index);
  if (repeated) throw new Error(`"${keyword}"'s layout names {${repeated[0].toUpperCase()}${repeated.slice(1)}} more than once.`);
  if (/[^a-z{}\s—–-]/i.test(String(template).replace(/\{[a-z]+\}/gi, ''))) throw new Error(`"${keyword}"'s layout may hold only its parameters, words, spaces and dashes.`);
  const gaps = String(template).split(/\{(?:cost|number|type)\}/i).slice(1, -1);
  if (gaps.some(gap => !gap)) throw new Error(`"${keyword}"'s layout needs a space or a dash between two parameters.`);
  const used = reminderParameters(reminder);
  const missing = used.find(name => !named.includes(name));
  if (missing) throw new Error(`The reminder uses {${missing[0].toUpperCase()}${missing.slice(1)}}, so "${keyword}"'s layout must place it.`);
  // A price may also be named as "its <keyword> cost".
  const unused = named.find(name => !used.includes(name) && !(name === 'cost' && hasReminderCostReference(reminder, keyword)));
  if (unused) throw new Error(`"${keyword}"'s layout places {${unused[0].toUpperCase()}${unused.slice(1)}}, but the reminder never uses it.`);
}

export function validateMechanic(definition) {
  const split = splitMechanicName(definition.keyword);
  const keyword = split.keyword;
  const template = split.template || (definition.template ? String(definition.template).trim() : null);
  const reminder = String(definition.reminder || '').trim();
  // Provenance of a generated ability is explicit data. It never follows the
  // permanent's current marker or guesses which counter effect resembles it.
  const abilityDesignation = definition.abilityDesignation == null ? null : mechanicKey(definition.abilityDesignation);
  if (abilityDesignation !== null && (abilityDesignation !== keyword || !/^[a-z]+(?: [a-z]+){0,4}$/.test(abilityDesignation))) {
    throw new Error('An ability designation must preserve this custom keyword name as one to five plain words.');
  }
  let statusPredicate;
  if (definition.statusPredicate != null) {
    const value=definition.statusPredicate;
    if (!value || typeof value!=='object' || Array.isArray(value)
      || Object.keys(value).some(key=>!['spellCopy','customKeyword'].includes(key))
      || value.spellCopy!==true || typeof value.customKeyword!=='string'
      || !/^[a-z][a-z '-]{0,63}$/i.test(value.customKeyword.trim())) throw new Error('A status predicate needs an explicit spell-copy property and exact custom keyword identity.');
    statusPredicate={spellCopy:true,customKeyword:mechanicKey(value.customKeyword)};
  }
  const declarationKind = definition.declarationKind == null ? null : String(definition.declarationKind);
  if (declarationKind !== null && !['ability-word','card-creation'].includes(declarationKind)) throw new Error('Unknown custom declaration kind.');
  if (declarationKind === 'card-creation' && (template || definition.prefixTemplate)) throw new Error('A card-creation declaration uses explicit supplied identities and destinations, without an operand template.');
  if (declarationKind === 'ability-word' && (template || definition.prefixTemplate || /[{}\r\n]/.test(reminder))) throw new Error('An ability-word heading needs a plain native heading, without operand templates or line breaks.');
  const programSelectionRules = definition.programSelectionRules ?? [];
  if (!Array.isArray(programSelectionRules) || programSelectionRules.some(rule => typeof rule !== 'string' || !rule.trim() || /[\r\n]/.test(rule))
      || programSelectionRules.length && declarationKind !== 'ability-word') throw new Error('Program selection rules need complete independent native lines on an ability-word declaration.');
  const triggerReferences = definition.triggerReferences ?? null;
  if (triggerReferences && (declarationKind !== 'ability-word' || typeof triggerReferences !== 'object'
      || !/^[a-z][a-z '-]{0,63}$/i.test(triggerReferences.class || '') || !/^[a-z][a-z '-]{0,63}$/i.test(triggerReferences.noun || ''))) throw new Error('Trigger references require an explicitly named class and noun on an ability-word declaration.');
  const linkedExileVerb=definition.linkedExileVerb==null?null:mechanicKey(definition.linkedExileVerb);
  if(linkedExileVerb && (!/^[a-z][a-z '-]{0,63}$/.test(linkedExileVerb)||reservedForms.has(linkedExileVerb)))throw new Error('A linked exile verb must be an explicit custom action name.');
  const linkedExileScope=definition.linkedExileScope==null?'entry':mechanicKey(definition.linkedExileScope);
  if(!['entry','source'].includes(linkedExileScope)||(linkedExileScope==='source'&&!linkedExileVerb))throw new Error('A linked exile scope must name entry or an explicitly declared source relation.');
  const paymentKind = definition.paymentKind == null ? null : String(definition.paymentKind);
  const textAdditionVerb=definition.textAdditionVerb==null?null:mechanicKey(definition.textAdditionVerb);
  if(textAdditionVerb && (paymentKind!=='text-addition' || !/^[a-z][a-z '-]{0,63}$/.test(textAdditionVerb) || reservedForms.has(textAdditionVerb)))throw new Error('A text-addition verb requires a custom text-addition payment definition.');
  if (paymentKind !== null && paymentKind !== 'text-addition') throw new Error('Unknown custom payment kind.');
  if (paymentKind && !/\{Cost\}/i.test(reminder)) throw new Error('A text-addition payment requires an explicit {Cost} operand.');
  const objectRelation = normalizeObjectRelation(definition.objectRelation);
  const collectionReferences = normalizeCollectionReferences(definition.collectionReferences);
  const sourceScope = definition.sourceScope == null ? null : String(definition.sourceScope).trim();
  const appliesToSubtype = definition.appliesToSubtype == null ? null : canonicalSubtypeText(definition.appliesToSubtype);
  if (appliesToSubtype !== null && (!appliesToSubtype || new TextEncoder().encode(appliesToSubtype).length > 32)) throw new Error('A subtype-scoped definition needs its exact subtype name.');
  if (sourceScope !== null && !/^[a-z][a-z0-9-]{0,63}$/.test(sourceScope)) throw new Error('A source scope must be an explicit lowercase namespace.');
  const officialCostDeclaration = definition.officialCostDeclaration === true
    && template === '{Cost}' && /^\{Cost\}\s*:/i.test(reminder);
  if (!/^[a-z][a-z '-]{0,63}$/.test(keyword) || reservedForms.has(keyword)) {
    throw new Error(`"${keyword}" cannot be defined as a custom mechanic. Use a distinct custom keyword; official keywords cannot be redefined.`);
  }
  if (Object.hasOwn(definition, 'actionProgram') && typeof definition.actionProgram !== 'boolean') throw new Error('actionProgram must be an explicit boolean declaration.');
  if (Object.hasOwn(definition, 'perObjectProgram') && typeof definition.perObjectProgram !== 'boolean') throw new Error('perObjectProgram must be an explicit boolean declaration.');
  if (definition.perObjectProgram && (!/\{object\}/i.test(template || '') || /\{(?:subject|player|ability|name)\}/i.test(template || ''))) throw new Error('A per-object program requires exactly one typed Object operand.');
  if (Object.hasOwn(definition,'collectiveObject') && typeof definition.collectiveObject !== 'boolean') throw new Error('collectiveObject must be an explicit boolean declaration.');
  if (definition.collectiveObject && (definition.actionProgram !== true || String(definition.template || '').toLowerCase() !== '{object}')) throw new Error('collectiveObject needs an explicit actionProgram and one {Object} layout.');
  if (definition.completionRequirement != null && (definition.completionRequirement !== 'all-choices-available' || definition.actionProgram !== true || definition.collectiveObject !== true)) throw new Error('A completion requirement needs an explicit collective action program and the all-choices-available contract.');
  if (!reminder || reminder.length > 2000 || /[()]/.test(reminder)) {
    throw new Error(`"${keyword}" needs one unambiguous parenthetical reminder explaining its action.`);
  }
  // Braces hold mana symbols or the declaration parameters.
  const unknown = reminder.match(/\{(?!(?:cost|number|type|object|ability|name|player|subject|counterKinds|tokenSizes)\})([a-z][a-z ]{2,})\}/i);
  if (unknown) throw new Error(`"{${unknown[1]}}" is not a parameter. Use {Cost} for a price, {Number} for an amount and {Type} for a card type, as in "${keyword} 3—{1}{U}".`);
  if (definition.prefixTemplate && definition.prefixTemplate !== '{Type}') throw new Error('A prefix layout must be {Type}.');
  if (definition.prefixTemplate && (template || !/\{Type\}/i.test(reminder) || /\{(?:Cost|Number)\}/i.test(reminder))) throw new Error('A type prefix requires only a {Type} reminder operand.');
  if (!definition.prefixTemplate && (template || hasActionParameters({ reminder }))) validateTemplate(keyword, template, reminder);
  const paymentReminder = definition.paymentReminder == null ? null : String(definition.paymentReminder).trim();
  if (paymentReminder !== null) {
    if (!paymentReminder || paymentReminder.length > 2000 || /[()]/.test(paymentReminder)
        || !hasActionParameters({template, reminder})) throw new Error('A payment reminder requires the same explicit action operands as its effect reminder.');
    validateActionTemplate(keyword, template, paymentReminder);
  }
  if (definition.costProgram && (/["“”\r\n]/.test(reminder) || template?.toLowerCase() !== '{number}')) throw new Error('A cost program needs one {Number} layout and an unquoted single-line body.');
  const operationPaymentAlternative = definition.operationPaymentAlternative;
  if (operationPaymentAlternative != null && (typeof operationPaymentAlternative !== 'object' || !/^[a-z][a-z ,'-]{0,180}$/i.test(operationPaymentAlternative.defaultPhrase || ''))) throw new Error('An operation payment alternative needs its explicit complete default price phrase.');
  const paymentReferences = definition.paymentReferences || [];
  if (!Array.isArray(paymentReferences) || paymentReferences.some(reference => !reference || !/^[a-z][a-z '-]{0,40}$/i.test(reference.subject || '') || !/^[a-z][a-z '-]{0,40}$/i.test(reference.participle || '') || reference.countUnit != null && !/^[a-z][a-z '-]{0,40}$/i.test(reference.countUnit))) throw new Error('Payment references need a subject and participle.');
  const resultReference = definition.resultReference == null ? null : String(definition.resultReference).trim().toLowerCase();
  if (resultReference && (!hasActionParameters({ template, reminder }) || !/^the (?:revealed|chosen|moved|exiled|returned|affected) (?:card|creature|permanent|object)s?$/.test(resultReference))) throw new Error('An action result reference must explicitly name its final affected object.');
  const namedObjectReferences = normalizeNamedObjectReferences(definition.namedObjectReferences);
  const resolutionProgramMultiplicity=normalizeResolutionProgramMultiplicity(definition.resolutionProgramMultiplicity,reminder);
  if (Object.hasOwn(definition, 'numberedCharacteristics') && typeof definition.numberedCharacteristics !== 'boolean') throw new Error('numberedCharacteristics must be an explicit boolean declaration.');
  if (definition.numberedCharacteristics === true && !(template === '{Cost}' && /Choose a number for this permanent\./i.test(reminder))) throw new Error('Numbered characteristics require an explicit paid permanent number choice.');
  if (definition.permanentLinkRelation != null && (typeof definition.permanentLinkRelation !== 'string' || !/^[a-z][a-z '-]{0,63}$/.test(definition.permanentLinkRelation))) throw new Error('A permanent link relation must be a plain source-defined name.');
  const overrides = normalizeFormOverrides(definition.forms);
  const forms = customKeywordForms({ keyword, forms: overrides });
  for (const form of [forms.participle, forms.designation, forms.costNoun, forms.eventVerb, forms.eventLabel, forms.eventPaymentNoun, ...forms.variants]) {
    if (form && form !== keyword && reservedForms.has(form)) {
      throw new Error(`"${keyword}" cannot use "${form}" as one of its forms; it belongs to an official keyword. Override it in the mechanic's forms.`);
    }
  }
  const expansionBase = bindResolutionProgramMultiplicity(bindSelfTransformActivationLabel(mechanicExpansion(bindReminderActivationLabel(reminder, keyword)), {keyword,reminder}), resolutionProgramMultiplicity);
  const entryExpansion = linkedExileVerb && /^As (?:this(?: (?:creature|permanent|card))?) enters\b/i.test(expansionBase)
    ? `Keyword ability "${keyword}" — ${expansionBase}` : expansionBase;
  const expansion = abilityDesignation ? designatedReminder(entryExpansion, abilityDesignation) : entryExpansion;
  const attribution = definition.attribution && typeof definition.attribution === 'object'
    ? Object.fromEntries(['sourceFormat','setCode','setName','collectorNumber','artist','originalArtist','designer','sourceCardKey','sourceUrl'].filter(key=>typeof definition.attribution[key] === 'string').map(key=>[key,definition.attribution[key]])) : null;
  return { ...(attribution ? {attribution} : {}), ...(statusPredicate ? {statusPredicate} : {}), ...(operationPaymentAlternative ? {operationPaymentAlternative: {defaultPhrase: operationPaymentAlternative.defaultPhrase}} : {}), ...(collectionReferences ? { collectionReferences } : {}), ...(objectRelation ? { objectRelation } : {}), ...(definition.costProgram ? {costProgram: true, paymentReferences} : {}), keyword, reminder, ...(resolutionProgramMultiplicity ? {resolutionProgramMultiplicity} : {}), ...(namedObjectReferences.length ? {namedObjectReferences} : {}), ...(definition.collectiveObject === true ? {collectiveObject:true} : {}), ...(definition.completionRequirement ? {completionRequirement:definition.completionRequirement} : {}), ...(abilityDesignation?{abilityDesignation}:{}), expansion, sourceCard: String(definition.sourceCard || 'an earlier card'), version: 1, ...(definition.actionProgram === true ? { actionProgram: true } : {}), ...(definition.perObjectProgram === true ? { perObjectProgram: true } : {}), ...(definition.numberedCharacteristics === true ? { numberedCharacteristics: true } : {}), ...(definition.permanentLinkRelation ? { permanentLinkRelation: definition.permanentLinkRelation } : {}), ...(declarationKind ? { declarationKind } : {}), ...(programSelectionRules.length ? { programSelectionRules: programSelectionRules.map(rule => rule.trim()) } : {}), ...(triggerReferences ? { triggerReferences: { class: triggerReferences.class.toLowerCase(), noun: triggerReferences.noun.toLowerCase() } } : {}), ...(template ? { template } : {}), ...(paymentReminder ? { paymentReminder } : {}), ...(officialCostDeclaration ? { officialCostDeclaration: true } : {}), ...(paymentKind ? {paymentKind} : {}), ...(textAdditionVerb ? {textAdditionVerb} : {}), ...(linkedExileVerb ? {linkedExileVerb} : {}), ...(linkedExileScope === 'source' ? {linkedExileScope} : {}), ...(sourceScope ? { sourceScope } : {}), ...(appliesToSubtype ? { appliesToSubtype } : {}), ...(definition.prefixTemplate ? { prefixTemplate: definition.prefixTemplate } : {}), ...(resultReference ? { resultReference } : {}), ...(overrides ? { forms: overrides } : {}) };

}

function designatedReminder(reminder,designation){
  return independentAbilityPrograms(String(reminder)).map(program=>{
    if(/^Keyword ability "/i.test(program))return program;
    for(let index=0;index<program.length;index++){
      if(program[index]!==':'||!outsideQuotation(program,index))continue;
      return `Keyword ability "${designation}" — ${program}`;
    }
    return `${designation} — ${program}`;
  }).join('\n');
}

// Every word or phrase that names a mechanic: its name and each form.
const namingForms = def => { const forms = customKeywordForms(def); return [...surfaceForms(forms), forms.costNoun]; };

// A second custom keyword may not reuse another's name or forms; references
// such as "if it was blinked" must name exactly one mechanic.
export function mechanicFormClash(definitions, incoming) {
  const words = new Set(namingForms(incoming));
  for (const def of definitions) {
    if (def.keyword === incoming.keyword) continue;
    const shared = namingForms(def).find(form => words.has(form));
    if (shared) return new Error(`Custom mechanic "${incoming.keyword}" cannot use "${shared}": "${def.keyword}" (from ${def.sourceCard}) already uses it. Give one of them a different form.`);
  }
  return null;
}

export function mechanicConflict(existing, incoming) {
  return new Error(`Custom mechanic "${incoming.keyword}" could not be added: "${existing.sourceCard}" already defined it as "${existing.reminder}". This card defines it as "${incoming.reminder}". Definitions are retained even when a card fails to compile. Use the existing reminder text, or rename this mechanic everywhere in the card to give it a different meaning.`);
}

export function mergeMechanics(existing, incoming) {
  const definitions = new Map(existing.map(value => { const def = validateMechanic(value); return [def.keyword, def]; }));
  for (const value of incoming) {
    const def = validateMechanic(value), previous = definitions.get(def.keyword);
    if (previous && Object.hasOwn(value,'objectRelation')
      && JSON.stringify(previous.objectRelation || null) !== JSON.stringify(def.objectRelation || null)) throw mechanicConflict(previous,def);
    if (previous && value.collectionReferences !== undefined
      && JSON.stringify(previous.collectionReferences || []) !== JSON.stringify(def.collectionReferences || [])) throw mechanicConflict(previous, def);
    if (previous && (previous.appliesToSubtype !== def.appliesToSubtype || reminderKey(previous.reminder) !== reminderKey(def.reminder)
      || (def.paymentReminder && reminderKey(previous.paymentReminder || '') !== reminderKey(def.paymentReminder)))) throw mechanicConflict(previous, def);
    // Every explicitly supplied executable policy participates in identity.
    // Missing inline metadata retains the known source definition.
    const policyValue = (definition, field) => {
      if (field === 'collectiveObject' || field === 'numberedCharacteristics') return !!definition[field];
      if (field === 'operationPaymentAlternative') return definition[field]
        ? {defaultPhrase: mechanicKey(definition[field].defaultPhrase)} : null;
      return definition[field] || null;
    };
    if (previous) for (const field of ['collectiveObject', 'completionRequirement', 'operationPaymentAlternative', 'numberedCharacteristics', 'permanentLinkRelation']) {
      if (Object.hasOwn(value, field) && JSON.stringify(policyValue(previous, field)) !== JSON.stringify(policyValue(def, field))) throw mechanicConflict(previous, def);
    }
    if (previous && Object.hasOwn(value, 'perObjectProgram') && !!previous.perObjectProgram !== !!def.perObjectProgram) throw new Error(`Custom mechanic "${def.keyword}" has conflicting perObjectProgram declarations.`);
    if (previous && Object.hasOwn(value, 'abilityDesignation')
      && (previous.abilityDesignation || null) !== (def.abilityDesignation || null)) throw mechanicConflict(previous, def);
    if (previous && Object.hasOwn(value, 'forms') && JSON.stringify(customKeywordForms(previous)) !== JSON.stringify(customKeywordForms(def))) throw mechanicConflict(previous, def);
    if (previous && (JSON.stringify(previous.programSelectionRules || []) !== JSON.stringify(def.programSelectionRules || [])
      || JSON.stringify(previous.triggerReferences || null) !== JSON.stringify(def.triggerReferences || null))) throw mechanicConflict(previous, def);
    if (previous && Object.hasOwn(value,'resolutionProgramMultiplicity') && JSON.stringify(previous.resolutionProgramMultiplicity || null) !== JSON.stringify(def.resolutionProgramMultiplicity || null)) throw mechanicConflict(previous,def);
    if (previous && Object.hasOwn(value,'statusPredicate') && JSON.stringify(previous.statusPredicate || null)!==JSON.stringify(def.statusPredicate || null)) throw mechanicConflict(previous,def);
    if (previous && Object.hasOwn(value, 'actionProgram') && !!previous.actionProgram !== !!def.actionProgram) throw new Error(`Custom mechanic "${def.keyword}" has conflicting actionProgram declarations.`);
      if (previous && Object.hasOwn(value, 'namedObjectReferences') && JSON.stringify(previous.namedObjectReferences || []) !== JSON.stringify(def.namedObjectReferences || [])) throw mechanicConflict(previous, def);
    if (!previous) {
      const clash = mechanicFormClash([...definitions.values()], def);
      if (clash) throw clash;
      definitions.set(def.keyword, def);
    }
  }
  return [...definitions.values()];
}

function actionPattern(keyword) {
  // Restrict substitution to complete action clauses, never card names, quoted
  // grants, or a keyword embedded in another word. Costed standalone
  // declarations use mechanicPattern instead.
  return new RegExp(`(^|[,.:;•—–]\\s*|\\b(?:then|and|or)\\s+|\\bunless\\s+you\\s+)(${escape(keyword)})(?=\\s*(?:[,.;(:]|or\\s+pay\\b|$))`, 'gim');
}

function costedPattern(keyword) {
  return new RegExp(`(^[ \t]*)(${escape(keyword)})(?=[ \t—–]|$)(?:[ \t]*[—–][ \t]*|[ \t]+)?([^\n]*)$`, 'gim');
}

function valuePattern(keyword) {
  return new RegExp(`(^[ \t]*)(${escape(keyword)})[ \t]+([XYZ],\\s*where\\s+[XYZ]\\s+is\\s+[^\\n]+)$`, 'gim');
}

function cardParameterPattern(keyword) {
  return new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(${escape(keyword)})\\s+(the top card of your library|one of them|one of those cards)(?=\\s*(?:[,.;(]|$))`, 'gim');
}

function exileObjectPattern(keyword) {
  return new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(${escape(keyword)})\\s+([^.!?(),:\\n]+)`, 'gim');
}

function selfCostPattern(keyword) {
  return new RegExp(`(^|,\\s*)(${escape(keyword)})\\s+this creature(?=\\s*:)`, 'gim');
}

function mechanicPattern(definition) {
  if (definition.declarationKind === 'ability-word') return new RegExp(`(^[ \t]*)(${escape(definition.keyword)})[ \t]*[—–][ \t]*([^\n]+)$`, 'gim');
  if (definition.officialCostDeclaration) return new RegExp(`(^[ \\t]*)(${escape(definition.keyword)})(?:[ \\t]*[—–][ \\t]*|[ \\t]+)(?=[^\\n]*\\S)([^\\n]+)$`, 'gim');
  if (hasPrintedCastingCharacteristics(definition.reminder)) return costedPattern(definition.keyword);
  if (hasLiteralTemplate(definition)) return literalTemplatePattern(definition);
  if (hasActionParameters(definition)) return actionParameterPattern(definition);
  if (hasInlineParameters(definition)) return inlineParameterPattern(definition);
  if (hasReminderParameters(definition.reminder, definition.template)) return costedPattern(definition.keyword);
  if (hasExileCastDefinition(definition.reminder)) return exileObjectPattern(definition.keyword);
  if (hasNextUntapCostDefinition(definition.reminder)) return selfCostPattern(definition.keyword);
  if (hasReminderCardParameter(definition.reminder)) return cardParameterPattern(definition.keyword);
  return hasReminderMaterialParameter(definition.reminder) || isActivationReminder(definition.reminder) || hasReminderCostReference(definition.reminder, definition.keyword, customKeywordForms(definition)) || hasRepeatedCopyPayment(definition.reminder)
    ? costedPattern(definition.keyword) : actionPattern(definition.keyword);
}

function stripReminders(text) {
  let depth = 0, result = '', italicOpening = null;
  for (let index = 0; index < text.length; index++) {
    const character = text[index];
    if (character === '(') {
      if (!depth) {
        // Remove only the paired, adjacent italic delimiters belonging to
        // this reminder. Ordinary stars (including */* characteristics) stay.
        italicOpening = text[index - 1] === '*' && (index === 1 || /[\s.,;:!?]/.test(text[index - 2]))
          ? result.length - 1 : null;
      }
      depth++;
    } else if (character === ')') {
      if (!depth) throw new Error('Unbalanced reminder parentheses. Close each reminder before saving.');
      depth--;
      if (!depth) {
        if (italicOpening !== null && text[index + 1] === '*'
            && (index + 2 === text.length || /[\s.,;:!?]/.test(text[index + 2]))) {
          result = result.slice(0, italicOpening) + result.slice(italicOpening + 1);
          index++;
        }
        italicOpening = null;
      }
    } else if (!depth || character === '\n') result += character;
  }
  if (depth) throw new Error('Unbalanced reminder parentheses. Close each reminder before saving.');
  return result;
}

function outsideQuotation(text, offset) {
  return (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
}

function definingReminders(face, keyword) {
  const lines = String(face.oracleText || '').split('\n');
  const found = [];
  for (let index = 0; index < lines.length; index++) {
    const semantic = lines[index].replace(/\([^()]*\)/g, '');
    if (![...semantic.matchAll(actionPattern(keyword)), ...semantic.matchAll(costedPattern(keyword)), ...semantic.matchAll(cardParameterPattern(keyword)), ...semantic.matchAll(selfCostPattern(keyword)), ...semantic.matchAll(exileObjectPattern(keyword))]
      .some(match => outsideQuotation(semantic, match.index))) continue;
    let next = index + 1;
    while (next < lines.length && !lines[next].trim()) next++;
    const adjacent = lines[index] + (/^\s*\(/.test(lines[next] || '') ? lines[next] : '');
    if (/\([^)]*\(/.test(adjacent)) throw new Error(`"${keyword}" needs a single reminder without nested parentheses; write its full meaning in that reminder.`);
    const reminders = [...adjacent.matchAll(/\(([^()]*)\)/g)];
    if (reminders.length > 1) throw new Error(`"${keyword}" has more than one possible reminder. Put its single definition in parentheses on the same line as the action, or immediately below it.`);
    if (reminders.length === 1) found.push({ keyword, reminder: reminders[0][1].trim(), sourceCard: face.name, sourceLine: index });
  }
  return found;
}

// Scoped library definitions require this draft's namespace or an explicit
// definition carried by this source. Merely sharing a keyword is insufficient.
function definitionAppliesToDraft(definition, draft) {
  if (!definition.sourceScope) return true;
  if (draft.sourceScope === definition.sourceScope) return true;
  return (draft.customMechanics || []).some(local => local.sourceScope === definition.sourceScope
    && mechanicKey(local.keyword) === mechanicKey(definition.keyword)
    && reminderKey(local.reminder) === reminderKey(definition.reminder));
}

export function draftMechanics(draft, known = []) {
  const supplied = mergeMechanics(known.filter(def => definitionAppliesToDraft(def, draft)), draft.customMechanics || []);
  const declared = supplied.filter(def=>def.declarationKind!=='card-creation').flatMap(def => draft.faces.flatMap(face => definingReminders(face, def.keyword)
    .map(declaration => def.sourceScope ? { ...declaration, sourceScope: def.sourceScope } : declaration)));
  return mergeMechanics(supplied, declared).filter(def => draft.faces.some(face => {
    const semantic = String(face.oracleText || '').replace(/\([^()]*\)/g, '');
    return expandSubmoduleActivationReferences(semantic,[def]) !== semantic || hasCardCreationProgram(semantic,def,[...suppliedCardDefinitions(draft),...(!draft.token?[{name:face.name}]:[])],face.name) || hasDeclaredObjectReference(semantic, def) || expandNamedActionReferences(semantic, [def], face.name) !== semantic || hasNamedActivatedReference(semantic,def) || hasCostProgramDeclaration(semantic, def) || hasCostProgramReference(semantic, def) || (hasLiteralTemplate(def) ? hasLiteralDeclaration(semantic, def) : mechanicPattern(def).test(semantic)) || hasTypeParameterProgram(semantic, def) || expandSimplePlayerActionDeclarations(semantic, def) !== semantic || expandPersistentPlayerActionPrograms(semantic, def) !== semantic || hasRecipientAction(semantic, def, face.name) || hasGrantedParameterizedAbility(semantic, def) || designationPattern(def).test(semantic) || paymentCountPattern(def).test(semantic) || mentionsStatus(semantic, def) || hasParticipantActionReference(semantic, def) || mentionsCustomKeywordPredicate(semantic, def) || hasConditionalKeywordSharingReference(semantic, def, face.name);
  }));
}

// Expand actions inside a quoted triggered ability only when the outer text
// explicitly grants that ability. Names and other quoted text stay opaque.
function expandGrantedTriggeredActions(text, definitions) {
  const actions = definitions.filter(def => hasActionParameters(def) || hasInlineParameters(def) || (!def.template
    && !hasReminderParameters(def.reminder, def.template)));
  if (!actions.length) return text;
  return text.replace(/"([^"\n]*)"|“([^”\n]*)”/g, (quoted, straight, curly, offset) => {
    const body = straight ?? curly;
    const grantPrefix = text.slice(0, offset).replace(/"[^"\n]*"|“[^”\n]*”/g, '');
    if (!(/\b(?:has|have|gains?|gain)\s*$/i.test(grantPrefix)
      || /\b(?:becomes?|is|are)\b[^.\n"“”]*\bwith\b[^.\n"“”]*$/i.test(grantPrefix))
      || !/^(?:When|Whenever|At)\b/i.test(body)) return quoted;
    const expanded = compileDraftInput({ customMechanics: actions, faces: [{ name: '', oracleText: body }] }, actions).faces[0].oracleText;
    return straight === undefined ? `“${expanded}”` : `"${expanded}"`;
  });
}

function subtypeScopeApplies(definition, face) {
  const subtype = definition.appliesToSubtype;
  if (!subtype) return false;
  if ((face.subtypes || []).some(name => canonicalSubtypeText(name) === subtype)) return true;
  const tail = canonicalSubtypeText(String(face.typeLine || '').split(/[—–]/).slice(1).join(' '));
  return new RegExp(`(?<![\\p{L}\\p{N}])${escape(subtype)}(?![\\p{L}\\p{N}])`, 'u').test(tail);
}

export function compileDraftInput(draft, definitions, options = {}) {
  definitions = definitions.map(def => ({...def, ...validateMechanic(def)}));
  const suppliedInputs=suppliedCardDefinitions(draft);
  // Expand a flat source node once. The inner pass can see every declared name
  // but never traverses dependency records, so self and mutual references stay finite.
  const suppliedDefinitions=options.expandSuppliedDefinitions === false ? suppliedInputs : suppliedInputs.map(record => {
    const lines=String(record.text).split('\n');
    const header=lines.filter(line=>/^(?:Mana cost|Type|Power\/Toughness|Loyalty|Defense|Color indicator):/.test(line));
    const oracleText=lines.filter(line=>!/^(?:Mana cost|Type|Power\/Toughness|Loyalty|Defense|Color indicator):/.test(line)).join('\n');
    const ownDefinitions=mergeMechanics(definitions,record.customMechanics||[]);
    const inner=compileDraftInput({
      name:record.name, sourceCardKey:record.sourceCardKey, scope:record.scope||draft.scope,
      suppliedCardDefinitions:suppliedInputs, customMechanics:ownDefinitions,
      customSubtypes:record.customSubtypes||[],
      faces:[{name:record.name,typeLine:header.find(line=>/^Type:/.test(line))?.slice(5).trim()||'',oracleText}],
    },ownDefinitions,{expandSuppliedDefinitions:false}).faces[0];
    return {...record,text:[...header,inner.oracleText].join('\n'),
      ...(inner.customKeywordIdentities?.length?{customKeywordIdentities:inner.customKeywordIdentities}:{}),
      ...(inner.customKeywordPayments?.length?{customKeywordPayments:inner.customKeywordPayments}:{}),
      ...(inner.customKeywordCounterDefinitions?.length?{customKeywordCounterDefinitions:inner.customKeywordCounterDefinitions}:{}),
    };
  });
  definitions = definitions.filter(def => definitionAppliesToDraft(def, draft)).map(bindNamedTapPaymentDefinition);
  const originalDefinitions = definitions;
  return { ...draft, ...(suppliedDefinitions.length?{suppliedCardDefinitions:suppliedDefinitions}:{}), customMechanics: definitions, faces: draft.faces.map(face => {
    const originalText = String(face.oracleText || '');
    let text = expandNumberedCharacteristics(originalText, originalDefinitions, face.name);
    const semantic = text.replace(/\([^()]*\)/g, '');
    const authoredSourceText = stripReminders(originalText).split('\n').map(line => line.trim()).filter(Boolean).join('\n');
    const playerCounterSource = expandPlayerCounterRuleDeclarations(text);
    const hasPlayerCounterRule = playerCounterSource !== text;
    const sourceText = expandRepeatedBareDeclarations(stripReminders(playerCounterSource).split('\n').map(line => line.trim()).filter(Boolean).join('\n'), originalDefinitions);
    const scoped = originalDefinitions.filter(def => subtypeScopeApplies(def, face));
    let layoutText = expandDeclaredObjectReferences(sourceText, originalDefinitions);
    const layoutCounterDefinitions = [];
    for (const def of scoped) {
      const ordinary = originalDefinitions.filter(candidate => !candidate.appliesToSubtype);
      const expanded = expandSubmoduleBarProgram(layoutText, def) ?? expandWorkBarProgram(layoutText, def, body => {
        const nested = compileDraftInput({customMechanics: ordinary, sourceScope: draft.sourceScope,
          faces: [{name: face.name, oracleText: body}]}, ordinary).faces[0];
        layoutCounterDefinitions.push(...(nested.customKeywordCounterDefinitions || []));
        return nested;
      });
      if (expanded === null) throw new Error('A subtype-scoped definition needs a supported complete layout program.');
      layoutText = expanded;
    }
    const submoduleText = expandSubmoduleActivationReferences(layoutText, originalDefinitions);
    const relationText = expandSourceObjectRelationReferences(expandCollectionReferences(submoduleText, originalDefinitions), originalDefinitions);
    const permanentLinkText = expandPermanentLinkReferences(relationText, originalDefinitions);
    const tapPaymentText = expandNamedTapPaymentReferences(permanentLinkText, originalDefinitions);
    const namedActivatedText=originalDefinitions.reduce((value,definition)=>expandNamedActivatedReferences(value,definition),expandTapPaymentReferences(tapPaymentText,originalDefinitions,face.name));
    const sourcePrices = bindSourceCastingPriceReferences(namedActivatedText, originalDefinitions, face.name || '');
    const paymentTiming = bindSourcePaymentTiming(sourcePrices, originalDefinitions, face.name || '');
    const definitions = paymentTiming.definitions;
    const compoundText = expandCompoundCastingActionGrants(paymentTiming.text, definitions);
    const creationText=expandCardCreationPrograms(compoundText,definitions,[...suppliedDefinitions,...(!draft.token?[{name:face.name}]:[])],face.name);
    const operationPrices = expandOperationTokenGrants(expandOperationPaymentAlternatives(creationText, definitions), definitions);
    const replacementText = expandNamedActionReplacementReferences(operationPrices, definitions);
    const statusText = expandNamedStatusCardReferences(replacementText, definitions);
    const preparedGrants = expandGrantedPreparationAbilities(statusText, definitions);
    const relationshipText=definitions.reduce((text,definition)=>expandExileRelationshipReferences(text,definition,face.name||''),preparedGrants);
    const nestedText = expandNestedCostPrograms(relationshipText, definitions);
    const sharingText = expandConditionalKeywordSharing(nestedText, definitions, face.name);
    const parameterizedText = expandGrantedParameterizedAbilities(sharingText, definitions);
    const actionText = expandGrantedTriggeredActions(parameterizedText, definitions);
    const triggeredText = expandGrantedTriggeredPrograms(actionText, definitions);
    const grantedPrograms = expandDeclaredGroupReferences(expandNamedActionReferences(triggeredText, definitions, face.name), definitions, face.name);
    const grantedText = definitions.reduce((text, definition) => expandTriggerProgramReferences(text, definition, face.name), grantedPrograms);
    const directSemantic = grantedText.replace(/"[^"\n]*"|“[^”\n]*”/g, '');
    const counterDefinitions=counterAbilityDefinitions(sourceText,definitions);
    const counterFreeSemantic=stripCounterAbilityLabels(directSemantic,counterDefinitions,definitions);
    const used = definitions.filter(def => hasCostProgramDeclaration(directSemantic, def) || (def.declarationKind === 'ability-word' ? hasAbilityWordDeclaration(directSemantic, def) : (hasPrefixParameter(def) ? hasPrefixDeclaration(directSemantic, def) : hasLiteralTemplate(def) ? hasLiteralDeclaration(directSemantic, def) : (hasActionParameters(def)
      ? expandActionParameterDeclarations(directSemantic, def, face.name) !== directSemantic
      : mechanicPattern(def).test(directSemantic)) || valuePattern(def.keyword).test(directSemantic) || repeatedActionPattern(def).test(directSemantic)
      || (hasInlineParameters(def) && actionPattern(def.keyword).test(directSemantic))
      || expandSimplePlayerActionDeclarations(directSemantic, def) !== directSemantic
      || expandPersistentPlayerActionPrograms(directSemantic, def) !== directSemantic
      || hasTypeParameterProgram(directSemantic, def) || hasRecipientAction(directSemantic, def, face.name)
      || expandChosenSetupDeclarations(grantedText, def, face.originalPrintedRulesText ?? face.oracleText) !== grantedText
      || expandCounterMenus(directSemantic, [def], face.name) !== directSemantic))).filter(def=>!counterDefinitions.some(entry=>(entry.name===def.keyword||entry.name.startsWith(def.keyword+' ')))||mechanicPattern(def).test(counterFreeSemantic));
    for (const definition of scoped) if (!used.includes(definition)) used.push(definition);
    // A card naming another card's keyword ("a blinked spell") reads its
    // receipts but never gains its identity.
    const referenced = definitions.filter(def => !def.officialCostDeclaration && (designationPattern(def).test(semantic) || paymentCountPattern(def).test(semantic) || mentionsStatus(semantic, def) || hasNamedActivationEvent(grantedText,def,face.name) || hasNamedActionState(grantedText,def,face.name) || hasParticipantActionReference(grantedText,def) || hasCostProgramReference(grantedText,def) || mentionsCustomKeywordPredicate(semantic, def) || hasOperationPaymentAlternative(sourceText,def)));
    if (!used.length && !referenced.length && !counterDefinitions.length && grantedText === sourceText && !hasPlayerCounterRule) {
      const result = { ...face, ...(suppliedDefinitions.length ? {suppliedCardDefinitions: suppliedDefinitions} : {}) };
      delete result.customKeywordIdentities;
      delete result.customKeywordPayments;
      delete result.customKeywordCounterDefinitions;
      return result;
    }
    // Source presentation is carried explicitly, never reconstructed by
    // replacing coincidentally similar compiler output.
    const displayOracleText = authoredSourceText;
    text = separateKeywordDeclarations(expandCounterMenus(grantedText, definitions, face.name), used);
    for (const def of definitions) text = costProgramReferences(expandNamedActionState(expandNamedActivationEvents(text,def,face.name),def,face.name),def);
    for (const originalDefinition of used) {
      if (originalDefinition.appliesToSubtype || originalDefinition.declarationKind==='card-creation') continue;
      const def = namedActionProgramDefinition(originalDefinition);
      const characteristics = expandPrintedCastingCharacteristics(text, def, face.name);
      if (characteristics !== text) { text = characteristics; continue; }
      if (hasTokenSizeParameters(def)) { text = expandTokenSizeParameters(text, def); continue; }
      if (def.declarationKind === 'ability-word') { text = expandAbilityWordPrograms(text, def); continue; }
      if (def.costProgram) {
        text = costProgramReferences(expandCostProgramDeclarations(text, def), def);
        continue;
      }
      const chosenSetup = expandChosenSetupDeclarations(text, def, face.originalPrintedRulesText ?? face.oracleText);
      if (chosenSetup !== text) { text = chosenSetup; continue; }
      const bars = expandAbilityBarProgram(text, def);
      if (bars !== text) { text = bars; continue; }
      const following = expandFollowingAbilities(text, def);
      if (following !== null) { text = following; continue; }
      if (hasPrefixParameter(def)) {
        text = expandPrefixParameters(text, def);
        continue;
      }
      if (hasLiteralTemplate(def)) {
        text = expandLiteralDeclarations(text, def);
        continue;
      }
      text = expandRecipientActions(text, def, face.name);
      text = expandTypeParameterPrograms(text, def, mechanicExpansion);
      let before = text;
      if (!hasReminderParameters(def.reminder, def.template) && !hasActionParameters(def)) {
        text = expandPersistentPlayerActionPrograms(text, def);
        text = expandRepeatedActionDeclarations(text, def);
        text = expandSimplePlayerActionDeclarations(text, def);
        before = text;
      }
      if (hasActionParameters(def)) {
        text = expandActionParameterDeclarations(text, def, face.name);
        continue;
      }
      if (hasReminderParameters(def.reminder, def.template)) {
        const inline = hasInlineParameters(def);
        const bare = inline && [...before.matchAll(actionPattern(def.keyword))].find(match => outsideQuotation(before, match.index));
        if (bare) {
          const keyword = bare[2];
          throw new Error(`"${keyword}" needs its parameters, for example "${parameterExample(def.reminder, keyword, def.template)}".`);
        }
        const parameterPattern = inline ? inlineParameterPattern(def) : costedPattern(def.keyword);
        text = text.replace(parameterPattern, (...args) => {
          const [match, prefix, keyword, rest] = args;
          if (def.officialCostDeclaration && !String(rest).trim()) return match;
          const where = inline ? args[4] : null;
          const then = inline ? args[5] : null;
          const offset = inline ? args[6] : args[4];
          if (!outsideQuotation(before, offset)) return match;
          const repetition = inline && rest.match(/\s+(once|twice|(?:one|two|three|four|five|six|seven|eight|nine|ten|\d+)\s+times)$/i);
          const priceDeclaration = inline ? {price:rest,following:''} : splitReminderPriceDeclaration(rest,def);
          const operand = repetition ? rest.slice(0, repetition.index) : priceDeclaration.price;
          const values = parseParameterDeclaration(def.reminder, keyword, `${keyword} ${operand}`, def.template);
          if (hasControlFlowOperand(values)) return match;
          if (!values) throw new Error(`"${keyword}" needs its parameters, for example "${parameterExample(def.reminder, keyword, def.template)}".`);
          let expanded = bindSelfTransformActivationLabel(mechanicExpansion(bindReminderParameters(def.reminder, keyword, values, customKeywordForms(def))),def);
          expanded = bindFollowingSourceAbilityReferences(expanded, def.keyword, before.slice(offset + match.length));
          if (where) {
            const bound = bindReminderValueFromDeclaration(expanded, `${keyword} ${rest}${where}`);
            if (!bound.binding) throw new Error(`"${keyword}" requires a matching variable in its reminder and declaration.`);
            expanded = bound.text;
          }
          if (repetition) {
            const words = { once: 1, twice: 2, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10 };
            const amount = repetition[1].toLowerCase().replace(/\s+times$/, '');
            const count = words[amount] ?? Number(amount);
            if (!Number.isInteger(count) || count < 1 || count > 32) return match;
            expanded = Array.from({ length: count }, () => expanded.replace(/[.!?]+$/, '')).join('. ');
          }
          if (priceDeclaration.following) expanded = expanded.replace(/[.!?]+$/, '') + '. ' + priceDeclaration.following;
          if (then) return prefix + expanded.replace(/[.!?]+$/, '') + '. Then ';
          const followedByPeriod = before[offset + match.length] === '.';
          return prefix + (followedByPeriod ? expanded.replace(/[.!?]+$/, '') : expanded);
        });
        continue;
      }
      if (/^You may cast this spell with different mana cost, color, and size\. It keeps its abilities and types\.$/i.test(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, parameters, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const expanded = mechanicExpansion(def.reminder, `${keyword} ${parameters}`);
          if (expanded === def.reminder) throw new Error(`"${keyword}" needs a mana cost and power/toughness, such as {2}{U}—2/3.`);
          return prefix + expanded;
        });
        continue;
      }

      if (hasReminderMaterialParameter(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, material, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const bound = bindReminderMaterials(def.reminder, `${keyword} ${material}`);
          if (!bound.binding) throw new Error('The material definition needs its complete declaration.');
          return prefix + bound.text;
        });
        continue;
      }
      if (hasExileCastDefinition(def.reminder)) {
        const pattern = new RegExp(exileObjectPattern(def.keyword).source + '(\\s*,\\s*then\\s+)?', 'gim');
        text = text.replace(pattern, (match, prefix, _keyword, operand, then, offset) => outsideQuotation(before, offset) ? prefix + bindExileCastObject(def.reminder, operand).replace(/[.!?]+$/, '') + (then ? '. Then ' : '') : match);
        continue;
      }
      if (hasNextUntapCostDefinition(def.reminder)) {
        text = text.replace(selfCostPattern(def.keyword), (match, prefix, _keyword, offset) => outsideQuotation(before, offset) ? prefix + nextUntapCostText : match);
        continue;
      }
      if (hasReminderCardParameter(def.reminder)) {
        const pattern = new RegExp(cardParameterPattern(def.keyword).source + '(\\s*,\\s*then\\s+)?', 'gim');
        text = text.replace(pattern, (match, prefix, _keyword, operand, then, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          return prefix + bindReminderCard(def.expansion, operand).replace(/[.!?]+$/, '') + (then ? '. Then ' : '');
        });
        continue;
      }
      text = text.replace(valuePattern(def.keyword), (match, prefix, keyword, parameter, offset) => {
        if (!outsideQuotation(before, offset)) return match;
        const bound = bindReminderValueFromDeclaration(def.expansion, `${keyword} ${parameter}`);
        if (!bound.binding) throw new Error(`"${keyword}" requires a matching variable in its reminder and declaration.`);
        return prefix + bound.text;
      });
      before = text;
      if (isActivationReminder(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, activation, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const bound = bindActivationReminder(def.reminder, `${keyword} — ${activation}`);
          if (!bound.binding) throw new Error(`"${keyword}" requires a complete cost and effect before its activation reminder.`);
          return prefix + mechanicExpansion(bound.text);
        });
        continue;
      }
      if (hasReminderCostReference(def.reminder, def.keyword, customKeywordForms(def)) || hasRepeatedCopyPayment(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, _word, cost, offset) => {
          if (def.officialCostDeclaration && !String(cost).trim()) return match;
          if (!outsideQuotation(before, offset)) return match;
          const declaration = splitReminderPriceDeclaration(cost,def);
          let expanded = hasRepeatedCopyPayment(def.reminder) ? expandRepeatedCopyPayment(def.expansion, def.keyword, declaration.price) : bindReminderCost(def.expansion, def.keyword, declaration.price, customKeywordForms(def));
          if(declaration.following) expanded=expanded.replace(/[.!?]+$/, '')+'. '+declaration.following;
          return prefix + bindFollowingSourceAbilityReferences(expanded, def.keyword, before.slice(offset + match.length));
        });
        continue;
      }
      const pattern = new RegExp(actionPattern(def.keyword).source + '(\\s*,\\s*then\\s+)?', 'gim');
      text = text.replace(pattern, (match, prefix, _word, then, offset) => {
        if (!outsideQuotation(before, offset)) return match;
        if (/^unless\s+you\s+$/i.test(prefix) && !isSinglePlayerAction(def)) return match;
        return `${prefix}${def.expansion.replace(/[.!?]+$/, '')}${then ? '. Then ' : ''}`;
      });
    }
    for (const def of definitions) {
      text = expandStatusGrants(text, def, mechanicExpansion);
      text = expandStatusActions(text, def);
      text = expandStatusReferences(text, def);
      text = expandParticipantActionReferences(text, def);
    }
    for (const def of definitions.filter(def=>used.includes(def)||def.statusPredicate)) text = bindParticipleReferences(text, def, face.name);
    for (const def of referenced) {
      text = bindPaymentCountReferences(text, def);
      text = bindDesignations(text, def);
    }
    for (const def of definitions) text = bindTextAdditionLineage(text,def,face.name);
    text = expandCustomKeywordPredicates(text, definitions);
    for (const def of definitions) {
      text = bindLinkedEntryExileReference(text,def,face.name);
      text = expandSelfTransformActionReferences(text,def,face.name);
    }
    const payments = used.filter(def => definitionPayment(text, def)).map(def => def.keyword);
    const result = { ...face, ...(suppliedDefinitions.length?{suppliedCardDefinitions:suppliedDefinitions}:{}), oracleText: text, displayOracleText,
      originalPrintedRulesText: face.originalPrintedRulesText ?? face.oracleText,
      customKeywordIdentities: used.filter(def => !def.appliesToSubtype && def.declarationKind!=='card-creation').map(def => def.keyword),
      ...(counterDefinitions.length ? {customKeywordCounterDefinitions:counterDefinitions} : {}),
      ...(payments.length === 1 ? { customKeywordPayments: payments } : {}) };
    if (layoutCounterDefinitions.length) {
      const counters = new Map();
      for (const definition of [...(result.customKeywordCounterDefinitions || []), ...layoutCounterDefinitions]) {
        const key = String(definition.name || '').toLowerCase();
        const previous = counters.get(key);
        if (previous && previous.reminder !== definition.reminder) throw new Error(`Conflicting ability-counter program for ${key}.`);
        counters.set(key, definition);
      }
      result.customKeywordCounterDefinitions = [...counters.values()];
    }
    if (!used.some(def => !def.appliesToSubtype)) delete result.customKeywordIdentities;
    if (payments.length !== 1) delete result.customKeywordPayments;
    if (!counterDefinitions.length && !layoutCounterDefinitions.length) delete result.customKeywordCounterDefinitions;
    return result;
  }) };
}

const OPTIONAL_ADDITIONAL_COST = /\b(?:As an additional cost(?: named ["'][^"']+["'])? to cast this spell, you may|You may pay an additional|As you cast this spell, you may pay the following additional cost any number of times)\b[^.]*/gi;
// Both alternative-cost wordings: "You may cast this spell for {R} …" and
// "You may pay {R} rather than pay this spell's mana cost."
const ALTERNATIVE_PRICE = /\bYou may (?:(?:cast this (?:spell|card)\b[^.]*?\bfor)\s+((?:\{[^{}]+\})+)|((?:pay|exile|discard|sacrifice|return|tap|remove)\b[^.!?]*?) rather than pay this (?:spell|card)'s mana cost)/gi;
const ALTERNATIVE_DEFINITION = /\bYou may (?:cast this (?:spell|card)\b[^.]*?\bfor\b|(?:pay|exile|discard|sacrifice|return|tap|remove)\b[^.]*?\brather than pay this (?:spell|card)'s mana cost\b)/i;
// A timed exile's expanded special action, the described form of Suspend
// ("Forge 4—{B}{B}"), and the reminder shape that defines one.
const TIMED_EXILE = /\bIf you could begin to cast this card from your hand, you may pay (?:\{[^{}]+\})+ and exile it with \S+ [a-z][a-z -]* counters on it\. This action doesn't use the stack\b/gi;
const TIMED_EXILE_DEFINITION = /\bRather than cast this card from your hand, you may pay \S+ and exile it with \S+ [a-z][a-z -]* counters on it\b/i;
const PARTICIPLE_SUBJECTS = '(it|this spell|this creature|this permanent|this card)';

// The single generic payment a definition creates on an expanded face: an
// optional additional cost, an alternative price in mana, or a timed exile.
// Ambiguous payment owners have no unique receipt a form can name.

const HAND_REVEAL_WINDOW = /\bYou may reveal this card from your hand as\b/i;
const HAND_REVEAL_PRICE = /\bWhen you reveal this card this way,\s*you may cast it by ([^.!?]+?) rather than its mana cost\b/gi;

export function definitionPayment(text, def) {
  // Explicit typed attribution is verified against the compiler's unique
  // descriptive text-addition price. It never infers a mechanic from prose.
  if (def.paymentKind === 'text-addition') return { kind: 'text-addition' };
  // A preparation payment is distinct from the later free casting price.
  // Attribute its authored identity, not a mana literal which belongs to no
  // ordinary alternative casting cost. Quoted recipient grants are separate.
  if (/^You may pay \{Cost\} and exile this card from your hand as a sorcery\. When you next resolve a spell, (?:you may )?cast this card from exile without paying its mana cost\.$/i.test(def.expansion)) {
    const matches=[...String(text).matchAll(/\bYou may (?:pay |tap |sacrifice )[^.\n]+ and exile this card from your hand as a sorcery\. When you next resolve a spell, (?:you may )?cast this card from exile without paying its mana cost\./gi)]
      .filter(match=>outsideQuotation(text,match.index));
    if(matches.length===1)return {kind:'preparation',cost:def.keyword};
  }

  if (hasPrintedCastingCharacteristics(def.reminder)) {
    const prices = [...text.matchAll(/\bCast it on a later turn for ((?:\{[^{}]+\})+)\./gi)];
    return prices.length === 1 ? { kind: 'alternative', cost: prices[0][1] } : null;
  }
  const optional = text.match(OPTIONAL_ADDITIONAL_COST) || [];
  const alternatives = [...text.matchAll(ALTERNATIVE_PRICE)];
  if (HAND_REVEAL_WINDOW.test(def.expansion)) {
    const prices=[...text.matchAll(HAND_REVEAL_PRICE)];
    if (prices.length!==1 || alternatives.length || optional.length) return null;
    return {kind:'alternative',cost:prices[0][1].replace(/^paying\s+/i,'').replace(/^exiling\b/i,'exile').replace(/^discarding\b/i,'discard').replace(/^sacrificing\b/i,'sacrifice')};
  }
  if (TIMED_EXILE_DEFINITION.test(def.reminder)) {
    if ((text.match(TIMED_EXILE) || []).length !== 1 || alternatives.length || optional.length) return null;
    return { kind: 'timed-exile' };
  }
  if (String(def.expansion || '').match(OPTIONAL_ADDITIONAL_COST)) {
    if (optional.length !== 1 || alternatives.length) return null;
    return { kind: 'additional' };
  }
  if (ALTERNATIVE_DEFINITION.test(def.expansion)) {
    if (alternatives.length !== 1 || optional.length) return null;
    return { kind: 'alternative', cost: alternatives[0][1] || alternatives[0][2].replace(/^pay\s+/i, '') };
  }
  return null;
}

// A participle reference ("if it was blinked") tests the payment its definition
// created, the generic reading of "kicked" or "cast for its warp cost". It binds
// only to a unique payment; any other reference is left as written for strict
// compilation to judge.
// Count a named payment on the exact triggering spell rather than the reader.
function paymentCountPattern(def) {
  const forms = customKeywordForms(def);
  const words = [...new Set([forms.participle, ...forms.variants])].map(escape).join('|');
  return new RegExp(`\\b(for each time|the number of times)\\s+that spell\\s+was\\s+(?:${words})\\b`, 'gi');
}
function bindPaymentCountReferences(text, def) {
  if (!String(def.expansion || '').match(OPTIONAL_ADDITIONAL_COST)) return text;
  const before = text;
  return text.replace(paymentCountPattern(def), (match, count, offset) =>
    outsideQuotation(before, offset) ? `${count} that spell's additional ${def.keyword} cost was paid` : match);
}

function bindParticipleReferences(text, def, sourceName = '') {
  const forms = customKeywordForms(def);
  const words = [forms.participle, ...forms.variants].map(escape).join('|');
  const verbs = def.statusPredicate ? "was|wasn't|was not|is|isn't|is not" : "was|wasn't|was not";
  // An opaque pronoun may refer to an independently selected object.
  // A source-relative status predicate may only normalize explicit self references.
  const subjects = def.statusPredicate ? '(this spell|this creature|this permanent|this card)' : PARTICIPLE_SUBJECTS;
  const pronouns = () => new RegExp(`\\b${subjects}\\s+(${verbs})\\s+(?:${words})\\b`, 'gi');
  // A full source name begins a clause/condition operand. A suffix of a
  // longer name (Other Experiment) never denotes the source Experiment.
  const named = () => sourceName ? new RegExp(`(^|(?:[.!?;,:\\n]|\\b(?:if|unless|when|whenever|while|and|or|then))\\s*)(${escape(sourceName)})\\s+(${verbs})\\s+(?:${words})\\b`, 'gi') : null;
  if (!pronouns().test(text) && !named()?.test(text)) return text;
  const payment = definitionPayment(text, def);
  if (!def.statusPredicate && (!payment || payment.kind === 'timed-exile')) return text;
  const render = (subject,verb) => {
    if (def.statusPredicate) {
      const not=/^(?:was|is)$/i.test(verb)?'':'not ';
      return `this spell is ${not}a copy with custom keyword "${def.statusPredicate.customKeyword}"`;
    }
    const owner = /^this spell$/i.test(subject) ? "this spell's" : 'its';
    const head = subject[0] === subject[0].toUpperCase() ? owner[0].toUpperCase() + owner.slice(1) : owner;
    const price = payment.kind === 'additional' ? 'additional' : payment.cost;
    return `${head} ${price} cost ${/^(?:was|is)$/i.test(verb) ? 'was' : "wasn't"} paid`;
  };
  const before = text;
  let result = text.replace(pronouns(), (match,subject,verb,offset) => outsideQuotation(before,offset) ? render(subject,verb) : match);
  if (sourceName) {
    const namedBefore = result;
    result = result.replace(named(), (match,prefix,subject,verb,offset) => outsideQuotation(namedBefore,offset) ? prefix + render(subject,verb) : match);
  }
  return result;
}

// "a blinked spell": another spell paid with that keyword's payment. Rewritten
// to "a spell using blink", which the compiler reads as that receipt.
function designationPattern(def) {
  const forms = customKeywordForms(def);
  const words = [...new Set([forms.participle, forms.designation, ...forms.variants].filter(Boolean))].map(escape).join('|');
  return new RegExp(`\\b(${words})\\s+((?:[a-z]+\\s+){0,3}?spells?)\\b`, 'gi');
}

// The relationship names the original card supplying the text, not a broad
// keyword-payment class. The compiler freezes this operand when text is added.
function bindTextAdditionLineage(text,def,sourceName){
  if(def.paymentKind!=='text-addition'||!def.textAdditionVerb)return text;
  const source=[sourceName,'this spell','this card','this permanent','this'].filter(Boolean).sort((a,b)=>b.length-a.length).map(escape).join('|');
  const pattern=new RegExp(`\\b(sources?)\\s+${escape(def.textAdditionVerb)}\\s+(${source})(?![a-z])`,'gi');
  const before=text;
  return text.replace(pattern,(match,noun,origin,offset)=>outsideQuotation(before,offset)?`${noun} with text added from ${origin}`:match);
}

function bindDesignations(text, def) {
  const before = text;
  return text.replace(designationPattern(def), (match, _form, noun, offset) =>
    outsideQuotation(before, offset) ? `${noun} using ${def.keyword}` : match);
}

// If another line fails first, still retain an unambiguous bare custom
// action declaration from this attempted card. Never infer arbitrary sentences.
function otherDeclarations(draft, definitions) {
  const candidates = [];
  for (const face of draft.faces) {
    const semantic = String(face.oracleText || '').replace(/\([^()]*\)/g, '');
    for (const match of semantic.matchAll(/(?:^|[,.:;]\s*|\bthen\s+)([a-z][a-z '-]*?)\s+(the top card of your library|one of them|one of those cards)(?=\s*(?:[,.;]|$))/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasReminderCardParameter(declaration.reminder)) candidates.push(declaration);
      }
    }
    for (const match of semantic.matchAll(/^[ \t]*([a-z][a-z '-]*?)[ \t]+[XYZ],\s*where\s+[XYZ]\s+is\s+[^\n]+$/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (bindReminderValueFromDeclaration(declaration.reminder, match[0]).binding) candidates.push(declaration);
      }
    }
    for (const match of String(face.oracleText || '').matchAll(/\(To ([a-z][a-z '-]*?) (?:a|an) [a-z]+, exile it\./gi)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasExileCastDefinition(declaration.reminder)) candidates.push(declaration);
      }
    }
    for (const line of semantic.split('\n')) {
      const match = line.match(/(?:^|,\s*)([a-z][a-z '-]*?) this creature\s*:/i);
      if (!match) continue;
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasNextUntapCostDefinition(declaration.reminder)) candidates.push(declaration);
      }
    }
    for (const match of semantic.matchAll(/^([a-z][a-z '-]*?) with [^\n]+$/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasReminderMaterialParameter(declaration.reminder)) candidates.push(declaration);
      }
    }
    // A parameterized declaration: "Keyword 3—{1}{U}", "Keyword 3 {1}{U}",
    // "Keyword a Goblin" or "Keyword 3". The keyword is one of the line's
    // leading words, proven by a reminder that reads the rest as its values.
    for (const line of semantic.split('\n')) {
      const words = line.trim().split(/\s+/);
      // The shortest such keyword: "Rally an Elf" names "rally", not "rally an".
      for (let count = 1, found = false; !found && count < Math.min(words.length, 5); count++) {
        const keyword = mechanicKey(words.slice(0, count).join(' '));
        if (!/^[a-z][a-z '-]*$/.test(keyword) || official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
        for (const declaration of definingReminders(face, keyword)) {
          if (hasReminderParameters(declaration.reminder) && parseParameterDeclaration(declaration.reminder, keyword, line.trim())) {
            candidates.push(declaration);
            found = true;
          }
        }
      }
    }
    // A costed declaration has an explicit dash or starts its price with mana.
    // Do not infer arbitrary prose as a mechanic name.
    for (const match of semantic.matchAll(/^[ \t]*([a-z][a-z '-]*?)(?:[ \t]*[—–][ \t]*|[ \t]+(?=\{))[^\n]+$/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasReminderCostReference(declaration.reminder, keyword) || bindReminderCostFromDeclaration(declaration.reminder, match[0]).binding) candidates.push(declaration);
      }
    }
    for (const match of semantic.matchAll(/(?:^|[,.:;]\s*|\bthen\s+)([a-z]+)(?=\s*(?:[,.;]|$))/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      candidates.push(...definingReminders(face, keyword));
    }
  }
  return candidates;
}

export async function prepareCustomCard(draft, compile, { known = [], reserve = async () => {}, knownTokens = [], knownSubtypes } = {}) {
  if (!knownSubtypes) { try { knownSubtypes = listCustomSubtypes(); } catch { knownSubtypes = []; } }
  let definitions = draftMechanics(draft, known);
  // Custom tokens the card names travel with it, like its mechanics.
  let tokens = [];
  const { customTokens: _carried, ...rest } = draft;
  const prepared = { ...rest };
  const compilerInput = defs => {
    const expanded = compileDraftInput(prepared, defs);
    tokens = draftTokens({...draft, faces:expanded.faces}, knownTokens);
    return withDraftSubtypes(applyCustomTokens({...expanded, ...(tokens.length ? {customTokens:tokens} : {})}, tokens), knownSubtypes);
  };
  await reserve(definitions);
  for (let attempts = 0; attempts < 16; attempts++) {
    try {
      const preview = await compile(compilerInput(definitions));
      if (!preview?.canCreate) throw new Error('This card must compile completely before it can be saved.');
      return { draft: { ...prepared, ...(tokens.length ? {customTokens:tokens} : {}), customMechanics: definitions, customSubtypes: compilerInput(definitions).customSubtypes }, input: compilerInput(definitions), preview };
    } catch (error) {
      const keyword = mechanicKey(String(error?.message || error).match(/could not find verb in effect clause \(clause: '([^']+)'/)?.[1]);
      if (!keyword || definitions.some(def => def.keyword === keyword) || official.has(keyword) || !/^[a-z][a-z '-]{0,63}$/.test(keyword)) {
        const declarations = otherDeclarations(draft, definitions);
        if (declarations.length) {
          const byReminder = new Map();
          for (const declaration of declarations) {
            const key = `${declaration.sourceCard}:${declaration.sourceLine}`, prior = byReminder.get(key);
            if (prior && prior !== declaration.keyword) throw new Error('Multiple custom actions share one reminder. Give each action its own definition on a separate line.');
            byReminder.set(key, declaration.keyword);
          }
          definitions = mergeMechanics(definitions, declarations);
          await reserve(definitions);
          // Line-family and target diagnostics can hide the unknown action.
          // Newly discovered definitions deserve the same compile retry as a
          // missing-verb diagnostic; remaining errors still fail strictly.
          continue;
        }
        throw error;
      }
      const candidates = draft.faces.flatMap(face => definingReminders(face, keyword));
      if (!candidates.length) throw new Error(`${String(error?.message || error)} Add a parenthetical reminder defining "${keyword}" on its action line or immediately below it.`);
      if (draft.faces.some(face => {
        const newLines = new Set(definingReminders(face, keyword).map(candidate => candidate.sourceLine));
        return definitions.some(def => definingReminders(face, def.keyword).some(candidate => newLines.has(candidate.sourceLine)));
      })) {
        throw new Error(`Cannot tell which action the reminder defines. Give "${keyword}" its own reminder on a separate line.`);
      }
      definitions = mergeMechanics(definitions, candidates);
      // Persist immediately, even if the next compilation fails elsewhere.
      await reserve(definitions);
    }
  }
  throw new Error('Too many unresolved custom mechanics. Simplify the card and define each action separately.');
}

const active = new Map();
let revision = 0;
export const customMechanicRevision = () => revision;
export function installCustomMechanics(definitions) {
  let changed = false;
  for (const value of definitions) {
    const def = validateMechanic(value);
    if (JSON.stringify(active.get(def.keyword)) !== JSON.stringify(def)) {
      active.set(def.keyword, def);
      changed = true;
    }
  }
  if (changed) revision++;
}
export function uninstallCustomMechanic(keyword) {
  if (active.delete(mechanicKey(keyword))) revision++;
}
export const activeCustomMechanics = () => [...active.values()];

// A mechanic defined on its own (the Mechanics tab) has no card to prove it
// compiles. Compile it on a minimal card that uses it the way a card would,
// through the same expansion as a real draft. Shapes that need a
// card-specific declaration (materials, activations, variables) can only be
// judged on a real card.
const PROBE_COST = '{1}';
const PERMANENT_REMINDER = /\b(?:this (?:creature|permanent)|enters|attacks|blocks|dies|leaves the battlefield)\b/i;

function probeUsage(def) {
  const { keyword, reminder } = def;
  const name = keyword[0].toUpperCase() + keyword.slice(1);
  if (hasPrintedCastingCharacteristics(reminder) || /^You may cast this spell with different mana cost/i.test(reminder) || hasReminderMaterialParameter(reminder)
    || hasExileCastDefinition(reminder) || hasNextUntapCostDefinition(reminder) || isActivationReminder(reminder)
    || /\b[XYZ]\b/.test(reminder)) return null;
  if (hasTokenSizeParameters(def)) return `${name} — 1/1, 2/2`;
  if (hasReminderParameters(reminder, def.template)) return parameterExample(reminder, name, def.template).replace('{1}{U}', PROBE_COST);
  if (hasReminderCardParameter(reminder)) return `${name} the top card of your library.`;
  if (hasReminderCostReference(reminder, keyword, customKeywordForms(def)) || hasRepeatedCopyPayment(reminder)) return `${name} ${PROBE_COST}`;
  return `${name}.`;
}

export async function probeMechanic(value, compile) {
  const def = validateMechanic(value);
  const usage = probeUsage(def);
  if (!usage) return { checked: false };
  const shapes = [
    { cardTypes: ['Instant'] },
    { cardTypes: ['Creature'], power: '2', toughness: '2' },
  ];
  if (PERMANENT_REMINDER.test(def.reminder)) shapes.reverse();
  const attempt = async shape => {
    const draft = { faces: [{ name: 'Mechanic Test', manaCost: '{0}', ...shape, oracleText: usage }] };
    const preview = await compile(compileDraftInput(draft, [def]));
    if (!preview?.canCreate) throw new Error('The reminder text did not compile completely.');
    return preview.faces?.[0]?.rawCompilation || '';
  };
  let firstError = null;
  for (const shape of shapes) {
    try {
      await attempt(shape);
      // A sentence with no ability shape ("You may pay {Cost} to …") compiles
      // as a spell instruction, which a permanent never runs.
      const onPermanent = await attempt(shapes.find(candidate => candidate.power) || shape).catch(() => '');
      return { checked: true, usage, permanentAbility: grantsPermanentAbility(onPermanent) };
    } catch (error) {
      firstError ||= error;
    }
  }
  const failure = new Error(String(firstError?.message || firstError));
  failure.usage = usage;
  throw failure;
}

// Abilities beyond the keyword's own identity marker.
const grantsPermanentAbility = raw => (raw.match(/\bAbility \{/g) || []).length > (raw.match(/\bKeywordMarker\b/g) || []).length;


