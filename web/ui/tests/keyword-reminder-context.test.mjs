import test from 'node:test';
import assert from 'node:assert/strict';
import {MTG_KEYWORD_REMINDERS} from '../src/lib/mtg-keyword-reminders.generated.js';
import {MTG_KEYWORD_RULES, getMtgKeywordRule, getMtgKeywordHelp, splitTextWithMtgKeywordRules} from '../src/lib/mtg-keywords.js';
import {NUMBER_REMINDERS, COST_REMINDERS, INDIRECT_COSTS, COMPOSITE_REMINDERS, CONTEXTUAL_REMINDERS} from '../src/lib/keyword-reminder-context.js';
import {mseRulesHelperView} from '../src/lib/mse/rules-helper-ranges.mjs';
const help=(key,tail='')=>getMtgKeywordHelp(key,getMtgKeywordRule(key),'en',tail);

test('every catalog entry with sample-specific text has an explicit policy',()=>{
  for(const [key,entry] of Object.entries(MTG_KEYWORD_REMINDERS)) {
    if(!entry.example)continue;
    assert.ok(NUMBER_REMINDERS[key] || COST_REMINDERS[key] || INDIRECT_COSTS.has(key) || COMPOSITE_REMINDERS[key] || CONTEXTUAL_REMINDERS.has(key),key);
    assert.equal(help(key).reminder,null,`${key} must not silently use a sample without context`);
  }
  for(const rule of MTG_KEYWORD_RULES) assert.ok(getMtgKeywordHelp(rule.title,rule).description,rule.title);
});

test('numeric abilities handle singular, plural and X without sample substitution',()=>{
  for(const key of Object.keys(NUMBER_REMINDERS))for(const value of ['1','2','7','X']) {
    const result=help(key,` ${value}`);
    assert.equal(result.reminder?.ability,`${key} ${value}`);
    assert.ok(result.description.includes(value) || (key==='mobilize' && value==='1'),`${key} ${value}`);
    assert.ok(!result.description.includes('undefined'));
  }
  assert.match(help('afterlife',' 3').description,/3 1\/1 white and black Spirit creature tokens/);
  assert.match(help('fabricate',' 2').description,/2 \+1\/\+1 counters.*2 1\/1 colorless Servo/);
  assert.match(help('bushido',' 3').description,/\+3\/\+3/);
  assert.match(help('devour',' 4').description,/4 \+1\/\+1 counters for each creature/);
  assert.match(help('firebending',' 5').description,/5 red mana/);
  assert.match(help('afflict',' 5').description,/loses 5 life/);
  assert.match(help('incubate',' 5').description,/5 \+1\/\+1 counters.*\{2\}.*0\/0/);
  assert.match(help('endure',' 0').description,/Nothing happens/);
  assert.match(help('earthbend',' X, where X is the number of Forests you control.').reminder.ability,/where X is/);
});

test('cost parameters preserve fixed costs, stats and unrelated subtype information',()=>{
  for(const key of [...Object.keys(COST_REMINDERS),...INDIRECT_COSTS]) assert.equal(help(key,' {5}{U/P}').reminder?.ability,`${key} {5}{U/P}`,key);
  for(const key of ['morph','megamorph','disguise']) {
    const result=help(key,' {7}{R}');
    assert.match(result.description,/\{3\}/,key);
    assert.match(result.description,/2\/2/,key);
  }
  assert.match(help('disguise',' {7}{R}').description,/ward \{2\}/);
  assert.match(help('foretell',' {7}{R}').description,/\{2\}/);
  assert.match(help('outlast',' {2}{G}').description,/\{2\}\{G\}, \{T\}: Put a \+1\/\+1 counter/);
  assert.match(help('offspring',' {5}').description,/1\/1 token/);
  assert.ok(!help('embalm',' {2}{W}').description.includes('Cat'));
  assert.ok(!help('eternalize',' {5}{B}').description.includes('Cat'));
  assert.ok(!help('prowl',' {5}{B}').description.includes('Rogue'));
  assert.match(help('ward','—Pay 3 life.').description,/Pay 3 life/);
  assert.match(help('cycling','—Discard two cards.').description,/Discard two cards, Discard this card/);
  assert.equal(help('escape','—{2}{B}, Exile three other cards from your graveyard.').reminder.ability,'escape {2}{B}, Exile three other cards from your graveyard');
});

test('combined numbers and costs are independent slots',()=>{
  for(const key of Object.keys(COMPOSITE_REMINDERS)) {
    const result=help(key,' 7—{2}{U}');
    assert.equal(result.reminder.ability,`${key} 7—{2}{U}`);
    assert.match(result.description,/7/);
    assert.match(result.description,/\{2\}\{U\}/);
  }
  assert.match(help('awaken',' 2—{5}{G}').description,/2 \+1\/\+1 counters.*0\/0/);
  assert.match(help('reinforce',' 1—{W}').description,/1 \+1\/\+1 counter on/);
});

test('local reminders win, are balanced, and cannot come from a neighboring ability',()=>{
  assert.equal(help('cycling',' {G} (Pay this cost (only once): Draw a card.)').description,'Pay this cost (only once): Draw a card.');
  assert.equal(help('gift',' a Food (They create a Food token.)').description,'They create a Food token.');
  assert.ok(!help('cycling',' {G}, flying (Unrelated reminder.)').description.includes('Unrelated'));
  assert.ok(!help('cycling',' {G}.\nWard {2} (Unrelated reminder.)').description.includes('Unrelated'));
  assert.equal(help('cycling',' {G}, vigilance').reminder.ability,'cycling {G}');
  assert.equal(help('ward','—Sacrifice a creature, flying').reminder.ability,'ward Sacrifice a creature');
  assert.match(help('partner with',' Amy, Sky Explorer').reminder.ability,/Amy, Sky Explorer/);
  assert.ok(!help('partner with',' Amy, Sky Explorer').description.includes('Okaun'));
});

test('text and both painted-frame helper ranges retain each keyword occurrence context',()=>{
  const line='Cycling {U}, cycling {3}, mobilize 2';
  const segments=splitTextWithMtgKeywordRules(line).filter(s=>s.type==='keyword');
  assert.deepEqual(segments.map(s=>getMtgKeywordHelp(s.text,s.rule,'en',s.context).reminder.ability),['Cycling {U}','cycling {3}','mobilize 2']);
  const ranges=mseRulesHelperView({lines:[line],actions:new Map()}).ranges.get(0).filter(r=>r.helper.type==='keyword');
  assert.deepEqual(ranges.map(({helper:h})=>getMtgKeywordHelp(h.text,h.rule,'en',h.context).reminder.ability),['Cycling {U}','cycling {3}','mobilize 2']);
});
