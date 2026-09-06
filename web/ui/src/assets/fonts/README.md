# Card typography

Fonts are bundled locally; no installed fonts or external font service are needed.
Selection follows the displayed printing's Scryfall `frame`, with `released_at` as
fallback. Recent retro-frame reprints therefore retain retro typography.

- Retro frames: Goudy Medieval titles, MPlantin type/rules/stats.
- 2003 frames: Matrix titles/type, Matrix Small Caps stats, MPlantin rules.
- 2015 frames: Beleren titles/type, Beleren Small Caps stats, MPlantin rules.
- Future frames retain the earlier Matrix approximation.
- Missing metadata defaults to Beleren titles and MPlantin rules. Showcase and
  other custom typography may differ from these conventional-frame mappings.

MPlantin includes a real italic face for flavor text and a semibold face for
emphasis. Font loading triggers inspector text remeasurement.

`Beleren2016-Bold.woff` comes from
[Saeris/typeface-beleren-bold](https://github.com/Saeris/typeface-beleren-bold).
The upstream project attributes it to Wizards of the Coast and describes
non-commercial distribution. The restored Goudy, Matrix, small-caps and Plantin
companion assets are the copies used in the earlier typography build, originally
sourced from [Card Conjurer](https://github.com/Investigamer/cardconjurer).
`MPlantin.ttf` retains the repaired user-supplied copy already in this repository.
