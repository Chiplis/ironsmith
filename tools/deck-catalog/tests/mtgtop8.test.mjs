import test from "node:test";
import assert from "node:assert/strict";
import {
  extractEventLinks,
  extractDeckLinks,
  modernFormatUrl,
  parseDeckPage,
} from "../sources/mtgtop8.mjs";

const fixture = `
<div class=event_title>#1 Dimir Control</div>
<div class=S14><a href=archetype?a=1702>Dimir Control decks</a></div>
<div id=mdabc001 class="deck_line hover_tr">4 <span class=L14>Psychic Frog</span></div>
<div id=mdabc002 class="deck_line hover_tr">2 <span class=L14>Counterspell</span></div>
<div id=sbabc003 class="deck_line hover_tr">2 <span class=L14>Rest in Peace</span></div>
`;

test("builds a bounded Modern catalog URL", () => {
  assert.equal(modernFormatUrl({ meta: 54, page: 2 }), "https://mtgtop8.com/format?a=&f=MO&meta=54&cp=2");
});

test("extracts unique event links from a format page", () => {
  const links = extractEventLinks('<a href=event?e=123&f=MO>one</a><a href=event?e=123&f=MO>duplicate</a><a href=event?e=456&f=MO>two</a>');
  assert.deepEqual(links.map((link) => link.id), ["123", "456"]);
});

test("extracts bounded deck links from an event page", () => {
  const links = extractDeckLinks(
    '<a href=?e=90808&d=889587&f=MO>one</a><a href=?e=90808&d=889588&f=MO>two</a><a href=?e=90808&d=889587&f=MO>duplicate</a>',
    { eventId: "90808", limit: 2 },
  );
  assert.deepEqual(links.map((link) => link.deckId), ["889587", "889588"]);
});

test("parses mainboard, sideboard and placement from an event deck page", () => {
  const deck = parseDeckPage(fixture, {
    eventId: "90808",
    deckId: "889587",
    sourceUrl: "https://mtgtop8.com/event?e=90808&d=889587&f=MO",
  });
  assert.equal(deck.id, "mtgtop8-90808-889587");
  assert.equal(deck.archetype, "Dimir Control");
  assert.equal(deck.placement, 1);
  assert.deepEqual(deck.mainboard, [
    { name: "Counterspell", count: 2 },
    { name: "Psychic Frog", count: 4 },
  ]);
  assert.deepEqual(deck.sideboard, [{ name: "Rest in Peace", count: 2 }]);
});
