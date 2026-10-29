import { normalizeCommunityFormat } from './community-format.js';
import { mergeMechanics } from './custom-mechanics.js';

export const COMMUNITY_FILE_MAX_BYTES = 4_000_000;
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
function requireValue(condition, message) {
  if (!condition) throw new Error(`Invalid Community file: ${message}`);
}
function title(value) {
  requireValue(typeof value === 'string' && value.trim().length > 0 && value.trim().length <= 80, 'a name of 1–80 characters is required.');
}
function array(value, label) {
  requireValue(Array.isArray(value), `${label} must be a list.`);
}
function strings(value, label) {
  array(value, label);
  requireValue(value.every(name => typeof name === 'string' && name.trim()), `${label} must contain card names.`);
}
function cards(value) {
  array(value, 'custom cards');
  for (const card of value) {
    requireValue(object(card) && object(card.draft), 'each custom card needs a draft.');
    array(card.draft.faces, 'card faces');
    requireValue(card.draft.faces.length > 0 && card.draft.faces.length <= 3, 'each custom card needs 1–3 faces.');
    for (const face of card.draft.faces) {
      requireValue(object(face) && typeof face.name === 'string' && face.name.trim(), 'every card face needs a name.');
      for (const field of ['oracleText', 'manaCost', 'displayOracleText']) {
        requireValue(face[field] == null || typeof face[field] === 'string', `${field} must be text.`);
      }
      for (const field of ['cardTypes', 'supertypes', 'subtypes', 'colorIndicator']) {
        if (face[field] != null) strings(face[field], field);
      }
    }
    if (card.draft.customMechanics != null) {
      array(card.draft.customMechanics, 'custom mechanics');
      requireValue(card.draft.customMechanics.every(object), 'mechanic definitions must be objects.');
      // Validate without replacing the originals: retain parameters, forms,
      // expansion and any additional authoring metadata exactly as exported.
      mergeMechanics([], card.draft.customMechanics);
    }
  }
}
function customSet(value) {
  requireValue(object(value), 'a custom set must be an object.');
  title(value.name);
  cards(value.cards);
}
export function validateCommunityShare(kind, value) {
  requireValue(object(value), 'missing content.');
  if (kind === 'set') {
    customSet(value);
    normalizeCommunityFormat({ name: value.name, customSets: [value] });
    return { name: value.name.trim(), cards: structuredClone(value.cards) };
  }
  requireValue(kind === 'format', 'unknown content type.');
  title(value.name);
  requireValue(value.version == null || value.version === 1, 'unsupported format version.');
  for (const field of ['includedSets', 'excludedSets']) {
    array(value[field] ?? [], field);
    for (const set of value[field] || []) {
      requireValue(object(set) && typeof set.code === 'string' && /^[a-z0-9]+$/i.test(set.code), 'official sets need a set code.');
      requireValue(typeof set.name === 'string' && set.name.trim(), 'official sets need a name.');
      strings(set.cards, 'official set cards');
    }
  }
  for (const field of ['includedCards', 'excludedCards']) strings(value[field] ?? [], field);
  array(value.customSets ?? [], 'custom sets');
  for (const set of value.customSets || []) customSet(set);
  cards(value.customCards ?? []);
  return normalizeCommunityFormat(value);
}
export function encodeCommunityShare(kind, value) {
  const data = validateCommunityShare(kind, value);
  const text = JSON.stringify({ schema: 'ironsmith-community', version: 1, kind, data });
  if (new TextEncoder().encode(text).length > COMMUNITY_FILE_MAX_BYTES) throw new Error('Community files must be no larger than 4 MB.');
  return text;
}
export function decodeCommunityShare(text) {
  if (new TextEncoder().encode(text).length > COMMUNITY_FILE_MAX_BYTES) throw new Error('Community files must be no larger than 4 MB.');
  let file;
  try { file = JSON.parse(text); } catch { throw new Error('This file is not valid JSON.'); }
  requireValue(object(file) && file.schema === 'ironsmith-community', 'choose an exported Community set or format.');
  requireValue(file.version === 1, 'unsupported file version.');
  return { kind: file.kind, data: validateCommunityShare(file.kind, file.data) };
}
export function communityFilename(kind, name) {
  const slug = name.normalize('NFKD').replace(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '').toLowerCase().slice(0, 80) || 'community';
  return `${slug}.${kind}.json`;
}
