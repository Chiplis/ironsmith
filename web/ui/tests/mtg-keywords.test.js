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
