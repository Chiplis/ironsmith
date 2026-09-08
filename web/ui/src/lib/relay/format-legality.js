import { PUBLIC_FORMATS } from './formats.js';
let catalog;
let loading;
export async function loadFormatCatalog() {
  if (!loading) loading = fetch(new URL('./format-catalog.generated.json', import.meta.url)).then(async response => {
    if (!response.ok) throw new Error('Card legality catalog unavailable. Try again before joining.');
    const data = await response.json();
    if (!data.cards || !data.sourceUpdatedAt) throw new Error('Invalid card legality catalog');
    catalog = data;
    return data;
  }).catch(error => { loading = null; throw error; });
  return loading;
}
export const formatCatalogDate = () => catalog?.sourceUpdatedAt;
const key = name => String(name || '').trim().toLowerCase();
function lookup(cards, name) {
  const exact = cards[key(name)];
  if (exact) return exact;
  // Deck imports commonly use the front face of double-faced cards.
  return Object.values(cards).find(card => key(card.name.split(' // ')[0]) === key(name));
}
function copyLimit(card, format) {
  if (/\bBasic\b/.test(card.type) || /deck can have any number of cards named/i.test(card.rules)) return Infinity;
  const match = card.rules.match(/deck can have up to (\w+) cards named/i);
  const number = match && ({ two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9 }[match[1]] || Number(match[1]));
  return number || (format === 'commander' ? 1 : 4);
}
function commanderEligible(card) {
  return /Legendary.*Creature/.test(card.type) || /can be your commander/i.test(card.rules);
}
function validPair(a, b) {
  const both = pattern => pattern.test(a.rules) && pattern.test(b.rules);
  if (both(/^Partner(?:\s*\(|\s*$)/m) || both(/Friends forever/)) return true;
  if (a.rules.includes(`Partner with ${b.name}`) && b.rules.includes(`Partner with ${a.name}`)) return true;
  const background = (a, b) => /Choose a Background/.test(a.rules) && /Legendary Enchantment.*Background/.test(b.type);
  const doctor = (a, b) => /Doctor's companion/.test(a.rules) && /Legendary Creature — Time Lord Doctor$/.test(b.type);
  return background(a, b) || background(b, a) || doctor(a, b) || doctor(b, a);
}
export function validateFormatDeck(format, deck = [], commanders = [], sideboard = [], data = catalog) {
  const rules = Object.hasOwn(PUBLIC_FORMATS, format) ? PUBLIC_FORMATS[format] : null;
  if (!rules) return { ready: false, errors: ['Choose a supported public format.'] };
  const errors = [];
  if (!data?.cards) return { ready: false, errors: ['Card legality catalog has not loaded.'] };
  if (format === 'commander') {
    if (![1, 2].includes(commanders.length) || deck.length + commanders.length !== 100) errors.push('Commander requires 100 cards including one commander or a legal commander pair.');
    if (sideboard.length) errors.push('Commander does not use a sideboard.');
  } else {
    if (deck.length < 60) errors.push(`${rules.label} requires at least 60 main-deck cards.`);
    if (sideboard.length > 15) errors.push('Sideboards may contain at most 15 cards.');
    if (commanders.length) errors.push(`${rules.label} does not use commanders.`);
  }
  const counts = new Map();
  const found = new Map();
  for (const name of [...deck, ...sideboard, ...commanders]) {
    const card = lookup(data.cards, name);
    if (!card) { errors.push(`Unknown card: ${name}.`); continue; }
    counts.set(card.name, (counts.get(card.name) || 0) + 1);
    found.set(card.name, card);
  }
  for (const [name, count] of counts) {
    const card = found.get(name);
    const legal = card.legalities[format];
    if (!['legal', 'restricted'].includes(legal)) errors.push(`${name} is ${legal === 'banned' ? 'banned' : 'not legal'} in ${rules.label}.`);
    const limit = legal === 'restricted' ? 1 : copyLimit(card, format);
    if (count > limit) errors.push(`${name}: at most ${limit} ${limit === 1 ? 'copy' : 'copies'} across the deck, sideboard, and commanders.`);
  }
  if (format === 'commander') {
    const leaders = commanders.map(name => lookup(data.cards, name));
    if (leaders.length === 1 && leaders[0] && !commanderEligible(leaders[0])) errors.push(`${leaders[0].name} cannot be a commander.`);
    if (leaders.length === 2 && leaders.every(Boolean) && !validPair(...leaders)) errors.push('These commanders do not form a legal pair.');
    const colors = new Set(leaders.filter(Boolean).flatMap(c => c.colors));
    for (const card of found.values()) {
      if (card.colors.some(color => !colors.has(color))) errors.push(`${card.name} is outside the commanders’ color identity.`);
      const basicColors = { Plains: 'W', Island: 'U', Swamp: 'B', Mountain: 'R', Forest: 'G' };
      for (const [land, color] of Object.entries(basicColors)) {
        if (new RegExp(`\\b${land}\\b`).test(card.type) && !colors.has(color)) errors.push(`${card.name} has a basic land type outside the commanders’ colors.`);
      }
    }
  }
  return { ready: errors.length === 0, errors: [...new Set(errors)] };
}
export function assertFormatMatch(config, data = catalog) {
  const rules = Object.hasOwn(PUBLIC_FORMATS, config.format) ? PUBLIC_FORMATS[config.format] : null;
  if (!rules) throw new Error('Unsupported public format');
  const count = config.playerNames?.length || config.players?.length || config.decks?.length;
  if (count < 2 || count > rules.maxPlayers || config.startingLife !== rules.startingLife) throw new Error(`${rules.label} requires ${rules.startingLife} life and ${rules.maxPlayers === 2 ? 'two' : 'two to four'} players.`);
  for (let i = 0; i < count; i++) {
    const result = validateFormatDeck(config.format, config.decks?.[i], config.commanders?.[i], config.sideboards?.[i], data);
    if (!result.ready) throw new Error(`Player ${i + 1}: ${result.errors.slice(0, 4).join(' ')}`);
  }
}
