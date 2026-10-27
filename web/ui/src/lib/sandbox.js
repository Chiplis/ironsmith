import { prepareCustomCard } from "./custom-mechanics.js";
import { normalizeDraftForApi } from './custom-card-draft.js';

export const SANDBOX_RULES = [
  { id: 'freeCasting', label: 'Cast without paying mana costs', text: 'You may cast spells without paying their mana costs.' },
  { id: 'unlimitedLands', label: 'Unlimited land plays', text: 'You may play any number of lands on each of your turns.' },
  { id: 'graveyard', label: 'Cast from your graveyard', text: 'You may cast spells from your graveyard.' },
  { id: 'exile', label: 'Cast from exile', text: 'You may cast spells from exile.' },
  { id: 'mana', label: 'Repeatable mana ability', text: '{0}: Add {W}{U}{B}{R}{G}{C}.' },
  { id: 'life', label: 'Set any player’s life total', text: '{0}: Choose a player, then choose a number. That player\'s life total becomes the chosen number.' },
  { id: 'draw', label: 'Draw cards on demand', text: '{0}: Draw a card.' },
  { id: 'handSize', label: 'No maximum hand size', text: 'You have no maximum hand size.' },
];

export function sandboxerDraft(options = {}) {
  return { layout: 'single', hasFuse: false, faces: [{
    name: 'IronSmith Sandboxer', manaCost: '{0}', colorIndicator: [], supertypes: [],
    cardTypes: ['Enchantment'], subtypes: [],
    oracleText: SANDBOX_RULES.filter(rule => options[rule.id]).map(rule => rule.text).join('\n'),
    power: null, toughness: null, loyalty: null, defense: null, artUrl: '',
  }] };
}

const BASICS = { W: 'Plains', U: 'Island', B: 'Swamp', R: 'Mountain', G: 'Forest' };
const CREATURES = { W: 'Silvercoat Lion', U: 'Coral Merfolk', B: 'Walking Corpse', R: 'Goblin Piker', G: 'Grizzly Bears' };
export function sandboxColors(draft) {
  const faces = draft?.faces || [];
  return Object.keys(BASICS).filter(color => faces.some(face =>
    face.colorIndicator?.includes(color) || new RegExp(`\\{[^}]*${color}[^}]*\\}`).test(`${face.manaCost || ''} ${face.oracleText || ''}`)));
}
function shuffled(cards, random) {
  const result = [...cards];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

// Resolve the complete setup before replacing the current table. The shuffled
// plan is sent to the worker, so journals replay the exact same opening hands.
export function buildSandboxPlan({ selections = [], testedId, deck = [], opponentDeck = [], options = {}, bothPlayers = true, startingLife = 20 }, random = Math.random) {
  if (!Number.isInteger(Number(startingLife)) || Number(startingLife) < 1 || Number(startingLife) > 1000000) throw new Error('Starting life must be between 1 and 1,000,000');
  const tested = selections.find(card => card.id === testedId);
  if (!tested) throw new Error('Choose a card to test');
  const custom = [];
  for (const selection of selections) {
    const copies = Number(selection.copies);
    if (!Number.isInteger(copies) || copies < 1 || copies > 1000) throw new Error('Copies must be between 1 and 1,000');
    if (!selection.draft?.faces?.[0]?.name?.trim()) throw new Error('A selected card has no name');
    custom.push(...Array.from({ length: copies }, () => ({ draft: selection.draft, key: selection.id })));
  }
  const colors = sandboxColors(tested.draft);
  const lands = colors.length ? colors.map(color => BASICS[color]) : ['Wastes'];
  const creatures = colors.map(color => CREATURES[color]);
  // Colorless tests use a custom vanilla body rather than an artifact with an ability.
  const vanilla = colors.length ? null : { layout: 'single', hasFuse: false, faces: [{
    name: 'Sandbox Practice Creature', manaCost: '{2}', cardTypes: ['Artifact', 'Creature'],
    colorIndicator: [], supertypes: [], subtypes: ['Construct'], oracleText: '', power: '2', toughness: '2',
  }] };
  const defaults = count => [
    ...Array.from({ length: 24 }, (_, i) => ({ name: lands[i % lands.length] })),
    ...Array.from({ length: Math.max(0, count - 24) }, (_, i) => vanilla ? { draft: vanilla } : { name: creatures[i % creatures.length] }),
  ];
  const decks = [
    [...(deck.length ? deck.map(name => ({ name })) : defaults(60 - custom.length)), ...custom],
    opponentDeck.length ? opponentDeck.map(name => ({ name })) : defaults(60),
  ];
  if (decks.some(cards => cards.length < 7 || cards.length > 2000)) throw new Error('Each test deck must contain 7 to 2,000 cards');
  const guaranteed = decks[0].findIndex(card => card.key === testedId);
  const [openingCard] = decks[0].splice(guaranteed, 1);
  const players = decks.map((cards, index) => {
    const mixed = shuffled(cards, random);
    return { hand: index === 0 ? [openingCard, ...mixed.splice(0, 6)] : mixed.splice(0, 7), library: mixed };
  });
  const sandboxer = sandboxerDraft(options);
  return { players, startingLife: Number(startingLife), sandboxer: sandboxer.faces[0].oracleText ? sandboxer : null, bothPlayers };
}

export function sandboxCatalogNames(plan) {
  return [...new Set(plan.players.flatMap(player => [...player.hand, ...player.library]).flatMap(card => card.name ? [card.name] : []))];
}

// One serialized worker operation: failed compilation or placement restores the
// old table, including its pending decisions. This uses normal engine abilities.
export async function startSandbox(game, plan) {
  const saved = game.createRuntimeSavepoint();
  try {
    const entries = plan.players.flatMap(player => [...player.hand, ...player.library]);
    const drafts = [...new Map([...entries.filter(card => card.draft).map(card => [JSON.stringify(card.draft), card.draft]),
      ...(plan.sandboxer ? [[JSON.stringify(plan.sandboxer), plan.sandboxer]] : [])]).values()];
    const compiledDrafts = new Map();
    for (const draft of drafts) {
      const prepared = await prepareCustomCard(normalizeDraftForApi(draft), input => game.previewCustomCard(input));
      compiledDrafts.set(JSON.stringify(draft), prepared.input);
    }
    game.resetEmpty(['Player 1', 'Player 2'], plan.startingLife);
    const created = [];
    for (const [playerIndex, player] of plan.players.entries()) {
      for (const zoneName of ['library', 'hand']) {
        for (const card of player[zoneName]) {
          const id = card.draft
            ? await game.createCustomCard({ draft: compiledDrafts.get(JSON.stringify(card.draft)), playerIndex, zoneName, skipTriggers: true })
            : await game.addCardToZone(playerIndex, card.name, zoneName, true);
          if (card.draft) created.push({ id, draft: card.draft });
        }
      }
      if (plan.sandboxer && (playerIndex === 0 || plan.bothPlayers)) {
        const id = await game.createCustomCard({ draft: compiledDrafts.get(JSON.stringify(plan.sandboxer)), playerIndex, zoneName: 'battlefield', skipTriggers: true });
        created.push({ id, draft: plan.sandboxer });
      }
    }
    game.setPerspective(0);
    game.finishPuzzleSetup();
    return { created };
  } catch (error) {
    game.restoreRuntimeSavepoint(saved);
    throw error;
  } finally {
    game.releaseRuntimeSavepoint(saved);
  }
}
