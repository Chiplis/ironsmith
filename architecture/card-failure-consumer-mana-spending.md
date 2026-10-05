# Consumer mana spending constraints

Status: **UNVALIDATED source implementation; no build, compilation, or test run.**

## Exact cohort and current accounting

`fixtures/consumer_mana_spending.json.fixture` preserves all ten frozen identities and complete programs. This first source proposal covers three:

- Imperiosaur: e9ced5d8-8337-403f-86a3-bddb9c77d658
- Myr Superion: 43dac6fa-4cdf-49ec-9e11-2e5e8f9f928b
- Security Rhox: dffc6785-31b0-487c-b9a0-980bca39753d

Atalya, Samite Master; Consume Spirit; Crimson Hellkite; Crypt Rats; Drain Life; Emblazoned Golem; and Soul Burn remain partial/not counted. Their X portion needs actual-color allocation distinct from fixed generic/taxes, including generic reductions and kicker's per-color cap. Drain Life and Soul Burn additionally need their capped actual-damage life-gain bodies. Replacing X with colored mana symbols is not valid: X is generic (CR 107.4b), generic reductions apply (118.7a), and as-though payment permissions do not change what was actually spent (609.4b).

## Typed boundary

`ManaProducerFilter` describes production-time permanent characteristics. `ManaSpendingRestriction::ProducedBy` is consumer-side evidence, independent of the existing producer-side `ManaUsageRestriction` and `ManaSpendPolicy` as-though permissions.

The restriction belongs to `ManaCost`, not a transient UI flag or a card-name check. A static spell rule is applied to whichever cost the prospective caster selects; Security Rhox's rule is attached only to its specified alternative. Taxes and additional mana inherit that rule. Zero remaining mana costs are valid without inventing a mana source.

All price rewrites retain constraints: compiler cost coalescing, spell optional/splice/modal costs, activation/reference cost totals, announced hybrid/X expansion, reductions, convoke/improvise residual costs, and the WASM cost-query combiner. The new static payload/ID are appended after existing variants. Legacy ManaCost JSON omits the empty field exactly as before and decodes without it. New typed artifacts and serialized payment requests carry the restriction. Unconstrained request/plan hashing keeps its old shape; constrained hashing includes a domain-separated restriction payload. Full request equality also protects resumable payment memoization.

The native pool qualifier and projected `ManaCredit` use the same production predicate. Untracked pool units fail a source requirement. Both projected source-equivalence classes and native search-state keys preserve the qualifying production evidence. Final payment replans against the authoritative constrained request, so a plan for the same pips without its rule is stale and cannot be committed.

## Production evidence

The mana event records checked current characteristics at production. If the source departed or phased out, exact-object departure LKI takes precedence over the older activation snapshot, with retained execution LKI as fallback. Already-produced mana keeps that snapshot across later animation, control/zone changes and source sacrifice. There is no stable-ID chase to a later incarnation.

An unqualified type/subtype description means a battlefield permanent (CR 109.2), so mana from a creature card's hand ability is not mana from a creature. An animated land can qualify for Myr Superion if it was a creature when it produced the mana. An extra mana trigger has its own producer (CR 106.3); it does not inherit the tapped basic land's qualification.

Primary references: the campaign's official 2026-09-25 Comprehensive Rules, CR 106.3, 109.2, 113.7a, 118.7a, 601.2f and 609.4b. The official [Time Spiral Remastered release notes](https://media.wizards.com/2021/downloads/TSR_Release_Notes/EN_MTGTSR_FAQ_20210118.pdf), page 44, specifically distinguish triggered bonus-mana sources, granted mana abilities on basic lands, and as-though permissions for Imperiosaur.

## Authored regressions

`consumer_mana_spending` has eleven unrun public compiler/runtime scenarios: exact strict/artifact programs, visible rules, wrong-source menu negatives, matching-colored source class separation, real casting/payment, sacrificed producers, production-time animation changes, ordinary versus Treasure-only alternative with taxes, stale-plan rejection without mutations, untracked/as-though negatives, legacy cost JSON, hand-card producer exclusion, and triggered-bonus exclusion. Core tests cover price transformation retention; serialization-feature engine tests cover request wire/root identity.

Deferred commands:

- `cargo test -p ironsmith-compiler-runtime --test consumer_mana_spending -- --nocapture`
- `cargo test -p ironsmith-core source_spending_rules_survive_price_transformations -- --nocapture`
- `cargo test -p ironsmith-engine --features serialization consumer_constraint_wire_tests -- --nocapture`

No execution is claimed. The seven X candidates remain open for a separate source change.

Assist's separate payer requests also inherit whole-spell producer restrictions, in both discovery and actual helper payment. An authored Imperiosaur-with-Assist regression supplies its caster's two qualified green mana and two helper mana: nonbasic helper sources cannot make casting legal; basic helper sources can pay the real contribution. The source constraint applies to all mana spent on the spell, regardless of which player supplies it.

The legacy potential-mana fast path and its symbol-only memo are bypassed whenever the cost carries a consumer constraint. Both cast-specific and generic source-aware affordability routes build the full authoritative request instead, before reducing the cost to bare pips.

### Incomplete production-characteristic queries

A tap/cost can make continuous discovery fail after the initial board was valid. Such a failure is now retained by the query's shared incomplete-calculation latch, distinct from `is_resource_exhaustion`, and the simulated activation preserves its original `ContinuousDiscovery` payload instead of becoming `NoLegalPlan`. The direct effect boundary rolls back incomplete calculations. Assist menu/response queries run inside the checked query owner, and payment-plan construction returns typed execution failure while retaining the announced proposal. Authored regressions cover native, sliced and checked queries; actual activation rollback; and an unaided-payable spell whose later Assist menu discovers an incomplete helper calculation.
