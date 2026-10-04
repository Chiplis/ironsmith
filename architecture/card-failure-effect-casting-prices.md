# Effect-authorized replacement prices (source draft, UNVALIDATED)

Frozen input: stack07 at bc9e56e2, exact identities in
`fixtures/effect_granted_casting_prices.json.fixture`. No build, compilation,
test or runtime/card probe has been executed for this draft.

## Shared executable owners

- `CastTaggedEffect<C>` and `GrantPlayTaggedEffect<C>` retain an optional typed
  `TotalCost<C>`. Absence is omitted in serialized payloads; old payloads read
  with no substituted price. Wire registry/materializers map every nested cost.
  Native traversal visits nested payment effects and the native cast encoder
  projects the costs rather than relying on retained JSON.
- A resolving instruction's price is mandatory if its optional cast is accepted.
  It prevents choosing an independent second alternative. Printed additional
  costs, optional costs, taxes/reductions and cost order stay with the common
  CR 601 transaction. The effect-specific pending cost is replay-local: pending
  input reverses that attempt and the owning resolving instruction is replayed.
- Mana components are aggregated once before the normal modifiers. Nonmana
  components join the ordinary cost steps. Life/energy values referring to the
  selected spell are fixed after its characteristic/target/X announcements.
  The old energy-only cast route now pays through that same owner instead of
  removing counters after a completed cast.
- A fixed replacement price gives a printed mana-cost X the value zero. A price
  with X still uses the X announcement and actual payment owner. Dynamic-mana
  cost components need their prospective-cost announcement and currently return
  an explicit unsupported-state error, never a free or partial payment. No
  fixture in the proposed cohort depends on that unsupported composition.
- A priced temporary tagged permission emits spell-only alternative grants and,
  if permitted, land-only ordinary grants. Both use one collection budget.
  The cast proposal now recovers a selected alternative's budget from its exact
  stored permission identity before the legacy plain-grant fallback.
- Priced tagged instructions preserve exact current incarnations. They never
  chase a departed tagged object through its stable ID. Ordinary unpriced legacy
  permissions retain their existing behavior in this bounded patch.
- Dream Halls uses the existing independent origin-plus-price route. Its cost
  selects another card from the payer's hand and compares its current colors
  to the proposed/stack spell through a typed source characteristic relation.
  Both players are beneficiaries; it grants no zone or timing permission.
- A temporary filtered top-of-library origin can carry the same life price.
  The current top restriction/private top view and duration remain on the
  grants. Its ordinary spell route is replaced; its land route remains valid.

## Full-body scope

Nine candidates are independently source-reviewed: Bismuth Mindrender, The Infamous
Cruelclaw, Dream Halls, Gwenom, Remorseless, Inside Information, Nashi, Moon
Sage's Scion, Xander's Pact, Blue Mage's Cane, and both faces of Jadzi // Journey. This is proposed source coverage only.

Two identities remain partial: Anrakyr's immediate hand/graveyard choice and Wizard's Spellbook's multiple prepared copies/any-number chosen casting order. Their price phrases alone do not count.

## Authored regressions

`crates/ironsmith-compiler-runtime/tests/effect_granted_casting_prices.rs`
contains exact source/artifact pairs, real effect-driven and priority casts,
mandatory/independent price separation, life payment and failed-payment rollback,
modifier/printed-X behavior, actual combat triggers, source departure, exact
incarnations, shared land/spell budgets, payer ownership and native cost codecs.
All remain unrun.

Deferred command:
`cargo test -p ironsmith-compiler-runtime --test effect_granted_casting_prices -- --nocapture`

Worldheart Phoenix's method-specific entry-counter rider is independently source-reviewed in `card-failure-method-entry-counters.md`. All nine price bodies use individual casting transactions; the generic simultaneous compound-Instead payment hold documented for Font is outside these paths. The coupled stage53 source review retains the checked direct payment meter/context, exact budget identity and graveyard recipient riders.
