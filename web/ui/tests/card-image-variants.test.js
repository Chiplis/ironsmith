import test from 'node:test';
import assert from 'node:assert/strict';
import { cardArtCropUrl } from '../src/lib/card-image-variants.js';

test('frame art follows the displayed printing, face, and image revision', () => {
  assert.equal(cardArtCropUrl('https://cards.scryfall.io/normal/back/a/b/selected.jpg?123'), 'https://cards.scryfall.io/art_crop/back/a/b/selected.jpg?123');
  assert.equal(cardArtCropUrl('https://cards.scryfall.io/png/front/a/b/selected.png?456'), 'https://cards.scryfall.io/art_crop/front/a/b/selected.jpg?456');
  assert.equal(cardArtCropUrl('https://cards.scryfall.io/art_crop/front/a/b/selected.jpg'), 'https://cards.scryfall.io/art_crop/front/a/b/selected.jpg');
});

test('custom and local card image URLs remain untouched', () => {
  for (const url of ['', 'data:image/svg+xml,custom', 'blob:https://example.org/image', '/cards/custom.png', 'https://example.org/normal/custom.png']) assert.equal(cardArtCropUrl(url), url);
});
