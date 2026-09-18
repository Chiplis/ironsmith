import test from "node:test";
import assert from "node:assert/strict";
import {
  extractEventLinks,
  extractEventCollections,
  extractArchetypeLinks,
  extractDeckLinks,
  formatUrl,
  modernFormatUrl,
  normalizeFormat,
  parseDeckPage,
} from "../sources/mtgtop8.mjs";

const fixture = `
<div class=event_title>Modern Challenge</div>
<div class=event_title>#1 Dimir Control</div>
<div class=S14><a href=archetype?a=1702>Dimir Control decks</a></div>
<div id=mdabc001 class="deck_line hover_tr">4 <span class=L14>Psychic Frog</span></div>
<div id=mdabc002 class="deck_line hover_tr">2 <span class=L14>Counterspell</span></div>
<div id=sbabc003 class="deck_line hover_tr">2 <span class=L14>Rest in Peace</span></div>
`;

test("builds a bounded Modern catalog URL", () => {
  assert.equal(modernFormatUrl({ meta: 54, page: 2 }), "https://mtgtop8.com/format?a=&f=MO&meta=54&cp=2");
});

test("normalizes supported formats for MTGTop8 URLs", () => {
  assert.deepEqual(normalizeFormat("pioneer"), { slug: "pioneer", code: "PI" });
  assert.equal(formatUrl({ format: "standard", meta: 54 }), "https://mtgtop8.com/format?a=&f=ST&meta=54");
});

test("extracts unique event links from a format page", () => {
  const links = extractEventLinks('<a href=event?e=123&f=MO>one</a><a href=event?e=123&f=MO>duplicate</a><a href=event?e=456&f=MO>two</a>');
  assert.deepEqual(links.map((link) => link.id), ["123", "456"]);
});

test("extracts Last 20 and Last Major event collections with dates", () => {
  const html = `
    <div class=w_title align=center>LAST MAJOR EVENTS<div class=c_tl></div></div>
    <table><tr class=hover_tr><td><a href=event?e=90001&f=MO>Major Open</a></td><td class=S12>13/09/26</td></tr></table>
    <div class=w_title align=center>LAST 20 EVENTS<div class=c_tl></div></div>
    <table><tr class=hover_tr><td><a href=event?e=90002&f=MO>Weekly Challenge</a></td><td class=S12>17/09/26</td></tr></table>
  `;
  const collections = extractEventCollections(html);
  assert.deepEqual(collections.lastMajorEvents.map(({ id, date }) => ({ id, date })), [{ id: "90001", date: "2026-09-13" }]);
  assert.deepEqual(collections.last20Events.map(({ id, date }) => ({ id, date })), [{ id: "90002", date: "2026-09-17" }]);
});

test("extracts bounded deck links from an event page", () => {
  const links = extractDeckLinks(
    '<a href=?e=90808&d=889587&f=MO>one</a><a href=?e=90808&d=889588&f=MO>two</a><a href=?e=90808&d=889587&f=MO>duplicate</a>',
    { eventId: "90808", limit: 2 },
  );
  assert.deepEqual(links.map((link) => link.deckId), ["889587", "889588"]);
});

test("extracts mono archetype sources for targeted catalog seeding", () => {
  const links = extractArchetypeLinks('<a href=archetype?a=819&meta=51&f=MO>Mono Black Aggro</a><a href=archetype?a=474&meta=51&f=MO>Mono Green Aggro</a><a href=archetype?a=1&meta=51&f=MO>Dimir Control</a>', { limit: 2 });
  assert.deepEqual(links, [
    { name: "Mono Black Aggro", url: "https://mtgtop8.com/archetype?a=819&meta=51&f=MO" },
    { name: "Mono Green Aggro", url: "https://mtgtop8.com/archetype?a=474&meta=51&f=MO" },
  ]);
});

test("parses mainboard, sideboard and placement from an event deck page", () => {
  const deck = parseDeckPage(fixture, {
    eventId: "90808",
    deckId: "889587",
    sourceUrl: "https://mtgtop8.com/event?e=90808&d=889587&f=MO",
  });
  assert.equal(deck.id, "mtgtop8-90808-889587");
  assert.equal(deck.archetype, "Dimir Control");
  assert.equal(deck.event, "Modern Challenge");
  assert.equal(deck.placement, 1);
  assert.deepEqual(deck.mainboard, [
    { name: "Counterspell", count: 2 },
    { name: "Psychic Frog", count: 4 },
  ]);
  assert.deepEqual(deck.sideboard, [{ name: "Rest in Peace", count: 2 }]);
});

test("parses shared placements such as #3-4", () => {
  const html = `
    <div class=event_title>Modern Challenge</div>
    <div class=event_title>#3-4 Scepter Chant - <a class=player_big>Player</a></div>
    <div id=md1 class="deck_line"><span>4</span></div>
    <div id=mdx class="deck_line">4 <span>Counterspell</span></div>
  `;
  const deck = parseDeckPage(html, { eventId: "1", deckId: "2" });
  assert.equal(deck.placement, 3);
  assert.equal(deck.archetype, "Scepter Chant");
});
