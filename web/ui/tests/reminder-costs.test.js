import test from 'node:test';
import assert from 'node:assert/strict';
import { bindReminderCost, hasReminderCostReference } from '../src/lib/reminder-costs.js';

test('cost references work for official or custom names without changing fixed numbers', () => {
  for (const keyword of ['escape', 'flashback', 'moon return']) {
    const text = `You may cast this card for its ${keyword} cost. Pay {2} rather than its mana cost.`;
    assert.equal(bindReminderCost(text, keyword, '{B}, Exile five other cards from your graveyard'), `You may cast this card for {B}, Exile five other cards from your graveyard. Pay {2} rather than its mana cost.`);
  }
});

test('foreign and quoted references are not rebound and missing prices are explicit', () => {
  const quoted = 'It gains "Pay its escape cost: Draw a card."';
  assert.equal(bindReminderCost(quoted, 'escape', '{B}'), quoted);
  assert.equal(hasReminderCostReference(quoted, 'escape'), false);
  assert.equal(bindReminderCost('Pay its escape cost.', 'recur', '{B}'), 'Pay its escape cost.');
  assert.equal(bindReminderCost('Pay its mana cost.', 'mana', '{B}'), 'Pay its mana cost.');
  assert.throws(() => bindReminderCost('Pay its recur cost.', 'recur', ''), /requires a declared cost/);
  assert.throws(() => bindReminderCost('Pay its recur cost.', 'recur', 'its recur cost'), /concrete cost/);
});

test('declaration binding preserves Escape mana and every additional payment', async () => {
  const { bindReminderCostFromDeclaration } = await import('../src/lib/reminder-costs.js');
  const source = 'You may cast this card from your graveyard for its escape cost.';
  const bound = bindReminderCostFromDeclaration(source, 'Escape—{4}{G}{U}, Exile a land you control, Exile five other cards from your graveyard.');
  assert.equal(bound.text, 'You may cast this card from your graveyard for {4}{G}{U}, Exile a land you control, Exile five other cards from your graveyard.');
  assert.equal(bound.binding.keyword, 'escape');
  assert.equal(bindReminderCostFromDeclaration(source, 'Other {B}').binding, null);
  assert.equal(bindReminderCostFromDeclaration('Pay its mana cost.', 'Mana {2}').binding, null);
  assert.equal(bindReminderCostFromDeclaration('Pay its suspend cost.', 'Suspend 6—{R}').text, 'Pay {R}.');
});


test('custom activation labels resolve only their own unquoted timing reference', async () => {
  const { bindReminderActivationLabel } = await import('../src/lib/reminder-costs.js');
  const text = '{2}: Put a charge counter on this. Moon ritual only as a sorcery.';
  assert.equal(bindReminderActivationLabel(text, 'moon ritual'), '{2}: Put a charge counter on this. Activate only as a sorcery.');
  assert.equal(bindReminderActivationLabel(text, 'other'), text);
  const quoted = 'It gains "Moon ritual only as a sorcery."';
  assert.equal(bindReminderActivationLabel(quoted, 'moon ritual'), quoted);
});

test('declared variable expressions bind independently of keyword names', async () => {
  const { bindReminderValueFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const name of ['Firebending', 'Battle fuel']) {
    const result = bind('Whenever this creature attacks, add X {R}. This mana lasts until end of combat.', `${name} X, where X is this creature's power.`);
    assert.equal(result.text, "Whenever this creature attacks, add X {R}, where X is this creature's power. This mana lasts until end of combat.");
    assert.equal(result.binding.variable, 'X');
  }
  assert.equal(bind('Draw a card.', 'Fuel X, where X is 3.').binding, null);
  assert.equal(bind('Draw X cards, where X is 2.', 'Fuel X, where X is 3.').binding, null);
  assert.equal(bind('It gains "Add X {R}."', 'Fuel X, where X is 3.').binding, null);
});

test('named prices substitute complete payment actions without a redundant pay verb', () => {
  for (const keyword of ['squad', 'assemble']) {
    assert.equal(bindReminderCost(`You may pay its ${keyword} cost any number of times.`, keyword, 'Exile four cards from your graveyard'), 'You may Exile four cards from your graveyard any number of times.');
    assert.equal(bindReminderCost(`Pay its ${keyword} cost.`, keyword, '{2}'), 'Pay {2}.');
    assert.equal(bindReminderCost(`Pay its ${keyword} cost.`, keyword, 'Pay 3 life'), 'Pay 3 life.');
    assert.equal(bindReminderCost(`It gains "Pay its ${keyword} cost."`, keyword, 'Discard a card'), `It gains "Pay its ${keyword} cost."`);
  }
});

test('paid-copy shorthand expands the declared repeated payment into full rules', async () => {
  const { bindReminderCostFromDeclaration } = await import('../src/lib/reminder-costs.js');
  const reminder = 'When you cast this spell, copy it for each time you paid {2}. You may choose new targets for the copies. Copies become tokens.';
  for (const keyword of ['Replicate', 'Resound']) {
    const result = bindReminderCostFromDeclaration(reminder, `${keyword} {2}`);
    assert.equal(result.binding.expanded_full_rule, true);
    assert.match(result.text, /^As an additional cost to cast this spell, you may pay \{2\} any number of times\./);
    assert.match(result.text, /for each time this spell's additional cost was paid/);
    assert.ok(!result.text.includes(keyword));
  }
  assert.equal(bindReminderCostFromDeclaration(reminder, 'Resound {3}').text, reminder);
});

test('qualified cost declarations bind the shorter cost reference', async () => {
  const { bindReminderCostFromDeclaration } = await import('../src/lib/reminder-costs.js');
  assert.equal(bindReminderCostFromDeclaration('Pay its upkeep cost for each age counter on it.', 'Cumulative upkeep {S}').text, 'Pay {S} for each age counter on it.');
  assert.equal(bindReminderCostFromDeclaration('Pay its ritual cost.', 'Growing ritual—Pay 3 life').text, 'Pay 3 life.');
});

 test('numeric mechanic parameters remain separate from named prices', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  assert.equal(bind('If you cast this spell for its delayed cost, it enters with four time counters.', 'Delayed 4—{2}{R}{R}').text, 'If you cast this spell for {2}{R}{R}, it enters with four time counters.');
});

test('activation reminders bind the complete declaration without native identity', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const name of ['Exhaust', 'Final flourish']) {
    const result = bind(`Activate each ${name.toLowerCase()} ability only once.`, `${name} — {2}, Pay 2 life: Put two +1/+1 counters on this creature.`);
    assert.equal(result.text, '{2}, Pay 2 life: Put two +1/+1 counters on this creature. Activate this ability only once.');
    assert.equal(result.binding.keyword, name);
  }
  for (const name of ['Boast', 'Battle promise']) {
    const reminder = 'Activate only if this creature attacked this turn and only once each turn.';
    assert.equal(bind(reminder, `${name} — {1}: Draw a card.`).text, `{1}: Draw a card. ${reminder}`);
  }
  assert.equal(bind('Activate each other ability only once.', 'Final flourish — {0}: Draw a card.').binding, null);
  assert.equal(bind('Activate only once each turn.', 'Final flourish — {0}').binding, null);
  assert.equal(bind('Activate only once each turn.', 'Final flourish — Draw a card.').binding, null);
});

test('upkeep reveal definitions restore the lasting reveal and owner timing', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Forecast', 'Augury']) {
    const bound = bind('Activate this ability only during your upkeep and only once each turn.', `${keyword} — {2}{W}, Reveal this card from your hand: Tap target untapped creature.`);
    assert.equal(bound.text, "{2}{W}, Reveal this card from your hand until upkeep ends: Tap target untapped creature. Activate only during this card's owner's upkeep and only once each turn.");
  }
  const unrelated = bind('Activate only during your upkeep and only once each turn.', 'Augury — {2}: Draw a card.');
  assert.equal(unrelated.text, '{2}: Draw a card. Activate only during your upkeep and only once each turn.');
});

test('the named cost is also a parameter, preserving full nonmana payment', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Entwine', 'Combine']) {
    assert.equal(bind(`Choose both if you pay the ${keyword.toLowerCase()} cost.`, `${keyword}—{1}, Pay 2 life`).text, 'Choose both if you pay {1}, Pay 2 life.');
  }
  assert.equal(bindReminderCost('Pay the mana cost.', 'mana', '{1}'), 'Pay the mana cost.');
});

test('this cost binds the full declared payment for per-mode definitions', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Escalate', 'Broaden']) {
    assert.equal(bind('Pay this cost for each mode chosen beyond the first.', `${keyword}—{1}, Pay 2 life`).text, 'Pay {1}, Pay 2 life for each mode chosen beyond the first.');
  }
  assert.equal(bindReminderCost('Pay this cost for each mode chosen beyond the first.', 'broaden', 'Discard a card'), 'Discard a card for each mode chosen beyond the first.');
});

test('qualified declarations bind a price only when their spell quality agrees', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Splice', 'Interleave']) {
    const reminder = `As you cast an instant or sorcery spell, you may reveal this card from your hand and pay its ${keyword.toLowerCase()} cost. If you do, add this card's effects to that spell.`;
    const bound = bind(reminder, `${keyword} onto instant or sorcery {2}, Pay 2 life`);
    assert.equal(bound.text, "As you cast an instant or sorcery spell, you may reveal this card from your hand and pay {2}, Pay 2 life. If you do, add this card's effects to that spell.");
    assert.throws(() => bindReminderCost(reminder, keyword.toLowerCase(), 'onto Arcane {1}'), /quality/);
  }
});

test('face-down definitions bind their card operand without rebinding the turn-up pronoun', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Manifest', 'Cloak', 'Obscure']) {
    const suffix = ` onto the battlefield face down as a 2/2 creature${keyword === 'Manifest' ? '' : ' with ward {2}'}. Turn it face up any time for its mana cost if it's a creature card.`;
    const reminder = `To ${keyword.toLowerCase()} a card, put it${suffix}`;
    assert.equal(bind(reminder, `${keyword} the top card of your library, then draw a card.`).text, `Put the top card of your library${suffix}`);
    assert.equal(bind(reminder, `${keyword} one of them.`).text, `Put one of them${suffix}`);
    assert.equal(bind(reminder, 'Draw a card.').binding, null);
  }
});

test('a next-untap reminder fills the self cost and preserves the rest of the activation', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Exert', 'Strain']) {
    assert.equal(bind("An exerted creature won't untap during your next untap step.", `{2}{W}, {T}, ${keyword} this creature: Draw a card.`).text, '{2}{W}, {T}, Choose to have this creature not untap during your next untap step: Draw a card.');
  }
});

test('official type shorthand expands to a live relationship, while literal custom types stay literal', async () => {
  const { expandOfficialReminderDefinition: expand } = await import('../src/lib/official-reminder-definitions.js');
  const printed = 'You may cast this spell for {3}{U} if you dealt combat damage to a player this turn with a Sphinx or Rogue.';
  assert.equal(expand(printed, '702.76'), 'You may cast this spell for {3}{U} if you dealt combat damage to a player this turn with a source that shares a creature type with this spell.');
  assert.equal(expand(printed, null), printed);
});

test('formal exile definitions bind quantified targets and keep the complete price', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Airbend', 'Displace']) {
    const reminder = `To ${keyword.toLowerCase()} a creature, exile it. While it's exiled, its owner may cast it for {2}, Pay 2 life rather than its mana cost.`;
    assert.equal(bind(reminder, `${keyword} up to two target creatures.`).text, "Exile up to two target creatures. While it's exiled, its owner may cast it for {2}, Pay 2 life rather than its mana cost.");
  }
});

test('counted material shorthand restores the full declaration filter', async () => {
  const { bindReminderCostFromDeclaration: bind } = await import('../src/lib/reminder-costs.js');
  for (const keyword of ['Craft', 'Reforge']) {
    const reminder = `{8}{U}, Exile this artifact, Exile the four or more from among other permanents you control and/or cards in your graveyard: Return this card transformed under its owner's control. ${keyword} only as a sorcery.`;
    const bound = bind(reminder, `${keyword} with four or more nonlands with activated abilities {8}{U}`);
    assert.match(bound.text, /Exile four or more nonlands with activated abilities from among other permanents/);
    assert.match(bound.text, /Activate only as a sorcery/);
    assert.throws(() => bind(reminder, `${keyword} with three or more artifacts {8}{U}`), /count/);
  }
});

test('official poisonous grant expands to triggers owned by each recipient', async () => {
  const { expandOfficialReminderDefinition: expand } = await import('../src/lib/official-reminder-definitions.js');
  const reminder = 'Whenever a Sliver deals combat damage to a player, that player gets a poison counter. A player with ten or more poison counters loses the game.';
  assert.equal(expand(reminder, '702.70', 'All Sliver creatures have poisonous 1.'), 'All Sliver creatures have "Whenever this creature deals combat damage to a player, that player gets 1 poison counter."');
  assert.equal(expand(reminder, null, 'All Sliver creatures have Venom 1.'), reminder);
});

test('official day/night shorthand expands transformation and initialization rules', async () => {
  const { expandOfficialReminderDefinition: expand } = await import('../src/lib/official-reminder-definitions.js');
  const printed = 'If a player casts at least two spells during their own turn, it becomes day next turn.';
  const full = expand(printed, '702.145', 'Nightbound');
  assert.match(full, /back face up, transform it/);
  assert.match(full, /can't transform except due to this ability/);
  assert.match(full, /no permanent establishes day/);
  assert.equal(expand(printed, null, 'Moonward'), printed);
});

test('official clash shorthand retains its declared opponent', async () => {
  const { expandOfficialReminderDefinition: expand } = await import('../src/lib/official-reminder-definitions.js');
  assert.match(expand('Each clashing player reveals the top card of their library.', '701.30', 'Whenever Marvo attacks, clash with defending player.'), /^You and defending player each reveal/);
});

test('costed modal reminders keep the source card branch programs', async () => {
  const { expandOfficialReminderDefinition: expand } = await import('../src/lib/official-reminder-definitions.js');
  for (const [keyword, rule, reminder, branches] of [
    ['Spree', '702.172', 'Choose one or more additional costs.', '+ {1} — Draw a card.\n+ {2} — You gain 3 life.'],
    ['Tiered', '702.183', 'Choose one additional cost.', '• Small — {1} — Draw a card.\n• Large — {2} — You gain 3 life.'],
  ]) assert.equal(expand(reminder, rule, keyword, `${keyword} (${reminder})\n${branches}`), `${reminder}\n${branches}`);
});

test('published casting shorthand expands by definition while retaining custom cost slots', async () => {
  const { expandKnownReminderDefinition } = await import('../src/lib/official-reminder-definitions.js');
  for (const [reminder, expected] of [
    ["You may cast this card for its glimpse cost when you draw it if it's the first card you drew this turn.", /reveal this card from your hand as you draw it/],
    ["If you cast this card for its mantle cost, it's an Aura spell with enchant creature. It becomes a creature again if it's not attached.", /If its target is illegal/],
    ["If you cast this spell for its joining cost, put it over or under target non-Human creature you own. They mutate into the creature on top plus all abilities from under it.", /same owner as this spell/],
    ["If you cast this spell for its growth cost, also put four +1/+1 counters on target land you control and it becomes a 0/0 Elemental creature with haste. It's still a land.", /put 4 \+1\/\+1 counters/],
  ]) {
    const expanded = expandKnownReminderDefinition(reminder);
    assert.match(expanded, expected);
    assert.match(expanded, /its [a-z]+ cost/);
  }
  const expanded = expandKnownReminderDefinition("You may cast this spell with different mana cost, color, and size. It keeps its abilities and types.", 'Miniature {2}{R}—3/2');
  assert.match(expanded, /mana cost \{2\}\{R\}, color determined by that mana cost, and base power and toughness 3\/2/);
  assert.equal(expandKnownReminderDefinition('Draw a card.'), 'Draw a card.');
});
