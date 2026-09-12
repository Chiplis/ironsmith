import test from "node:test";
import assert from "node:assert/strict";
import { localizeEnginePhrase, DECISION_REASON_KEYS } from "../src/i18n/decisionPhrases.js";
import { messages } from "../src/i18n/messages.js";

const translator = (locale) => (key, params) => {
  const template = messages[locale]?.[key] ?? key;
  if (!params) return template;
  return String(template).replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name) => (
    params[name] == null ? match : String(params[name])
  ));
};

const es = translator("es");

test("the engine's cost prompt frame localizes and swaps the card name", () => {
  const localized = localizeEnginePhrase("Choose the next cost to pay for Yawgmoth, Thran Physician's ability", {
    t: es,
    cardName: "Yawgmoth, medico thran",
    englishCardName: "Yawgmoth, Thran Physician",
  });

  assert.equal(localized, "Elige el siguiente coste a pagar de la habilidad de Yawgmoth, medico thran");
});

test("an unknown card name is left as the engine wrote it", () => {
  const localized = localizeEnginePhrase("Choose the next cost to pay for Some Other Card's ability", { t: es });

  assert.equal(localized, "Elige el siguiente coste a pagar de la habilidad de Some Other Card");
});

test("every reason the bridge can emit has a message in both locales", () => {
  for (const [reason, key] of Object.entries(DECISION_REASON_KEYS)) {
    assert.ok(messages.en[key], `missing en message for ${reason}`);
    assert.ok(messages.es[key], `missing es message for ${reason}`);
    assert.equal(localizeEnginePhrase(reason, { t: es }), messages.es[key]);
  }
});

test("mana and tap costs localize, and card wording is left alone", () => {
  assert.equal(localizeEnginePhrase("Mana: {2}{B}", { t: es }), "Mana: {2}{B}");
  assert.equal(localizeEnginePhrase("Tap this permanent", { t: es }), "Girar este permanente");
  // Costs quoted from a card are the card-text resolver's job, not this table's.
  assert.equal(localizeEnginePhrase("Sacrifice another creature", { t: es }), null);
  assert.equal(localizeEnginePhrase("", { t: es }), null);
});

test("both locales define the same keys", () => {
  assert.deepEqual(Object.keys(messages.en).sort(), Object.keys(messages.es).sort());
});
