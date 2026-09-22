# Font estimation fixtures

`weapons-manufacturing-eoe-168.jpg` is the 488 × 680 Scryfall normal scan of
Weapons Manufacturing (EOE 168), printing `a058f1a6-318c-4bba-981e-ace079ada806`.
Source: https://cards.scryfall.io/normal/front/a/0/a058f1a6-318c-4bba-981e-ace079ada806.jpg

The first two rules lines have faint ink and weak overlap with MPlantin. Their
independent width estimates agree at about 20.6px, whereas the first line's
height alone gives 16.84px. This local fixture keeps the regression independent
of network availability and future scan changes.
