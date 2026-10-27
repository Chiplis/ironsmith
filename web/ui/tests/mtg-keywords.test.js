import test from "node:test";
import assert from "node:assert/strict";
import {
  getMtgKeywordRule,
  splitTextWithMtgKeywordRules,
} from "../src/lib/mtg-keywords.js";

function keywordTexts(text) {
  return splitTextWithMtgKeywordRules(text)
    .filter((segment) => segment.type === "keyword")
    .map((segment) => segment.text);
}

function keywordRules(text) {
  return splitTextWithMtgKeywordRules(text)
    .filter((segment) => segment.type === "keyword")
    .map((segment) => [segment.text, segment.rule.rule]);
}

test("matches keyword abilities and actions from rules text", () => {
  assert.equal(getMtgKeywordRule("flying")?.rule, "702.9");
  assert.equal(getMtgKeywordRule("scry")?.rule, "701.22");

  assert.deepEqual(
    keywordTexts("Flying, vigilance, and first strike"),
    ["Flying", "vigilance", "first strike"],
  );
});

test("uses word boundaries so card names and longer words are not split", () => {
  assert.deepEqual(keywordTexts("Counter target spell"), ["Counter"]);
  assert.deepEqual(keywordTexts("Counterspell is on the stack"), []);
});

test("treats counter as a keyword action only in spell or ability contexts", () => {
  assert.deepEqual(keywordRules("Counter target noncreature spell."), [["Counter", "701.6"]]);
  assert.deepEqual(keywordRules("This spell can't be countered."), [["countered", "701.6"]]);
  assert.deepEqual(keywordRules("Put a +1/+1 counter on target creature."), []);
  assert.deepEqual(keywordRules("Put a vigilance counter on it."), [["vigilance", "702.20"]]);
});

test("matches punctuation variants for compound mechanics", () => {
  assert.deepEqual(
    keywordTexts("Doctor’s companion and web-slinging are active."),
    ["Doctor’s companion", "web-slinging"],
  );
  assert.deepEqual(
    keywordTexts("Partner with a named card."),
    ["Partner with"],
  );
});

test("matches official result and status terms for mechanics", () => {
  assert.deepEqual(keywordRules("Goaded creatures attack each combat if able."), [["Goaded", "701.15"]]);
  assert.deepEqual(keywordRules("A suspected creature has menace."), [["suspected", "701.60"], ["menace", "702.111"]]);
  assert.deepEqual(keywordRules("Cast a plotted card from exile."), [["Cast", "701.5"], ["plotted", "702.170"], ["exile", "701.13"]]);
});

// Exercise the same resolver used by text, MSE, and CardGlint tooltips.
const { getMtgKeywordHelp } = await import('../src/lib/mtg-keywords.js');
const { collectKeywordReminders } = await import('../scripts/generate-mtg-keyword-reminders.mjs');

test('prefers reminder text and retains CR fallback', () => {
  const flying = getMtgKeywordRule('flying');
  assert.match(getMtgKeywordHelp('Flying', flying).description, /can't be blocked/);
  assert.ok(getMtgKeywordHelp('Flying', flying).reminder.card);
  const counter = getMtgKeywordRule('counter');
  assert.equal(getMtgKeywordHelp('counter', counter).description, counter.summary);
  assert.equal(getMtgKeywordHelp('counter', counter).reminder, null);
});

test('keeps variants separate and uses general help without a card parameter', () => {
  const help = alias => getMtgKeywordHelp(alias, getMtgKeywordRule(alias));
  assert.ok(help('cycling').description);
  assert.ok(help('forestcycling').description);
  assert.equal(help('cycling').reminder, null);
  assert.match(help('partner').description, /two commanders/);
  assert.ok(help('partner with').description);
  assert.notEqual(help('daybound').description, help('nightbound').description);
  assert.equal(help('typecycling').reminder, null);
});

test('retains localized CR descriptions', () => {
  const rule = getMtgKeywordRule('flying');
  const help = getMtgKeywordHelp('Volar', rule, 'es');
  assert.equal(help.reminder, null);
  assert.ok(help.description);
  assert.notEqual(help.description, getMtgKeywordHelp('flying', rule).description);
});

test('extracts faces and actions without stealing unrelated or combined reminders', () => {
  const reminders = collectKeywordReminders([
    {name: 'Face pair', keywords: ['Flying', 'Scry'], card_faces: [
      {name: 'Front', oracle_text: 'Flying (This creature can fly.)'},
      {name: 'Back', oracle_text: 'Scry 2. (Look at two cards.)'},
    ]},
    {name: 'Ambiguous', keywords: ['Haste', 'Trample'], oracle_text: 'Haste, trample (Combined explanation.)'},
    {name: 'Mention', keywords: ['Reach'], oracle_text: 'Creatures you control have reach. (Unrelated explanation.)'},
    {name: 'Nested', keywords: ['Vigilance'], oracle_text: 'Vigilance (Outer (inner) text.)'},
  ]);
  assert.deepEqual(Object.keys(reminders), ['flying', 'scry']);
  assert.equal(reminders.flying.card, 'Front');
  assert.equal(reminders.scry.example, 'Scry 2.');
});

test('prefers common general reminders and produces deterministic output', () => {
  const cards = [
    {name: 'B', keywords: ['Hexproof'], oracle_text: 'Hexproof (General reminder.)'},
    {name: 'A', keywords: ['Hexproof'], oracle_text: 'Hexproof (General reminder.)'},
    {name: 'C', keywords: ['Hexproof'], oracle_text: 'Hexproof from white (Specific reminder.)'},
  ];
  assert.deepEqual(collectKeywordReminders(cards), collectKeywordReminders([...cards].reverse()));
  assert.equal(collectKeywordReminders(cards).hexproof.text, 'General reminder.');
  assert.equal(collectKeywordReminders(cards).hexproof.example, undefined);
});

test('keyword reminders use the hovered occurrence cost and number', () => {
  const help = (alias, context) => getMtgKeywordHelp(alias, getMtgKeywordRule(alias), 'en', context);
  assert.match(help('Cycling', ' {1}{U}').description, /^\{1\}\{U\}, Discard/);
  assert.equal(help('Cycling', ' {1}{U}').reminder.ability, 'Cycling {1}{U}');
  assert.match(help('Ward', ' {3}').description, /ward cost: \{3\}/);
  assert.match(help('Annihilator', ' 4').description, /4 permanents/);
  assert.equal(help('Cycling', ' {U} (Actual printed reminder.)').description, 'Actual printed reminder.');
  assert.equal(help('Cycling', '').reminder, null);
  assert.ok(!help('Cycling', ' {U}, flying (Unrelated reminder.)').description.includes('Unrelated'));
  assert.match(help('Ward', '—Pay 3 life.').description, /Pay 3 life/);
});

test('Mobilize uses the current token count without changing token stats', () => {
  const rule = getMtgKeywordRule('Mobilize');
  const help = count => getMtgKeywordHelp('Mobilize', rule, 'en', ` ${count}`);
  assert.match(help(1).description, /create a tapped and attacking 1\/1 red Warrior creature token\. Sacrifice it/);
  for (const count of [2, 3, 5]) {
    assert.equal(help(count).reminder.ability, `Mobilize ${count}`);
    assert.ok(help(count).description.includes(`create ${count} tapped and attacking 1/1 red Warrior creature tokens. Sacrifice them`));
  }
});
