import test from "node:test";
import assert from "node:assert/strict";
import {
  splitPrintedLines,
  splitPrintedSentences,
  translatePrintedPrompt,
} from "../src/lib/decision-text-translation.js";

const IVY_EN = "Flying\nWhenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.";
// Verbatim from public/card-i18n/es/by-name/iv.json, reminder text included.
const IVY_ES = "Vuela.\nSiempre que un jugador lance un hechizo que solo haga objetivo a una \u00fanica criatura que no sea Ivy, robahechizos jubilosa, puedes copiar ese hechizo. La copia hace objetivo a Ivy. (Una copia de un hechizo de Aura se convierte en una ficha.)";

const translate = (prompt, englishText = IVY_EN, translatedText = IVY_ES, locale = "es") =>
  translatePrintedPrompt({ prompt, englishText, translatedText, locale });

test("a may prompt becomes the localized clause without its lead-in", () => {
  assert.equal(translate("Copy that spell"), "Copiar ese hechizo");
});

test("a whole printed sentence maps to its localized sentence", () => {
  assert.equal(translate("The copy targets Ivy"), "La copia hace objetivo a Ivy");
});

test("a multi-sentence prompt keeps the printed order", () => {
  const english = "At the beginning of your upkeep, you may draw a card. Gain 2 life.";
  const spanish = "Al comienzo de tu mantenimiento, puedes robar una carta. Ganas 2 vidas.";

  assert.equal(
    translate("Draw a card. Gain 2 life", english, spanish),
    "Robar una carta. Ganas 2 vidas"
  );
});

test("English keeps the English prompt", () => {
  assert.equal(translate("Copy that spell", IVY_EN, IVY_ES, "en"), null);
});

test("a localized text with a different ability count is refused", () => {
  assert.equal(translate("Copy that spell", IVY_EN, "Vuela."), null);
});

test("a localized line with a different sentence count is refused", () => {
  const spanish = "Vuela.\nSiempre que un jugador lance un hechizo, puedes copiar ese hechizo.";

  assert.equal(translate("Copy that spell", IVY_EN, spanish), null);
});

test("a prompt that is not in the printed text is refused", () => {
  assert.equal(translate("Perform the effect"), null);
  assert.equal(translate(""), null);
});

test("a sentence outside the quoted ability does not drag another line in", () => {
  const english = "Flying\nYou may draw a card.\nSacrifice a creature: Gain 2 life.";
  const spanish = "Vuela.\nPuedes robar una carta.\nSacrifica una criatura: Ganas 2 vidas.";

  assert.equal(translate("Draw a card. Gain 2 life", english, spanish), null);
});

test("a localized sentence without an optional marker keeps its lead-in", () => {
  const english = "When this enters, you may draw a card.";
  const spanish = "Cuando entre, roba una carta.";

  assert.equal(translate("Draw a card", english, spanish), "Cuando entre, roba una carta");
});

test("reminder text is dropped so official printings still line up", () => {
  const english = "Flying\nKicker {2}. Draw a card.";
  const spanish = "Vuela. (Esta criatura no puede ser bloqueada.)\nKicker {2}. (Puedes pagar un coste adicional.) Roba una carta.";

  assert.equal(translate("Draw a card", english, spanish), "Roba una carta");
});

test("the whole text box maps as a card, not as a sentence", () => {
  const joined = "Flying; Whenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.";

  assert.match(translate(joined), /^Vuela\.; Siempre que un jugador lance/);
});

const YAWGMOTH_EN = "Protection from Humans\nPay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n{B}{B}, Discard a card: Proliferate.";
// Verbatim from public/card-i18n/es/by-name/ya.json.
const YAWGMOTH_ES = "Protecci\u00f3n contra Humanos.\nPagar 1 vida, sacrificar otra criatura: Pon un contador -1/-1 sobre hasta una criatura objetivo y roba una carta.\n{B}{B}, descartar una carta: Prolifera. (Elige cualquier cantidad de permanentes y/o jugadores, luego pon sobre cada uno un contador de cada tipo que ya tenga.)";

test("a cost prompt maps to the same component of the printed cost clause", () => {
  assert.equal(translate("Pay 1 life", YAWGMOTH_EN, YAWGMOTH_ES), "Pagar 1 vida");
  assert.equal(
    translate("Sacrifice another creature", YAWGMOTH_EN, YAWGMOTH_ES),
    "Sacrificar otra criatura"
  );
  assert.equal(translate("Discard a card", YAWGMOTH_EN, YAWGMOTH_ES), "Descartar una carta");
});

test("a cost that several abilities share is refused", () => {
  const english = "Pay 1 life: Draw a card.\nPay 1 life: Gain 2 life.";
  const spanish = "Pagar 1 vida: Roba una carta.\nPagar 1 vida: Ganas 2 vidas.";

  assert.equal(translate("Pay 1 life", english, spanish), null);
});

test("sentence splitting matches the engine rule", () => {
  assert.deepEqual(splitPrintedSentences("Draw a card. Then discard a card."), [
    "Draw a card.",
    "Then discard a card.",
  ]);
  assert.deepEqual(splitPrintedSentences("Add {C}{C}. Spend this mana only on X."), [
    "Add {C}{C}.",
    "Spend this mana only on X.",
  ]);
  assert.deepEqual(splitPrintedLines("Vuela.\n\nSiempre que...\n"), ["Vuela.", "Siempre que..."]);
});
