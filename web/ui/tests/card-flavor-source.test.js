import test from 'node:test';
import assert from 'node:assert/strict';
import {cardFlavorSource} from '../src/lib/card-flavor-source.js';

test('source-only flavor retains its full URL with a compact readable host',()=>{
  const href='https://www.reddit.com/r/custommagic/comments/ch910l/bob_ross_basic_lands/';
  assert.deepEqual(cardFlavorSource(href),{href,host:'reddit.com'});
  for(const value of ['',null,'A quiet shore.','See https://example.com for more','javascript:alert(1)','https://','https://user:pass@example.com'])assert.equal(cardFlavorSource(value),null);
});
