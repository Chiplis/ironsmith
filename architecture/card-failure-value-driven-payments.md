# Value-driven activation payments

Status: **UNVALIDATED**. Source review and authored regressions only. No compilation or test execution under the implementation-first campaign workflow.

Baseline: stack07 `bc9e56e2`; exact oracle identities and complete frozen programs are in `fixtures/value_driven_payments.json.fixture`.

## Proposed scope

Seven complete-card proposals: Chthonian Nightmare, HELIOS One, Sphinx of the Revelation, Krumar Initiate, Lurking Evil, Murderous Betrayal, and Tornado.

Skeleton Scavengers is a separate partial. Its printed per-counter mana payment is represented, but “When it regenerates this way” needs a delayed triggered ability with normal stack timing. The existing regeneration replacement's immediate follow-up effects are insufficient; this work does not relabel that card complete.

## Typed representation

The activation CST admits X energy and life, explicitly rounded half-life, and fully consumed typed for-each mana/life quantities. These lower to the existing PayEnergyEffect, PayLifeEffect, DynamicManaCost and Value families. Existing fixed energy and hand-count forms retain their old payload shapes. There is no new runtime enum variant or serialization shape.

PayEnergyEffect and PayLifeEffect expose the existing cost-X hooks. Preannouncement resource checks use the legal minimum X=0; actual payment still requires the announced X. X bounds use payer energy/life alongside existing mana bounds, and zero energy payment succeeds without manufacturing a counter-removal event.

State-dependent activation costs are determined before increases/reductions and payment (CR 601.2f, 602.2b). Resolvable dynamic mana is materialized before merging mana components and applying whole-total modifiers. Direct dynamic life-payment amounts are likewise frozen in the temporary total cost. Unbound X and unresolved references are retained and continue to fail closed at the context-aware payment stage. Conditional source-mana reduction choices remain on their original decision path. No source ability or serialized card definition is mutated.

## Authored regression coverage

- Strict exact-card/artifact round trips for the seven proposals.
- Real X-energy payment, X=0, cancellation rollback, source departure, and announced-X resolution.
- HELIOS target mana value/type legality and source sacrifice before resolution.
- Chthonian sacrifice, payer energy, target announcement before sacrifice, and source owner/controller separation.
- Krumar joint mana/life X bounds and the enduring amount.
- Half-life rounding and cost locking across a later change to life total.
- Murderous Betrayal against a real regeneration shield.
- Tornado velocity versus age counters, zero multiplier, and once-per-turn usage.
- Isolated printed Scavengers cost syntax, additive mana-component merging, Training Grounds reduction, and a fixed determined total.
- Nonmutating insufficient-X preflight and failed payment; complete-consumption grammar rejection controls.

Deferred command after the compilation gate opens:

`cargo test -p ironsmith-compiler-runtime --test value_driven_payments -- --nocapture`
