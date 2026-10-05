# Participant-scoped life and damage turn totals

Status: **UNVALIDATED source proposal** for six exact full-card candidates. No builds, compilation, or tests were run.

Fixture: `fixtures/life_history_quantities.json.fixture`.

- Ancient Cellarspawn: inherited closure from the typed comparison-difference and intervening-if preservation in `bfea8a63`, now covered as a complete card. Its cast spell's actual mana spent is compared with the mana value of the cast object, including X and a prototype casting method, after departure from the stack.
- Astarion, the Decadent: the targeted opponent's life lost or the ability controller's life gained this turn.
- Children of Korlis: all actual life lost by its controller this turn, including payments; earlier gains do not subtract from that history.
- Simulacrum: damage actually received by the controller this turn, distinct from life lost and currently marked damage.
- Warlock Class and Wound Reflection: each opponent's individual life-loss total, not a sum over all opponents and not the controller's history.

All six frozen stack07 errors were missing life quantities. The proposed after-contract is full metadata-bearing strict compilation without loss, using existing executable `LifeLostThisTurn`, `LifeGainedThisTurn`, `DamageDealtToPlayersThisTurn`, and comparison arithmetic. Existing event/history producers, reset semantics and life executors remain in use.

## Parser and binding boundary

The shared history leaf accepts optional `the`, `total`, and `amount of`, contraction/expanded subject forms, and explicit `this turn`. `You` remains the effect controller. `They` / `that player` retain a relative participant until the existing subject or player-loop binder supplies the announced target or iterated player. An explicit `your opponents` total remains an aggregate over opponents. Damage received uses `dealt to`, never `dealt by` or life-loss totals.

The old life equal-to registry has two narrower turn-history aliases. They now defer only when the complete shared reader has proven the same history surface, avoiding duplicate readings and the older accidental new target for `that player`. Prior-effect life-loss/prevention readings and the all-player aggregate keep their routes. No loss diagnostics are suppressed, and there are no new serialized value variants or runtime placeholders.

## Authored, unrun evidence

- Normal tools integration aggregate: all six exact metadata-bearing payloads.
- Normal compiler-runtime integration target: seven tests, each running direct definitions and JSON-round-tripped materialized artifacts.
- Actual damage, prevention, infect, life payments, losses and gains create distinct histories (Alice loses 9 life, gains 10, receives 8 damage and ends at 21 life).
- Children pays its actual sacrifice cost, sees a response gain and resets across a public turn advance.
- Simulacrum gains and deals the damage total rather than either life-loss total or net change.
- Astarion's modes read their own participant after responses alter that player's history.
- Wound Reflection preserves the trigger controller across a later source-controller change, reads each opponent separately, and observes the next-turn reset.
- Warlock Class reaches levels 2 and 3 through actual paid activations and resolves its level-2 library choice before the level-3 history trigger.
- Ancient Cellarspawn casts discounted normal, X and prototype Demon spells, counters the compared spell, and loses its own source before its trigger resolves; an undiscounted Human spell must not trigger.
- Grammar tests cover exact metrics, participants, contractions, negative scopes and the overlapping legacy equal-to surfaces.

Other `gain`/`lose` diagnostics involving granted abilities, replacements, poison/radiation counter removal, shared coordinated amounts or linked earlier-entry totals remain separate. This family does not claim them.
