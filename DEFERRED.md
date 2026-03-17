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

- Distinct-color mana output remains deferred for `Bloom Tender` and `Faeburrow Elder`. Exact support for "For each color among permanents you control, add one mana of that color" needs new sentence-level color iteration or a dedicated mana effect, which is outside this parser/lowering-only pass.

- Spent-to-cast provenance work remains deferred, including "colors of mana spent to cast" style tracking.

- Keyword-cost work remains deferred, including new lowering for keyword-specific payment machinery.

- Generic X-expression work remains deferred where existing value parsing cannot lower the expression without broader support.

- Any new mechanic or effect work remains deferred in this pass.

- Fireball-style divided damage and other distributed-target damage phrases remain deferred.

- Coin-flip or random-choice pseudo-targets such as `coin` remain deferred.

- `"the rest"` / pile-splitting / Balance-style restructuring follow-ups remain deferred.

- Player-iteration subject phrases like `each other player` that currently route through non-owned player/for-each parsing paths remain deferred.

- Player-subject restriction clauses with leading condition/duration prefixes, including representative failures like `Island Sanctuary`, `Teferi's Protection`, and `Grand Abolisher`, remain deferred.

- Any new mechanic/effect work beyond existing `ObjectFilter` / `TargetAst` / restriction lowering support remains deferred.

- Per-player untap caps such as `Players can't untap more than one creature during their untap steps` remain deferred. These need a new restriction/tracker model instead of the existing binary `can't untap` restriction.

- Mana-retention clauses such as `You don't lose this mana as steps and phases end` remain deferred. These need explicit mana-pool lifetime semantics, not just parsing.

- Cast restrictions on a specific named card or card subset gated by a condition, such as `You can't cast Rakdos unless an opponent lost life this turn`, remain deferred. These need a way to restrict casting a specific object/card identity, not just broad player/spell filters.

- Clauses that stop players from both casting spells and activating abilities when the ability stop must include mana abilities, such as `That player can't cast spells or activate abilities`, remain deferred. Existing player-based restriction support only covers non-mana activated abilities.

- Modal headers like `Choose any number —` on cards such as `Rankle, Master of Pranks` remain deferred. Existing choose-mode execution is already present, but the remaining parser work for that header shape lives outside the owned files for this task.
