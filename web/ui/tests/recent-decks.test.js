import test from 'node:test';
import assert from 'node:assert/strict';
import { deckIdentity, rememberDeckChoice, recordPlayedDeck, readRecentlyPlayedDecks, pickFeaturedDecks } from '../src/lib/recent-decks.js';
test('played history preserves metadata, deduplicates lists, and keeps three across reloads', () => {
  const data = new Map();
  globalThis.localStorage = {getItem: k => data.get(k), setItem: (k,v) => data.set(k,v)};
  try {
    const deck = {deckText:'4 Lightning Bolt\n20 Mountain', commanderText:''};
    rememberDeckChoice(deck, {id:'burn', name:'Boros Burn', artCard:'Lightning Bolt'});
    assert.equal(readRecentlyPlayedDecks().length, 0, 'browsing is not playing');
    recordPlayedDeck(deck);
    for (const name of ['Island','Forest','Swamp']) recordPlayedDeck({deckText:`60 ${name}`});
    recordPlayedDeck({deckText:'20 Mountain\n4 Lightning Bolt'});
    const history = readRecentlyPlayedDecks();
    assert.equal(history.length, 3);assert.equal(history[0].name, 'Boros Burn');
    assert.equal(history[0].artCard, 'Lightning Bolt');
    assert.equal(deckIdentity(deck), deckIdentity({deckText:'20 Mountain\n4 Lightning Bolt'}));
    assert.notEqual(deckIdentity(deck), deckIdentity({...deck,commanderText:'1 Atraxa, Praetors\' Voice'}));
    assert.equal(pickFeaturedDecks(history,[{id:'a'},{id:'b'},{id:'c'}],()=>.5)[0].name,'Boros Burn');
    const random = pickFeaturedDecks([], [{id:'a'},{id:'b'},{id:'c'},{id:'d'}],()=>0);
    assert.equal(new Set(random.map(x=>x.id)).size,3);
  } finally {delete globalThis.localStorage;}
});
test('unavailable or malformed storage does not block playing', () => {
  globalThis.localStorage = {getItem:()=>'{',setItem:()=>{throw Error('full');}};
  try {assert.deepEqual(readRecentlyPlayedDecks(),[]);assert.doesNotThrow(()=>recordPlayedDeck({deckText:'60 Forest'}));} finally {delete globalThis.localStorage;}
});
