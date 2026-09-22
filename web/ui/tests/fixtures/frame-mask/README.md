# Text-mask regression corpus

Run from `web/ui`:

```sh
node --test tests/card-frame-font-mask.test.js tests/card-frame-source.test.js tests/card-frame-mask-corpus.browser.test.js
node --test tests/card-frame-mixed-ink.browser.test.js
```

The browser test runs offline against small, original text-region crops. `corpus.js`
records the Scryfall printing ID, language, frame version, and mask parameters.
The PNGs retain the scan's pixels, including connected lettering and frame edges.
Do not replace them with already-cleaned images.

`arnjlot-normal.jpg` and `arnjlot-art.jpg` are the normal and art-crop scans of
Scryfall printing `2307fb16-8b77-45b5-8a02-51a13214791d` (Ice Age, English).
The mixed-ink test checks white shadowed name/type lettering independently of
black rules, both scan and registered-field sampling, and removal of the full
labels including their initial letters.

Coverage includes Yawgmoth in Spanish and English, Brineborn Cutthroat in Spanish,
JPEG compression, and rendered classic white-on-gold, modern gold, and silver
labels at two resolutions. The Spanish Yawgmoth crop includes the connected
51-pixel-wide word fragment that the old component limit discarded.

Each successful mask is checked independently for remaining dark ink and changes
outside its mask. Frame-edge pixels are excluded from the ink count and covered
by explicit preservation tests. Brineborn's type line also covers dash recognition; its previously rejected
mask now removes the complete line.

The test writes `test-results/frame-mask/corpus.png` for visual inspection.
These tests cover representative cases, not every possible printing. Add new
scan crops when a failure reveals a new layout, font, or image-quality condition.

`bounding-krasis-normal.jpg` and `bounding-krasis-art.jpg` are the normal and
art-crop scans of Scryfall printing `0feca3b2-e822-4772-a644-782789f938cb`
(Magic Origins 212, English), image version `1782745490`. The ink-symbols browser
regression checks that pale mana discs cannot turn the black title white.
Run it with `node --test tests/card-frame-ink-symbols.browser.test.js`.
