# Deferred Parser/Lowering Cases

This worktree only fixes parser/lowering gaps that already fit existing shuffle and move-zone semantics.

## Still Deferred

- `put ... onto the battlefield tapped and attacking`
  - Representative cards: `Ilharg, the Raze-Boar`, `Winota, Joiner of Forces`, `Arni Metalbrow`
  - Reason: current lowering paths (`MoveToZoneEffect`, `PutOntoBattlefieldEffect`) support controller overrides and tapped entry, but not generic non-token "enters attacking" behavior.

- `put ... onto the battlefield attached to ...`
  - Representative card: `Danitha, Benalia's Hope`
  - Reason: this needs attachment-aware battlefield entry lowering for the exact "attached to" form seen on the card, which is outside the shuffle / move-zone / simple put-onto-battlefield scope requested here.

- reveal/replacement-mechanics follow-ons that would need new execution support
  - Examples: any future fix that depends on introducing new reveal-state tracking or new replacement-effect execution rather than reusing the existing shuffle / move-zone / put-onto-battlefield semantics.
