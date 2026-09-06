import test from 'node:test';
import assert from 'node:assert/strict';
import { cardTypography } from '../src/lib/card-typography.js';
test('printing frame takes precedence over release date for retro reprints', () => {
  const retro = cardTypography({frame:'1997', released_at:'2023-01-13'});
  assert.equal(retro.era, 'retro');
  assert.match(retro.title, /Goudy/);
  assert.match(retro.rules, /MPlantin/);
  assert.match(cardTypography({frame:'2003'}).title, /Matrix/);
  assert.match(cardTypography({frame:'2015'}).title, /Beleren/);
});
test('missing frame falls back to printing date, then modern typography', () => {
  assert.equal(cardTypography({released_at:'2001-01-01'}).era, 'retro');
  assert.equal(cardTypography({released_at:'2010-01-01'}).era, 'modern');
  assert.equal(cardTypography({released_at:'2024-01-01'}).era, 'beleren');
  assert.equal(cardTypography().era, 'beleren');
});
