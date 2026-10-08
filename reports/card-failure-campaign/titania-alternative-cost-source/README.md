# Titania alternative-cost panic: source-stage candidate

Status: **UNVALIDATED source repair; no recovery or whole-body runtime pass is claimed.**

Base: `ad0b0056c` (integrated Oct 8 stack). Baseline measurement remains the exact
`1dd81cd` refresh. Titania, Rugged Rumbler, oracle ID
`e380e37d-926b-4a4b-a275-7844bf4956d5`, remains the measured compiler-panic entry.
The compiler, tests, corpus, probes, generators, and formatters were not run.
No remote publication was performed.

## Source trace of the panic

The retained refresh record reports `TotalCost::costs called for an alternative
cost`, with an empty route-diagnostics array (there is no captured backtrace).
This is a source call-path diagnosis, not a newly executed reproduction:

1. `ironsmith-tools/src/tooling.rs::parse_card_payload` catches panics around
   `definition_from_payload`. That calls the registry's
   `compile_builder_to_runtime_definition` with the metadata-bearing parse input.
2. `keyword_static/ward_cost_readings.rs::read_payment_clause` calls
   `parse_payment_clause_as_total_cost`. Its `find_payment_alternative_or` branch
   retains `OneOf([discard a card, pay {2}])`; it does not concatenate the prices.
3. Lowering/runtime conversion retain that graph. In compiler-runtime,
   `into_runtime_definition` calls `attach_rendered_presentation`, which calls
   `ironsmith_text::compiled_text_lines` and ability surface rendering. Static
   ability rendering calls the runtime static ability's `display` implementation.
4. `ironsmith-engine/src/static_abilities/protection.rs::Ward::display` called
   `self.cost.costs()` unconditionally while looking for a waterbend mana cost.
   `ironsmith-core/src/cost_model.rs::TotalCost::costs` requires `All` and panics
   on the parsed `OneOf`. The same offending line is present in exact `1dd81cd`.

This explains the observed panic without blaming the independent casting-cost
choice, weakening a rejection, or claiming that a compiler execution succeeded.

## Repair scope and preexisting execution owners

The production change only makes the waterbend display predicate recurse over
`All` and arbitrarily nested `OneOf`. It inspects components without selecting,
flattening, appending, or replacing any payment branch. Existing separator and
waterbend presentation behavior is retained. It is independent of card names,
Oracle IDs, and the particular discard/mana alternatives.

Source inspection found the support needed for this particular complete body:

- The additional casting cost goes through `parse_additional_cost_choice` into
  `LineAst::AdditionalCostChoice`, then
  `materialize_additional_cost_choice` creates a validated `ChooseModeEffect`.
- `costs::simple_modal_mana_cost_branches` recognizes this exact single-effect
  discard versus mana form. `priority_cast::announce_modal_mana_costs` requires
  exactly one selection and contributes the chosen mana to the spell's total;
  it replaces the modal payment with only the selected nonmana components.
  `decision::mana::modal_additional_costs_are_payable` includes printed mana and
  excludes the cast card from its own discard payment. This avoids independently
  funding the {2} branch after spending the printed {2}{B/G}.
- Ward stays `TotalCost::OneOf`. `targeting/ward.rs::WardCounterEffect` finds the
  controller of the targeted stack object as payer and retains the ward
  permanent as the source of cost references. Its resolution payment gateway
  handles alternatives and checked transactional payment. The unchosen branch
  is not paid. Declined/failed ward counters the targeting object normally.
- Core `TotalCost` has recursive serde and `try_map`; compiler-runtime's total
  cost mapping preserves those branches. Ward's retained typed model and runtime
  artifact materialization retain the graph; modal payment effects use existing
  effect codecs. No new wire payload, price variant, or choice protocol is added.

These existing owners justify a narrow display repair rather than introducing
an artificial fail-closed compiler rejection for already represented choices.
Their actual end-to-end behavior still requires the unrun regression suite.
Other unguarded `costs()` callers elsewhere are not globally rewritten; this
patch is not a claim of universal composite-cost support.

## Authored coverage (NOT RUN)

`crates/ironsmith-compiler-runtime/tests/titania_alternative_cost.rs` uses the
complete official record's raw Oracle body, {2}{B/G} cost, legendary Human
Villain type, and 5/5 stats. Both direct compilation and serialized/deserialized
artifact materialization feed each runtime case. The authored assertions cover:

- strict compilation with no recorded parse loss; metadata, canonical text,
  modal cast owner and disjunctive ward owner retained separately;
- cast via discard and mana, exactly one announcement, printed hybrid cost plus
  only the selected additional price, and no ward payment on casting itself;
- unpayable cast absent from legal actions; invalid two-branch announcement and
  cancellation of each branch leave hand, graveyard, stack and mana unchanged;
- actual targeting spell and real ward trigger, each ward price independently,
  exactly one payment, correct opposing payer, unchosen resources retained;
- ward decline, neither branch payable, and mana-payment cancellation counter
  the targeting spell without taking either price;
- each price succeeds when the unchosen price is unavailable, for casting and
  ward, including the protected controller's resources staying untouched.

The engine unit test also covers ordinary mana ward, a mixed disjunction, nested
alternatives with waterbend, and retained-model restoration without changing the
cost tree. The preexisting nested retained-Ward display regression also exercises
the same repaired call.

No assertion has been verified by compilation or test execution. Failures found
when execution is authorized must remain blockers, not be converted into a
success label or counted recovery.

## Fixture authentication and checks performed

`fixtures/titania_alternative_cost.json.fixture` retains the entire exact official
card object, not an invented record. Dataset SHA-256:
`bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750`.

The offline authentication script compares that digest, finds the unique Oracle
ID, compares the whole retained record, and reconstructs the metadata/body input.
It was executed against the refreshed `data/cards-current.json` and passed.
This is data/byte authentication only; it does not load the compiler or execute
any card. `git diff --check` passed. Source paths and APIs were manually inspected.
No AGENTS.md or .agents/skills instructions were present in this checkout or the
workspace instruction locations inspected.

## Compatibility and follow-through

The inherited published artifact-format **15** and audit-semantics **29** boundary
and all previous descriptors remain unchanged. This patch adds no serialized
fields, discriminants, or runtime decision schema. Existing compatible typed
Ward artifacts now have a safe display path; newly compiled canonical text and
support results may change from an earlier panic. A subsequent coordinated
release must retain the new source identity in its compiler/catalog and audit
cache keys, invalidate stale Titania panic/support results, and decide any new
release descriptor/version under that release's compatibility policy. Do not
rewrite an already published or prior descriptor to absorb this source change.

Before promotion: compile and run the authored engine and compiler-runtime
regressions on the final integrated stack, then the approved exact-current-ID
and artifact/runtime audit. Authenticate the fixture again if the dataset
changes. Only those future results can establish recovered compilation or
whole-body/runtime support. The old measured counts and diagnostic remain
immutable until an explicitly new measurement replaces them.

## Independent-review coverage follow-up (source-only, UNRUN)

The original repair commit `5d87bb62f7292070dcaea89f88cd573807bcad93` is preserved.
The independent source review found the production display change source-clear
and independently authenticated the fixture, but requested additional full-card
scenarios before source admission. A follow-up adds only authored test coverage:

- Both additional cast choices now run with black-only and green-only funding,
  explicitly selecting the matching printed hybrid pip through the public
  `HybridChoice` response. Exact remaining mana, hand, graveyard and single-branch
  assertions are retained.
- Blue-only funding is rejected from legal cast actions, both with and without
  a discard resource, and leaves mana, hand, graveyard and stack unchanged.
- If Titania leaves before target announcement, the targeting spell has no legal
  creature target or legal cast action. No ward/payment prompts or queued trigger
  are produced, and all payer resources stay unchanged.
- If Titania leaves after its real ward trigger is stacked, that obligation
  still charges the targeting controller exactly one selected price. The spell
  remains on the stack after ward payment, then loses its effect because its
  original target is gone. Returning the same physical card before spell
  resolution supplies a fresh untapped identity which must not be retargeted.

The existing cancellation cases are **immediate cancellation at the first real
mana-payment prompt**, with the enumerated resource-restoration assertions.
They do not claim rollback after a spent activation prefix, nor full queued
output/receipt rollback. Those inherited-payment edge cases and choosing an
unaffordable price despite another payable price remain advisory future coverage.

No production behavior, wire schema, published boundary descriptor, or fixture
changed in this follow-up. The separate release coordinator owns the subsequent
boundary identity. All compiler/test/runtime gates remain UNRUN, and measured
support/recovery counts remain unchanged. `git diff --check` is the only executed
follow-up check; this is not a compiler or runtime validation.
