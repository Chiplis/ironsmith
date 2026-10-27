// Explicit parameter slots: never replace every occurrence of a sample number
// or mana symbol (token stats, morph costs, and activation costs can be fixed).
const count = (n, noun) => `${n} ${noun}${n === '1' ? '' : 's'}`;
const counters = n => count(n, '+1/+1 counter');
export const NUMBER_REMINDERS = {
  afflict: n => `Whenever this creature becomes blocked, defending player loses ${n} life.`,
  afterlife: n => `When this creature dies, create ${count(n, '1/1 white and black Spirit creature token')} with flying.`,
  amplify: n => `As this creature enters, reveal any number of cards from your hand that share a creature type with it. It enters with ${counters(n)} for each card revealed this way.`,
  annihilator: n => `Whenever this creature attacks, defending player sacrifices ${count(n, 'permanent')} of their choice.`,
  backup: n => `When this creature enters, put ${counters(n)} on target creature. If that's another creature, it gains the non-backup abilities printed below this backup ability until end of turn.`,
  bloodthirst: n => `If an opponent was dealt damage this turn, this creature enters with ${counters(n)} on it.`,
  bolster: n => `Choose a creature with the least toughness among creatures you control and put ${counters(n)} on it.`,
  bushido: n => `Whenever this creature blocks or becomes blocked, it gets +${n}/+${n} until end of turn.`,
  casualty: n => `As you cast this spell, you may sacrifice a creature with power ${n} or greater. When you do, copy this spell. You may choose new targets for the copy.`,
  crew: n => `Tap any number of other untapped creatures you control with total power ${n} or more: This permanent becomes an artifact creature until end of turn.`,
  devour: n => `As this creature enters, you may sacrifice any number of creatures. It enters with ${counters(n)} for each creature sacrificed this way.`,
  discover: n => `Exile cards from the top of your library until you exile a nonland card with mana value ${n} or less. Cast it without paying its mana cost or put it into your hand. Put the remaining exiled cards on the bottom in a random order.`,
  dredge: n => `If you would draw a card and your library has at least ${count(n, 'card')}, you may mill ${count(n, 'card')} instead. If you do, return this card from your graveyard to your hand.`,
  earthbend: n => `Target land you control becomes a 0/0 creature with haste that's still a land. Put ${counters(n)} on it. When it dies or is exiled, return it to the battlefield tapped.`,
  fabricate: n => `When this creature enters, put ${counters(n)} on it or create ${count(n, '1/1 colorless Servo artifact creature token')}.`,
  fading: n => `This permanent enters with ${count(n, 'fade counter')} on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.`,
  firebending: n => `Whenever this creature attacks, add ${n} red mana. This mana lasts until end of combat.`,
  graft: n => `This permanent enters with ${counters(n)} on it. Whenever another creature enters, you may move a +1/+1 counter from this permanent onto it.`,
  hideaway: n => `When this permanent enters, look at the top ${count(n, 'card')} of your library, exile one face down, then put the rest on the bottom in a random order.`,
  incubate: n => `Create an Incubator token with ${counters(n)} on it and "{2}: Transform this token." It transforms into a 0/0 Phyrexian artifact creature.`,
  mobilize: n => `Whenever this creature attacks, create ${n === '1' ? 'a' : n} tapped and attacking 1/1 red Warrior creature token${n === '1' ? '' : 's'}. Sacrifice ${n === '1' ? 'it' : 'them'} at the beginning of the next end step.`,
  modular: n => `This permanent enters with ${counters(n)} on it. When it dies, you may put its +1/+1 counters on target artifact creature.`,
  rampage: n => `Whenever this creature becomes blocked, it gets +${n}/+${n} until end of turn for each creature blocking it beyond the first.`,
  renown: n => `When this creature deals combat damage to a player, if it isn't renowned, put ${counters(n)} on it and it becomes renowned.`,
  ripple: n => `When you cast this spell, you may reveal the top ${count(n, 'card')} of your library. You may cast spells with the same name as this spell from among those cards without paying their mana costs. Put the rest on the bottom of your library.`,
  saddle: n => `Tap any number of other untapped creatures you control with total power ${n} or more: This permanent becomes saddled until end of turn. Activate only as a sorcery.`,
  scry: n => `Look at the top ${count(n, 'card')} of your library. Put any number of them on the bottom and the rest on top in any order.`,
  soulshift: n => `When this permanent dies, you may return target Spirit card with mana value ${n} or less from your graveyard to your hand.`,
  support: n => `Put a +1/+1 counter on each of up to ${count(n, 'target creature')}. If this ability's source is a permanent, it can't target itself.`,
  surveil: n => `Look at the top ${count(n, 'card')} of your library, then put any number of them into your graveyard and the rest on top in any order.`,
  toxic: n => `Players dealt combat damage by this creature also get ${count(n, 'poison counter')}.`,
  tribute: n => `As this creature enters, an opponent of your choice may put ${counters(n)} on it.`,
  vanishing: n => `This permanent enters with ${count(n, 'time counter')} on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.`,
  absorb: n => `If a source would deal damage to this creature, prevent ${n} of that damage.`,
  frenzy: n => `Whenever this creature attacks and isn't blocked, it gets +${n}/+0 until end of turn.`,
  poisonous: n => `Whenever this creature deals combat damage to a player, that player gets ${count(n, 'poison counter')}.`,
  fateseal: n => `Look at the top ${count(n, 'card')} of an opponent's library. Put any number of them on the bottom and the rest on top in any order.`,
  monstrosity: n => `If this creature isn't monstrous, put ${counters(n)} on it and it becomes monstrous.`,
  adapt: n => `If this creature has no +1/+1 counters on it, put ${counters(n)} on it.`,
  'collect evidence': n => `Exile any number of cards from your graveyard with total mana value ${n} or greater.`,
  endure: n => n === '0' ? 'Nothing happens: no counters are put on the permanent and no token is created.' : `Put ${counters(n)} on this permanent or create a ${n}/${n} white Spirit creature token.`,
  blight: n => `Put ${count(n, '-1/-1 counter')} on a creature you control.`,
  mill: n => `A player who mills ${count(n, 'card')} puts that many cards from the top of their library into their graveyard.`,
  connive: n => `Draw ${count(n, 'card')}, then discard ${count(n, 'card')}. Put a +1/+1 counter on the conniving creature for each nonland card discarded this way.`,
};

// These reminders refer to "its ... cost" or "this cost" already. Their other
// numbers and symbols are fixed rules, not the cost printed after the keyword.
export const INDIRECT_COSTS = new Set(['bestow','blitz','cleave','cumulative upkeep','dash','disguise','disturb','echo','emerge','entwine','escalate','escape','evoke','flashback','foretell','freerunning','harmonize','madness','megamorph','miracle','morph','mutate','overload','replicate','spectacle','surge','warp']);
export const COST_REMINDERS = {
  'aura swap': c => `${c}: Exchange this Aura with an Aura card in your hand.`,
  buyback: c => `You may pay an additional cost of ${c} as you cast this spell. If you do, put this card into your hand as it resolves.`,
  cycling: c => `${c}, Discard this card: Draw a card.`,
  'commander ninjutsu': c => `${c}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand or the command zone tapped and attacking.`,
  embalm: c => `${c}, Exile this card from your graveyard: Create a token copy of it, except it's white, is a Zombie in addition to its other types, and has no mana cost. Activate only as a sorcery.`,
  encore: c => `${c}, Exile this card from your graveyard: For each opponent, create a token copy that attacks that opponent this turn if able. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery.`,
  equip: c => `${c}: Attach this Equipment to target creature you control. Activate only as a sorcery.`,
  eternalize: c => `${c}, Exile this card from your graveyard: Create a token copy of it, except it's a 4/4 black Zombie in addition to its other types and has no mana cost. Activate only as a sorcery.`,
  fortify: c => `${c}: Attach this Fortification to target land you control. Activate only as a sorcery.`,
  kicker: c => `You may pay an additional cost of ${c} as you cast this spell.`,
  'level up': c => `${c}: Put a level counter on this permanent. Activate only as a sorcery.`,
  'more than meets the eye': c => `You may cast this card converted for ${c}.`,
  multikicker: c => `You may pay an additional cost of ${c} any number of times as you cast this spell.`,
  ninjutsu: c => `${c}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.`,
  offspring: c => `You may pay an additional cost of ${c} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.`,
  outlast: c => `${c}, {T}: Put a +1/+1 counter on this creature. Activate only as a sorcery.`,
  plot: c => `You may pay ${c} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.`,
  prowl: c => `You may cast this spell for ${c} if you dealt combat damage to a player this turn with a creature that shared a creature type with this spell.`,
  reconfigure: c => `${c}: Attach to target creature you control; or unattach from a creature. Activate only as a sorcery. While attached, this isn't a creature.`,
  recover: c => `When a creature is put into your graveyard from the battlefield, you may pay ${c}. If you do, return this card from your graveyard to your hand. Otherwise, exile it.`,
  scavenge: c => `${c}, Exile this card from your graveyard: Put a number of +1/+1 counters equal to this card's power on target creature. Activate only as a sorcery.`,
  sneak: c => `You may cast this spell for ${c} if you also return an unblocked attacker you control to hand during the declare blockers step.`,
  squad: c => `As an additional cost to cast this spell, you may pay ${c} any number of times. When this creature enters, create that many token copies of it.`,
  transfigure: c => `${c}, Sacrifice this creature: Search your library for a creature card with the same mana value as this creature, put it onto the battlefield, then shuffle. Activate only as a sorcery.`,
  transmute: c => `${c}, Discard this card: Search your library for a card with the same mana value as this card, reveal it, put it into your hand, then shuffle. Activate only as a sorcery.`,
  unearth: c => `${c}: Return this card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Activate only as a sorcery.`,
  ward: c => `Whenever this permanent becomes the target of a spell or ability an opponent controls, counter it unless that player pays the ward cost: ${c}.`,
  'web-slinging': c => `You may cast this spell for ${c} if you also return a tapped creature you control to its owner's hand.`,
};
for (const [keyword, type] of Object.entries({'basic landcycling':'basic land',forestcycling:'Forest',islandcycling:'Island',mountaincycling:'Mountain',plainscycling:'Plains',swampcycling:'Swamp',wizardcycling:'Wizard'})) {
  COST_REMINDERS[keyword] = c => `${c}, Discard this card: Search your library for a ${type} card, reveal it, put it into your hand, then shuffle.`;
}
export const COMPOSITE_REMINDERS = {
  awaken: (n,c) => `If you cast this spell for ${c}, also put ${counters(n)} on target land you control and it becomes a 0/0 Elemental creature with haste. It's still a land.`,
  impending: (n,c) => `If you cast this spell for ${c}, it enters with ${count(n,'time counter')} and isn't a creature until the last is removed. At the beginning of your end step, remove a time counter from it.`,
  reinforce: (n,c) => `${c}, Discard this card: Put ${counters(n)} on target creature.`,
  suspend: (n,c) => `Rather than cast this card from your hand, pay ${c} and exile it with ${count(n,'time counter')} on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost. If it's a creature, it gains haste until you lose control of it.`,
};
// Context-sensitive abilities that cannot be determined from a keyword alone
// (card types, mana cost, Saga chapters, station thresholds, ability bodies).
export const CONTEXTUAL_REMINDERS = new Set(['assist','boast','companion','compleated','craft','exhaust','forecast','haunt','indestructible','max speed','read ahead','solved','station','sunburst','waterbend','prototype','gift','affinity','champion','partner with','protection','hexproof','enchant','splice','offering','amass','devour','equip']);

const NUMBER = /^\s*(\d+|X)(?=$|[\s.,;:()—–-])/i;
const COST_START = /^(?:\{[^{}]+\}|pay\b|discard\b|sacrifice\b|exile\b|return\b|tap\b|remove\b|reveal\b)/i;
function costPrefix(source) {
  const leading = source.match(/^\s*[—–-]?\s*/)[0];
  let value = source.slice(leading.length).split(/[\n().;:]/,1)[0].trim();
  if (!COST_START.test(value)) return null;
  // Commas can separate costs or separate unrelated keyword declarations.
  for (const comma of value.matchAll(/,\s*/g)) {
    if (!COST_START.test(value.slice(comma.index + comma[0].length))) {
      value = value.slice(0,comma.index); break;
    }
  }
  // A pure mana cost ends at its symbols, unless followed by another cost.
  const mana = value.match(/^(?:\{[^{}]+\})+/)?.[0];
  if (mana && value.slice(mana.length) && !/^(?:,|\s+(?:or|and\/or)\s+)/i.test(value.slice(mana.length))) value = mana;
  return {value, end: leading.length + value.length};
}
function ownReminder(source, end) {
  const rest = source.slice(end).replace(/^[ \t]*\.?[ \t]*/, '');
  if (!rest.startsWith('(')) return null;
  let depth = 0;
  for (let i=0;i<rest.length;i++) {
    if (rest[i]==='(') depth++;
    if (rest[i]===')' && --depth===0) return rest.slice(1,i);
  }
  return null;
}

export function contextualKeywordReminder(key, alias, source, catalog, summary) {
  if (key === 'web slinging') key = 'web-slinging';
  const number = source.match(NUMBER);
  let parsed = null, description = null;
  if (COMPOSITE_REMINDERS[key] && number) {
    const tail = source.slice(number[0].length);
    if (/^\s*[—–-]/.test(tail)) {
      const cost = costPrefix(tail);
      if (cost) { parsed={value:`${number[1]}—${cost.value}`,end:number[0].length+cost.end}; description=COMPOSITE_REMINDERS[key](number[1],cost.value); }
    }
  } else if (NUMBER_REMINDERS[key] && number) {
    parsed={value:number[1].toUpperCase(),end:number[0].length}; description=NUMBER_REMINDERS[key](parsed.value);
  } else if (COST_REMINDERS[key] || INDIRECT_COSTS.has(key)) {
    parsed=costPrefix(source);
    if(parsed) description=COST_REMINDERS[key]?.(parsed.value) || catalog?.text || summary;
  }
  if (parsed) {
    const definition=source.slice(parsed.end).match(/^,\s*where X\b[^.\n(]*/i)?.[0] || '';
    return {text:ownReminder(source,parsed.end + definition.length) || description,ability:`${alias} ${parsed.value}${definition}`};
  }
  // Parameterless printed reminders are usable only when immediately adjacent.
  const printed=ownReminder(source,0);
  if(printed) return {text:printed};
  if(CONTEXTUAL_REMINDERS.has(key)) {
    const declaration=source.split(/[\n(]/,1)[0].trim().replace(/\.$/,'');
    if(declaration && /^(?:[—–-]|with\b|for\b|from\b|a\b|an\b|creature\b|artifact\b|land\b|player\b|permanent\b|enchantment\b|instant\b|sorcery\b|\{|[A-Z])/.test(declaration)) {
      const end=source.indexOf(declaration)+declaration.length;
      let explanation = summary;
      if(key === 'affinity' && /^for /i.test(declaration)) explanation = `The affinity cost reduction applies to ${declaration.slice(4)} you control. Each qualifying permanent reduces this spell's cost by {1}.`;
      if(key === 'champion' && /^(a|an) /i.test(declaration)) explanation = `When this permanent enters, sacrifice it unless you exile another ${declaration.replace(/^(a|an) /i,'')} you control. When it leaves the battlefield, return the exiled card under its owner's control.`;
      if(key === 'partner with') explanation = `When this permanent enters, target player may search their library for a card named ${declaration}, reveal it, put it into their hand, then shuffle. These two named cards may be your commanders together.`;
      const army = key === 'amass' && declaration.match(/^([A-Za-z]+) (\d+|X)$/i);
      if(army) explanation = `If you don't control an Army creature, create a 0/0 black ${army[1]} Army creature token. Put ${counters(army[2])} on an Army creature you control. It also becomes a ${army[1]} in addition to its other types.`;
      return {text:ownReminder(source,end)||explanation,ability:`${alias} ${declaration}`};
    }
  }
  // Unparameterized reminders are safe; sample-specific ones are not.
  return catalog && !catalog.example ? catalog : null;
}
