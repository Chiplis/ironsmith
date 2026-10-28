import { MTG_KEYWORD_REMINDERS } from './mtg-keyword-reminders.generated.js';

// Full-rule expansions for official reminders that abbreviate a dependency.
// These return ordinary rules prose. Custom names use that same prose without
// acquiring an official identity; literal custom definitions remain literal.
export function expandOfficialReminderDefinition(reminder, rule, declaration = '', fullOracleText = '') {
  if (rule === '702.174') return expandKnownReminderDefinition(reminder);
  if (rule === '702.62') {
    const match = String(declaration).match(/^Suspend\s+(\d+)\s*[—–-]\s*(.+)/i);
    if (match) return `If you could begin to cast this card from your hand, you may pay ${match[2].replace(/\.$/, '')} and exile it with ${match[1]} time counters on it. This action doesn't use the stack. At the beginning of your upkeep, if this card is exiled with a time counter on it, remove a time counter from it. When the last time counter is removed from this card, if it's exiled, you may play it without paying its mana cost. If you cast a creature spell this way, it gains haste until you lose control of the spell or the permanent it becomes.`;
  }
  if (rule === '702.102') return "You may cast one or both halves of this card from your hand. If you cast both, combine their characteristics and mana costs into one spell, then follow the left half's instructions before the right half's instructions.";
  if (rule === '702.113') {
    const match = String(declaration).match(/^Awaken\s+(\d+)\s*[—–-]\s*(.+)/i);
    if (match) return `You may cast this spell for ${match[2].replace(/\.$/, '')} rather than pay its mana cost. If you do, put ${match[1]} +1/+1 counters on target land you control. That land becomes a 0/0 Elemental creature with haste. It's still a land.`;
  }
  if (rule === '702.103' || rule === '702.140') {
    const price = String(declaration).replace(/^(?:Bestow|Mutate)\s*[—–-]?\s*/i, '').replace(/\.\s*$/, '');
    return rule === '702.103'
      ? `You may cast this spell for ${price} rather than pay its mana cost as an Aura enchantment spell with enchant creature. If its target is illegal as it begins resolving, it instead resolves using its normal types. If the resulting Aura becomes unattached or phases in unattached, it returns to its normal types.`
      : `You may cast this spell for ${price} rather than pay its mana cost targeting a non-Human creature with the same owner as this spell. As it resolves, if that target is legal, merge this spell above or below that creature without entering the battlefield. The resulting permanent has the top object's characteristics and all abilities of the merged objects. Otherwise, this spell resolves normally as a creature.`;
  }
  if (rule === '702.170') return String(reminder).replace(/Plot only as a sorcery\.?$/i, 'Activate only as a sorcery.');
  if (rule === '702.192') return "If this is the first time a spell you control with this spell's name has resolved this game, at the beginning of each of your precombat main phases for the rest of the game, create a copy of this object in exile. You may cast the copy without paying its mana cost. Exile this spell.";
  if (rule === '702.165') {
    const lines = String(fullOracleText).split('\n');
    const index = lines.findIndex(line => /^Backup\b/i.test(line));
    return index < 0 ? reminder : [reminder, ...lines.slice(index + 1)].join('\n');
  }
  if (rule === '702.160') {
    const match = String(declaration).match(/^Prototype\s+((?:\{[^}]+\})+)\s*[—–-]\s*(\d+)\/(\d+)/i);
    if (match) return `You may cast this spell with mana cost ${match[1]}, color determined by that mana cost, and base power and toughness ${match[2]}/${match[3]}. It keeps its abilities and types.`;
  }
  if (rule === '702.94') {
    const price = String(declaration).replace(/^Miracle\s*[—–-]?\s*/i, '').replace(/\.\s*$/, '');
    return `You may reveal this card from your hand as you draw it if it is the first card you drew this turn. When you reveal it this way, you may cast it for ${price} rather than pay its mana cost.`;
  }
  if (rule === '701.67') {
    const match = String(declaration).match(/^(?:Exhaust\s*[—–-]\s*)?Waterbend\s+(\{[^}]+\}):\s*(.*)$/i);
    if (match) return `Pay ${match[1]} using any combination of mana and tapping untapped artifacts and creatures you control for {1} each: ${match[2]}${/^Exhaust\b/i.test(declaration) ? ' Activate this ability only once.' : ''}`;
    const additional = String(declaration).match(/^As an additional cost to cast this spell, waterbend\s+(\{[^}]+\})\.?$/i);
    if (additional) return `As an additional cost to cast this spell, pay ${additional[1]} using any combination of mana and tapping untapped artifacts and creatures you control for {1} each.`;
  }
  if (rule === '702.21' && /Waterbend/i.test(declaration)) {
    const price = String(declaration).match(/Waterbend\s+(\{[^}]+\})/i)?.[1];
    if (price) return `Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays ${price} using any combination of mana and tapping untapped artifacts and creatures you control for {1} each.`;
  }
  if (rule === '702.119') {
    const match = String(declaration).match(/^Emerge(?: from ([a-z ]+))?\s+(\{.*)$/i);
    if (match) return `You may cast this spell from your hand for ${match[2].replace(/\.$/, '')} and sacrifice a${/^[aeiou]/i.test(match[1] || 'creature') ? 'n' : ''} ${match[1] || 'creature'} rather than pay its mana cost. Reduce that price by an amount of generic mana equal to the sacrificed permanent's mana value.`;
  }
  if (rule === '702.95') return "You may pair creatures using this ability. Whenever this creature or another creature enters under your control, if this creature and an eligible other creature are both unpaired creatures you control, you may pair them. If this creature entered, choose another unpaired creature you control; otherwise pair it only with the creature that entered. They remain paired until either leaves the battlefield, stops being a creature, or stops being under your control. Pairing doesn't target.";
  if (rule === '702.49') {
    return String(reminder).replace('to hand:', "to its owner's hand:").replace(/tapped and attacking\.$/, 'tapped and attacking the same player, planeswalker, or battle as the returned creature.');
  }
  if (rule === '702.190') {
    const price = String(declaration).replace(/^Sneak\s*[—–-]?\s*/i, '').replace(/\.\s*$/, '');
    return `During your declare blockers step, any time you could cast an instant, you may cast this spell from your hand for ${price} and return an unblocked attacker you control to its owner's hand rather than pay this spell's mana cost. It enters tapped and attacking the same player, planeswalker, or battle as the returned creature.`;
  }
  if (rule === '702.185') {
    const price = String(declaration).replace(/^Warp\s*[—–-]?\s*/i, '').replace(/\.\s*$/, '');
    return `You may cast this card from your hand for ${price} rather than its mana cost. If you do, exile the permanent this spell becomes at the beginning of the next end step. Its owner may cast that card after this turn ends for as long as it remains exiled.`;
  }
  if (rule === '702.22') return "As you declare attackers, you may group one or more attacking creatures with this ability and up to one other attacking creature into a band. Each creature can belong to only one band, and all members must attack the same player, planeswalker, or battle. The band lasts for the rest of combat even if this ability is lost; creatures removed from combat leave the band. Members don't share abilities. A creature blocking any member blocks all members, and an effect that blocks one member blocks the whole band. If this creature is blocking an attacker, you choose how that attacker divides its combat damage among its blockers. If this creature is blocked, you choose how its blockers divide their combat damage among the creatures they block.";
  if (rule === '702.139') {
    const condition = String(declaration).replace(/^Companion\s*[—–-]\s*/i, '').replace(/\.\s*$/, '');
    return `Before the game begins, you may reveal this card from outside the game as your chosen companion if ${condition.charAt(0).toLowerCase() + condition.slice(1)}. Once during the game, during a main phase of your turn while you have priority and the stack is empty, you may pay {3} to put your chosen companion into your hand. This is a special action that doesn't use the stack.`;
  }
  if (rule === '702.154') return "As this creature attacks, you may tap up to one untapped creature you control that you didn't choose to attack with and that either has haste or has been under your control continuously since this turn began. When you do, this creature gets +X/+0 until end of turn, where X is the tapped creature's power.";
  if (rule === '701.71') {
    const amount = String(declaration).match(/\bEmpower Jace\s+([0-9]+|[XYZ])\b/i)?.[1] || 'X';
    return `If you don't control a Jace planeswalker token, create a blue Jace planeswalker token with 0 loyalty, "[-1]: Surveil 1" and "[-3]: Draw a card." Put ${amount} loyalty counters on a Jace planeswalker token you control.`;
  }
  if (rule === '702.193') {
    const ability = String(declaration).replace(/^Power[- ]?up\s*[—–-]\s*/i, '').replace(/\.\s*$/, '');
    return `${ability}. If this permanent entered this turn, this ability's cost is reduced by this permanent's mana cost. Activate this ability only once.`;
  }
  if (rule === '702.158') return "As a state-based action, each creature without a sector designation is assigned alpha, beta, or gamma sector by its controller. Players who don't control a permanent with this ability assign first, then the other players assign, in active-player order within each group. A permanent keeps its sector until no player controls a permanent with this ability or an ability whose source has this ability. Sector designations aren't copiable.";
  if (rule === '702.179') return "If you have no speed while you control this permanent, your speed becomes 1 as a state-based action. Whenever one or more opponents lose life during your turn, if your speed is less than 4, your speed increases by 1. This inherent ability has no source and triggers only once each turn. Your speed remains after this permanent leaves the battlefield. Max speed is 4.";
  if (rule === '701.49') return "If you don't own a dungeon in the command zone, choose a dungeon from outside the game, put it into the command zone, and put your marker on its first room. Otherwise, move your marker to an adjacent room following an arrow. If your marker is in the final room, first remove that dungeon from the game and complete it, then start a new dungeon.";
  if (rule === '702.144') return "When you cast this spell, you may copy it and you may choose new targets for the copy. If you copy the spell, choose an opponent. That player copies the spell and may choose new targets for that copy.";
  if (rule === '702.96' || rule === '702.148') {
    const label = rule === '702.96' ? /^Overload\b/i : /^Cleave\b/i;
    const body = String(fullOracleText).split('\n').filter(line => line.trim() && !label.test(line));
    return [reminder, ...body].join('\n');
  }
  if (rule === '702.155') {
    const definition = "As this Saga enters, choose a number between one and its final chapter number. It enters with the chosen number of lore counters. Chapter abilities of this Saga can't trigger the turn it entered unless it has exactly the number of lore counters specified in that chapter's symbol.";
    const lines = String(fullOracleText).split('\n');
    const index = lines.findIndex(line => /^Read ahead\b/i.test(line));
    return index < 0 ? definition : [definition, ...lines.slice(index + 1)].join('\n');
  }
  if (rule === '701.64') return "If this permanent isn't harnessed, it becomes harnessed.";
  if (rule === '701.47') return String(reminder).replace(/on an Army you control/g, 'on an Army creature you control').replace(/control an Army,/g, 'control an Army creature,');
  if (rule === '701.22' && /create a Treasure token/i.test(declaration)) {
    return String(reminder).replace(/A Treasure token is an artifact with (".*")\.?$/i, 'Create a colorless Treasure artifact token with $1.');
  }
  if (rule === '702.172' || rule === '702.183') {
    // These reminders define a header, not its missing branch programs. Keep
    // the source card's complete cost/mode block as the definition's parameter.
    const lines = String(fullOracleText).split('\n');
    const index = lines.findIndex(line => /^(?:Spree|Tiered)\b/i.test(line));
    if (index >= 0) return [reminder, ...lines.slice(index + 1)].join('\n');
  }
  if (rule === '702.132') return "As you cast this spell, before activating mana abilities, you may choose another player. That player may activate mana abilities before you do and may pay any amount of the generic mana in this spell's total cost before you pay the rest.";
  if (rule === '701.66') {
    // CR 701.66a returns the land to the instruction's controller, not owner.
    return String(reminder).replace(/return it to the battlefield tapped\.$/i, 'return it to the battlefield tapped under your control.');
  }
  if (rule === '701.30') {
    const opponent = String(declaration).match(/clash with (an opponent|target opponent|defending player)\.?$/i)?.[1] || 'an opponent';
    return `You and ${opponent} each reveal the top card of your library, then each puts that card on their choice of the top or bottom. A player wins if their card had a greater mana value.`;
  }
  if (rule === '702.145') {
    if (/^Nightbound\b/i.test(declaration)) return "If it's neither day nor night and no permanent establishes day, it becomes night. As it becomes day, if this permanent is back face up, transform it. This permanent can't transform except due to this ability. If a player casts at least two spells during their own turn, it becomes day next turn.";
    if (/^Daybound\b/i.test(declaration)) return "If it's neither day nor night, it becomes day. If it's night, this permanent enters transformed. As it becomes night, if this permanent is front face up, transform it. This permanent can't transform except due to this ability. If a player casts no spells during their own turn, it becomes night next turn.";
  }
  if (rule === '702.70') {
    // CR 702.70a: each creature owns its granted trigger. A global trigger on
    // the grantor would use the wrong controller and source for other players.
    const grant = String(declaration).match(/^(.*?) have poisonous (\d+)\.?$/i);
    if (grant) return `${grant[1]} have "Whenever this creature deals combat damage to a player, that player gets ${grant[2]} poison counter${grant[2] === '1' ? '' : 's'}."`;
  }
  if (rule === '702.76') {
    // CR 702.76a: compare damage-time source types with the spell's current
    // creature types. The printed types are examples, not a frozen list.
    return String(reminder).replace(/(\bif you dealt combat damage to a player this turn with )a [a-z]+(?: or [a-z]+)*\.$/i,
      '$1a source that shares a creature type with this spell.');
  }
  return reminder;
}

// Recognize published, unambiguous shorthand by its definition, not by the
// user's keyword name. Parametric definitions still bind their own declaration.
export function expandKnownReminderDefinition(reminder, declaration = '') {
  const text = String(reminder).replace(/[‘’]/g, "'").trim();
  // Recognize the authored definition, never a custom name. Preserve price
  // slots so the caller can bind any declared price, including nonmana costs.
  let match;
  if ((match = text.match(/^You may cast this card for (.+?) when you draw it if it's the first card you drew this turn\.$/i)))
    return expandOfficialReminderDefinition(text, '702.94', `Miracle ${match[1]}`);
  if ((match = text.match(/^If you cast this card for (.+?), it's an Aura spell with enchant creature\. It becomes a creature again if it's not attached\.$/i)))
    return expandOfficialReminderDefinition(text, '702.103', `Bestow ${match[1]}`);
  if ((match = text.match(/^If you cast this spell for (.+?), put it over or under target non-Human creature you own\. They mutate into the creature on top plus all abilities from under it\.$/i)))
    return expandOfficialReminderDefinition(text, '702.140', `Mutate ${match[1]}`);
  const number = value => ({ a: 1, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10 }[value.toLowerCase()] ?? (/^\d+$/.test(value) ? Number(value) : null));
  if ((match = text.match(/^If you cast this spell for (.+?), also put (\w+) \+1\/\+1 counters? on target land you control and it becomes a 0\/0 Elemental creature with haste\. It's still a land\.$/i)) && number(match[2]) !== null)
    return expandOfficialReminderDefinition(text, '702.113', `Awaken ${number(match[2])}—${match[1]}`);
  if ((match = text.match(/^Rather than cast this card from your hand, you may pay (.+?) and exile it with (\w+) time counters? on it\. At the beginning of your upkeep, remove a time counter\. When the last is removed, you may cast it without paying its mana cost\. It has haste\.$/i)) && number(match[2]) !== null)
    return expandOfficialReminderDefinition(text, '702.62', `Suspend ${number(match[2])}—${match[1]}`);
  // The same timed exile with its own counter kind, an optional haste clause
  // and optional extra removal events ("…upkeep, or whenever …, remove …").
  // It reads as Suspend's full rule over that counter, without its identity.
  if ((match = text.match(/^Rather than cast this card from your hand, you may pay (.+?) and exile it with (\w+) ([a-z][a-z-]*) counters? on it\. At the beginning of your upkeep(?:, or ([^.]+?))?, remove an? \3 counter(?: from it)?\. When the last(?: \3 counter)? is removed, you may cast it without paying its mana cost\.(?: (It has haste|If it's a creature, it has haste)\.)?$/i)) && number(match[2]) !== null) {
    const [, price, count, kind, extra, haste] = match;
    const counter = kind.toLowerCase();
    const lines = [`If you could begin to cast this card from your hand, you may pay ${price} and exile it with ${number(count)} ${counter} counters on it. This action doesn't use the stack. At the beginning of your upkeep, if this card is exiled with a ${counter} counter on it, remove a ${counter} counter from it. When the last ${counter} counter is removed from this card, if it's exiled, you may play it without paying its mana cost.${haste ? ' If you cast a creature spell this way, it gains haste until you lose control of the spell or the permanent it becomes.' : ''}`];
    for (const event of extra ? extra.split(/,?\s+or\s+(?=whenever\b)/i) : []) {
      if (!/^whenever\b/i.test(event.trim())) return reminder;
      lines.push(`${event.trim()[0].toUpperCase()}${event.trim().slice(1)}, if this card is exiled with a ${counter} counter on it, remove a ${counter} counter from it.`);
    }
    return lines.join('\n');
  }
  if (/^You may cast this spell with different mana cost, color, and size\. It keeps its abilities and types\.$/i.test(text)) {
    const parameters = String(declaration).match(/((?:\{[^}]+\})+)\s*[—–-]\s*(\d+)\/(\d+)\.?$/);
    if (parameters) return expandOfficialReminderDefinition(text, '702.160', `Prototype ${parameters[1]}—${parameters[2]}/${parameters[3]}`);
  }
  if (/^You may cast one or both halves of this card from your hand\.$/i.test(text))
    return expandOfficialReminderDefinition(text, '702.102');
  if ((match = text.match(/^You may cast this card from your hand for (.+?)\. If you do, exile this (?:creature|permanent) at the beginning of the next end step, then you may cast it from exile on a later turn\.$/i)))
    return expandOfficialReminderDefinition(text, '702.185', `Warp ${match[1]}`);
  if ((match = text.match(/^You may promise an opponent a gift as you cast this spell\. If you do, they (.+?) before its other effects\.(?: (.*))?$/i))) {
    const benefit = `the chosen player ${match[1]}`;
    return `As an additional cost to cast this spell, you may choose an opponent. If this is a permanent, when it enters, if you paid that cost, ${benefit}. If this is an instant or sorcery spell, if you paid that cost, ${benefit} before its other effects.${match[2] ? ` ${match[2]}` : ''}`;
  }

  const normalize = value => String(value).normalize('NFKC').replace(/[‘’]/g, "'").replace(/\s+/g, ' ').trim().toLowerCase();
  for (const [name, rule] of [['soulbond', '702.95'], ['banding', '702.22'], ['enlist', '702.154'], ['space sculptor', '702.158'], ['start your engines!', '702.179'], ['venture into the dungeon', '701.49'], ['demonstrate', '702.144'], ['daybound', '702.145'], ['nightbound', '702.145'], ['assist', '702.132'], ['read ahead', '702.155'], ['harness', '701.64'], ['earthbend', '701.66']]) {
    const known = MTG_KEYWORD_REMINDERS[name];
    if (known && normalize(known.text) === normalize(reminder)) return expandOfficialReminderDefinition(reminder, rule, known.example || name);
  }
  return reminder;
}

