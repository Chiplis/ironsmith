# Intrinsic graveyard alternative costs

Status: **UNVALIDATED**, source-only. No builds, compilation, or test execution.
Base afc092a55. Exact frozen identities are recorded in
`fixtures/intrinsic_zone_alternative_costs.json.fixture`.

Four proposed complete cards: Raffine's Guidance, Scourge of Nel Toth,
Squee, Dubious Monarch, Glimpse the Cosmos.

Worldheart Phoenix remains partial: its alternative method can be parsed by the
same price grammar, but its method-specific enters-with-two-counters rider needs
an actual entry-replacement binding to the chosen method. A generic
`cast from graveyard` predicate would incorrectly apply to other permissions.
The reader rejects that unrepresented rider rather than silently discarding it.

## Source contract

The reader emits the existing `AlternativeCastingMethod::FromZone`: exact
Graveyard origin, full typed `TotalCost`, optional cast condition, and the existing
graveyard-departure exile flag. It reuses the payment parser after changing only
cost-head gerunds to their imperative form, preserving spans, count/owner/other
filters, conjoined mana and nonmana costs, and alternative branches. A real
keyword-line owner prevents the sentence from becoming an optional resolving
payment effect.

No casting/payment implementation or wire enum is added. The selected method is
already retained on the stack object, conditions are checked against the actual
caster, and the actual cast pipeline pays the selected full price with ordinary
additional costs/taxes. Existing FromZone departure replacement excludes a
countered/fizzled spell too, while only replacing moves to the graveyard; unlike
flashback, a bounce may still return the spell to hand.

Typed rendering retains the origin, full price, condition and exile rider. An
unrecognized rider is an error; Worldheart is not counted.

## Authored verification

Deferred command:
`cargo test -p ironsmith-compiler-runtime --test intrinsic_zone_alternative_costs -- --nocapture`

The seven authored runtime scenarios cover strict full payloads/artifact round
trips, printed hand price versus graveyard price, real Aura targets and boost,
two-creature sacrifice payment, four other owned graveyard cards and source
exclusion, Squee's haste/attack token body, Giant controller condition and
announcement locking, Glimpse's actual choose/remainder body and exile versus
normal method, counter versus bounce, taxes and forbidden origins/owners.
Grammar scenarios guard complete compound costs and rejection of unknown riders
and unrelated static price grants. All remain unrun.

## Coordination boundary

Static price-only candidates were inspected but are outside this patch. Existing
filtered permissions and shared static budgets belong to the filtered-zone lane;
its `GrantSpec.additional_zones`, `top_card_only`, `instant_timing`, and shared
`GrantPermissionIdentity::Static` do not mean a price modifier can authorize new
zones. As Foretold/Conspiracy-style price changes must reuse an independently
legal cast, preserve alternative-cost exclusivity, and account usage only when
that price is chosen. No such coverage is claimed here.
