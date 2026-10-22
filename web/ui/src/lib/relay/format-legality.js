import { indexFormatCatalog, installFormatAliases } from './format-catalog-index.js';
import { PUBLIC_FORMATS, usesCommanders } from './formats.js';
let catalog;
let loading;
export async function loadFormatCatalog() {
  const load = async () => {
    if (typeof Worker === 'function') return new Promise((resolve, reject) => {
      const worker = new Worker(new URL('../../workers/formatCatalogWorker.js', import.meta.url), { type: 'module' });
      worker.onmessage = ({ data: result }) => {
        worker.terminate();
        if (result.error) { reject(new Error(result.error)); return; }
        installFormatAliases(result.data.cards, result.aliases);
        resolve(result.data);
      };
      worker.onerror = event => { worker.terminate(); reject(new Error(event.message || 'Could not load card legality catalog')); };
      worker.postMessage({ type: 'load' });
    });
    const response = await fetch(new URL('./format-catalog.generated.json', import.meta.url));
    if (!response.ok) throw new Error('Card legality catalog unavailable. Try again before joining.');
    const data = await response.json();
    if (!data.cards || !data.sourceUpdatedAt) throw new Error('Invalid card legality catalog');
    indexFormatCatalog(data.cards);
    return data;
  };
  if (!loading) loading = load().then(data => {
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
  // Deck imports commonly use the front face of double-faced cards, and
  // tournament lists write split cards with a single slash ("Wear / Tear").
  return indexFormatCatalog(cards).get(key(name)) || cards[key(name).replace(/\s+\/\s+/, ' // ')];
}
function copyLimit(card, format) {
  if (/\bBasic\b/.test(card.type) || /deck can have any number of cards named/i.test(card.rules)) return Infinity;
  const match = card.rules.match(/deck can have up to (\w+) cards named/i);
  const number = match && ({ two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9 }[match[1]] || Number(match[1]));
  return number || (usesCommanders(format) ? 1 : 4);
}
function commanderEligible(card, format) {
  // Pauper EDH leads with a creature (or Background) printed at uncommon.
  if (format === 'paupercommander') return card.pdhCommander === true;
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
// `options.allowUnreleased` admits cards Scryfall previews before any format
// lists them (the lobby's "Allow unreleased cards" toggle); they still obey
// copy limits, commander rules, and color identity.
export function validateFormatDeck(format, deck = [], commanders = [], sideboard = [], data = catalog, options = {}) {
  const rules = Object.hasOwn(PUBLIC_FORMATS, format) ? PUBLIC_FORMATS[format] : null;
  if (!rules) return { ready: false, errors: ['Choose a supported public format.'] };
  const errors = [];
  // Card name as the deck wrote it → why that card keeps the deck from starting.
  const cardProblems = {};
  const flag = (name, message) => { errors.push(message); cardProblems[name] ||= message; };
  if (!data?.cards) return { ready: false, errors: ['Card legality catalog has not loaded.'], cardProblems };
  const commanderFormat = usesCommanders(format);
  if (commanderFormat) {
    if (![1, 2].includes(commanders.length) || deck.length + commanders.length !== 100) errors.push(`${rules.label} requires 100 cards including one commander or a legal commander pair.`);
    if (sideboard.length) errors.push(`${rules.label} does not use a sideboard.`);
  } else {
    if (deck.length < 60) errors.push(`${rules.label} requires at least 60 main-deck cards.`);
    if (sideboard.length > 15) errors.push('Sideboards may contain at most 15 cards.');
    if (commanders.length) errors.push(`${rules.label} does not use commanders.`);
  }
  const counts = new Map();
  const found = new Map();
  const requested = new Map();
  const written = new Map();
  for (const name of [...deck, ...sideboard, ...commanders]) requested.set(name, (requested.get(name) || 0) + 1);
  for (const [name, count] of requested) {
    const card = lookup(data.cards, name);
    if (!card) { flag(name, `Unknown card: ${name}.`); continue; }
    counts.set(card.name, (counts.get(card.name) || 0) + count);
    found.set(card.name, card);
    written.set(card.name, name);
  }
  const flagCard = (card, message) => flag(written.get(card.name) || card.name, message);
  // Pauper EDH commanders are uncommons, which its card legality (the 99's
  // commons-only rule) never covers; their eligibility is checked below.
  const leaders = commanders.map(name => lookup(data.cards, name));
  const exempt = new Set(format === 'paupercommander' ? leaders.filter(Boolean).map(card => card.name) : []);
  for (const [name, count] of counts) {
    const card = found.get(name);
    const legal = card.legalities[format];
    if (card.unreleased) {
      if (!options.allowUnreleased) flagCard(card, `${name} is unreleased; this lobby does not allow unreleased cards.`);
    } else {
      const allowed = exempt.has(name) ? legal !== 'banned' : ['legal', 'restricted'].includes(legal);
      if (!allowed) flagCard(card, `${name} is ${legal === 'banned' ? 'banned' : 'not legal'} in ${rules.label}.`);
    }
    const limit = legal === 'restricted' ? 1 : copyLimit(card, format);
    if (count > limit) flagCard(card, `${name}: at most ${limit} ${limit === 1 ? 'copy' : 'copies'} across the deck, sideboard, and commanders.`);
  }
  if (commanderFormat) {
    if (leaders.length === 1 && leaders[0] && !commanderEligible(leaders[0], format)) flagCard(leaders[0], `${leaders[0].name} cannot be a commander in ${rules.label}.`);
    if (leaders.length === 2 && leaders.every(Boolean)) {
      // A partner pair is legendary by construction; Pauper EDH also wants both uncommon.
      if (format === 'paupercommander') for (const leader of leaders) if (!commanderEligible(leader, format)) flagCard(leader, `${leader.name} cannot be a commander in ${rules.label}.`);
      if (!validPair(...leaders)) errors.push('These commanders do not form a legal pair.');
    }
    const colors = new Set(leaders.filter(Boolean).flatMap(c => c.colors));
    for (const card of found.values()) {
      if (card.colors.some(color => !colors.has(color))) flagCard(card, `${card.name} is outside the commanders’ color identity.`);
      const basicColors = { Plains: 'W', Island: 'U', Swamp: 'B', Mountain: 'R', Forest: 'G' };
      for (const [land, color] of Object.entries(basicColors)) {
        if (new RegExp(`\\b${land}\\b`).test(card.type) && !colors.has(color)) flagCard(card, `${card.name} has a basic land type outside the commanders’ colors.`);
      }
    }
  }
  return { ready: errors.length === 0, errors: [...new Set(errors)], cardProblems };
}
// Checks every deck of a saved configuration against each chosen format;
// `decks` holds { index, deck, commanders, sideboard } card-name lists.
export function deckFormatProblems(formats, decks, data = catalog, options = {}) {
  return formats.flatMap(format => decks.flatMap(({ index, deck, commanders, sideboard }) => {
    const result = validateFormatDeck(format, deck, commanders, sideboard, data, options);
    return result.ready ? [] : [{ format, index, errors: result.errors }];
  }));
}
const matchPlayerCount = config => config.playerNames?.length || config.players?.length || config.decks?.length;
// Public relay tables also fix the life total and seat count.
export function assertFormatMatch(config, data = catalog) {
  const rules = Object.hasOwn(PUBLIC_FORMATS, config.format) ? PUBLIC_FORMATS[config.format] : null;
  if (!rules) throw new Error('Unsupported public format');
  const count = matchPlayerCount(config);
  if (count < 2 || count > rules.maxPlayers || config.startingLife !== rules.startingLife) throw new Error(`${rules.label} requires ${rules.startingLife} life and ${rules.maxPlayers === 2 ? 'two' : 'two to four'} players.`);
  assertFormatDecks(config, data);
}
// Every seat's deck obeys the match format's card legality.
export function assertFormatDecks(config, data = catalog) {
  const options = { allowUnreleased: config.allowUnreleased === true };
  for (let i = 0; i < matchPlayerCount(config); i++) {
    const result = validateFormatDeck(config.format, config.decks?.[i], config.commanders?.[i], config.sideboards?.[i], data, options);
    if (!result.ready) throw new Error(`Player ${i + 1}: ${result.errors.slice(0, 4).join(' ')}`);
  }
}
