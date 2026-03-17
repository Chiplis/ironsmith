# Deferred Parser/Lowering Cases

These parser-wave changes intentionally stay within existing lowering and runtime semantics.

## Still Deferred

- `put ... onto the battlefield tapped and attacking`
  - Representative cards: `Ilharg, the Raze-Boar`, `Winota, Joiner of Forces`, `Arni Metalbrow`
  - Reason: current lowering paths (`MoveToZoneEffect`, `PutOntoBattlefieldEffect`) support controller overrides and tapped entry, but not generic non-token "enters attacking" behavior.

- `put ... onto the battlefield attached to ...`
  - Representative card: `Danitha, Benalia's Hope`
  - Reason: this needs attachment-aware battlefield entry lowering for the exact "attached to" form seen on the card, which is outside the shuffle / move-zone / simple put-onto-battlefield scope requested here.

- reveal/replacement-mechanics follow-ons that would need new execution support
  - Examples: any future fix that depends on introducing new reveal-state tracking or new replacement-effect execution rather than reusing the existing shuffle / move-zone / put-onto-battlefield semantics.

- Generic `where X is ...` follow-up work that would need broader value-grammar expansion beyond the parser-only tails handled here.

- Exact non-target single-opponent chooser support for clauses like `look at an opponent's hand` in multiplayer, which would need dedicated non-target player-choice lowering instead of the existing broad opponent filter semantics.

- New runtime/value mechanics for exact "cards you've drawn this turn" counts, including `Fists of Flame` style `gets +1/+0 for each card you've drawn this turn`.

- New runtime/value mechanics for counting distinct mana values among cards in graveyards, including `All-Seeing Arbiter` style `where X is the number of different mana values among cards in your graveyard`.
