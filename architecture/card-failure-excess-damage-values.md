# Excess-damage quantities and result bindings

Status: **UNVALIDATED implementation-first source proposal**. No compiler, build,
or test was executed. The fixture retains all seven exact frozen metadata-bearing
inputs and their stack07 diagnostics. Proposed full-card coverage is seven,
subject to deferred aggregate validation:

- Bolg of the North
- Contest of Claws
- Fall of Cair Andros
- Violent Echoes
- Goblin Negotiation
- Hell to Pay
- Windswift Slice

The first four failed on unsupported where-X clauses. The last three failed on
unsupported dynamic token counts. This is a shared quantity/reference family,
not seven card-name routes. Violent Echoes also depends on the already integrated
Empower Jace action; this patch closes that previously recorded partial proposal.

## Typed boundary

The engine already emits both `ExecutionFact::ExcessDamageDealt` and the numeric
`ExecutionFact::ExcessDamage(n)` after actual damage processing, including damage
prevention and the target's marked damage, deathtouch threshold, and loyalty.
`EffectMetric::ExcessDamage` reads those numeric facts rather than substituting
full damage or calculating a post-resolution toughness difference. Its result
condition is the existing executable `EffectPredicate::ExcessDamageDealt`.
No serialized variant, runtime marker, or new numeric evaluator is necessary.

The shared value grammar distinguishes two references:

1. Explicit “the amount of excess damage dealt ... this way” is an existing
   `PendingPriorEffectMetric` query with action `DealtDamage`. Producer selection
   skips unrelated actions. Once bound, an unfiltered numeric excess query lowers
   to the existing `EffectMetric` form so existing runtime and renderer paths stay
   intact.
2. Bare “that excess damage” remains a pending excess metric. Only the typed
   excess-damage trigger capability may bind it to ambient `EventValue(Amount)`.
   Ordinary life gain/damage triggers cannot supply that quantity. A local excess
   result branch pins its own damage producer and takes precedence over ambient
   trigger context. An unrelated body action does not steal the trigger amount.

The capability is compiler-only and copied through the existing reference frames,
branch environments, and lowering contexts. Delayed triggers derive it from their
own event. Combined triggers require both alternatives to expose excess. Neither
loss diagnostics nor metadata fallback behavior are suppressed.

The result-condition parser also accepts the exact bare “excess damage was dealt
this way” and permanent-target wording used by the frozen inputs. Result producer
selection requires a damage action.

## Authored verification

- Grammar tests distinguish explicit prior-damage queries from ambient-capable
  excess references, retain the existing exile quantity grammar, and reject
  non-excess/incorrect-time result near misses.
- Reference tests cover absent producers, ordinary event capabilities, explicit
  producer identity, branch pinning, and event amounts after unrelated actions.
- Semantic helper tests ensure life and ordinary-damage triggers do not acquire
  the excess capability, including mixed alternatives.
- `ironsmith-tools/tests/excess_damage_values.rs` checks all seven strict payloads
  without metadata fallback or loss flags.
- `ironsmith-compiler-runtime/tests/excess_damage_values.rs` checks every exact
  card through direct compilation and artifact JSON/materialization, with typed
  amounts on actual Amass, Discover, Empower, and token instructions.
- Live authored scenarios use the public casting/decision/payment flow for X
  spells, Windswift Slice, Contest, Violent Echoes, and Bolg. They distinguish
  actual excess from full damage under prior marked damage, partial/total
  prevention, exact lethal damage, deathtouch, and planeswalker loyalty; verify
  tapped Treasures and creature token types; preserve Bolg's sacrificed-power
  reflexive binding; and ensure zero excess skips Empower rather than creating a
  zero-loyalty token.
- Fall scenarios deliver real damage outcome events through trigger matching,
  check opposing-controller/noncombat/positive-excess scopes, then resolve after
  the enchantment leaves. Additional probes distinguish ambient excess after an
  unrelated action from local explicit/conditional damage outcomes.

These tests are authored, not passing claims. The remaining where-X families and
Mathemagics are outside this change.
