# Independent casting origins and prices

Status: **UNVALIDATED. No compilation, builds, or tests have been run.**
The six exact identities received bounded source clearance through `1e756c0e0`
and are proposed/unvalidated coverage. The 22 runtime scenarios and all auxiliary
tests remain unrun. The standalone shared phased-source correction is
`ddb7f363c57cfa9707d8c10facfde7c22752cda7`.

## Frozen cohort

`fixtures/independent_casting_prices.json.fixture` retains the full frozen texts,
printed characteristics, canonical identities and the separate Retrieve Prey face.

| Frozen identity | Oracle ID | Shared missing representation |
| --- | --- | --- |
| As Foretold | e4b3e0af-5caa-47c1-b5be-c56a875cbb15 | Per-source once-turn price, dynamic counter-bound mana value |
| Darksteel Monolith | fdd3482d-7bce-411b-80c4-956a12fc143d | Once-turn price for colorless spells from the caster's own hand |
| Tlincalli Hunter // Retrieve Prey | 79b69b38-c01d-499a-a865-8c1cecf9fae0 | Once-turn creature price restricted to an independently legal exile origin |
| Conspiracy Unraveler | e4541d02-2860-46ef-b630-1d6307b60766 | Real collect-evidence alternative price, with no new zone permission |
| Nissa, Worldsoul Speaker | 00037840-6089-42ec-8c5c-281f9f474504 | Eight-energy replacement price for permanent spells |
| Primal Prayers | 8d087fe0-d554-4d7c-ba22-32db2cf71887 | One-energy price and selected-route flash for small creature spells |

## Contract

- `Grantable::AlternativePrice` is appended after the old grant variants. It
  contains a conjunction of typed cost components plus an optional origin
  restriction. It cannot materialize a `PlayFrom` or ordinary alternative-zone
  grant. All filter, beneficiary, source, duration and use-limit checks remain.
- `CastingMethod::AlternativePrice` retains its underlying face/origin, the exact
  optional origin permission, and the independent exact price permission. Native
  identities survive the announcement. Public references use source and checked
  snapshot ordinal, and the existing command/head checks validate their complete
  canonical reference against current legal actions; native occurrence IDs are
  not emitted as portable public IDs.
- Discovery takes the product of legal origins and eligible prices *before*
  ordinary mana affordability or sorcery timing removes candidates. Both selected
  faces and fused split characteristics are installed in fallible read-only
  queries. Ordinary front-face and effect-owned price queries also establish a
  checked continuous snapshot; provider/on-use snapshots come from that same
  completed frame rather than legacy infallible discovery. A price cannot discover a card from an otherwise inaccessible zone.
- Native Adventure-exile and prepared-copy designations are independent origins
  too. The shared authority check retains the exact designated caster/controller
  and permits only the normal face; it never reopens the Adventure half.
- Prototype is an explicit price-route characteristic choice (CR 718.3), not a
  second replacement cost. Both origin and price filters query the selected
  prototype characteristics. The selected overlay is applied at proposal and
  captured in the receipt; priced casts do not offer a later optional prototype
  switch. Colorless-only and mana-value-bound prices therefore cannot be
  selected with one characteristic set and paid with another.
- An origin whose typed rule imposes mandatory additional costs retains those
  costs. Its mana surcharge is captured separately from the replacement price.
  Retrace and jump-start retain their own origin/additional-cost/departure rules.
  Flashback, escape, morph, and true alternative-cost origins cannot be combined
  with a second price. Ordinary additional costs, taxes, reductions, Assist and
  mana production/spending restrictions still use the established payment path.
- A resolving instruction can offer an independent price only through its
  existing private effect-casting authority and only when it has not already
  prescribed a competing alternative/free cost. This authority never becomes a
  priority permission or outlives the instruction.
- Each origin and price has its own once-turn identity and shared-use budget.
  Tentative shared-use reservations belong to the existing rollback checkpoint;
  completed casts mark both identities. Choosing the ordinary route leaves the
  price budget unused. Multiple on-use programs are retained in announcement
  order and each completes once, including when costs remove providers.
- `Object.cast_price` captures the cost, source, identity, origin mana surcharge
  and timing/cost constraints. Its historical provider snapshot is retained with
  the cast so occurrence-safe checkpoint encoding can bind a departed provider.
  The optional retained field is omitted for legacy/unpriced objects; a present
  receipt requires its complete payload. It is cleared on zone change.
- Printed mana-cost X is zero when the selected replacement lacks X (CR 107.3b).
  Other announced choices and actual payment use the captured price, rather than
  re-querying a provider after paying costs.
- Collect-evidence price payment emits the ordinary keyword action event. It
  does **not** pay a spell's distinct optional Evidence cost: CR 701.59c links
  “if evidence was collected” to that spell's own additional-cost ability. The
  Behind the Mask negative explicitly preserves that distinction.

The strict reader accepts the demonstrated once-turn, energy, collect-evidence
and cast-by forms. Unknown qualifiers or follow-ups are errors. A nested
alternative price branch is not silently flattened or treated as free. The
retained canonical surface is presentation only; runtime never reparses it.

## Authored regression scope

`crates/ironsmith-compiler-runtime/tests/independent_casting_prices.rs` covers
exact direct/artifact materialization, both Retrieve Prey and Hunter bodies,
source-relative counters, no new origins, both use identities, ordinary versus
alternative payment, own-hand and chosen-face filters, evidence ownership and
linked-cost separation, energy, chosen-route flash, taxes, mandatory origin
additional costs, provider sacrifice, cancellation/retry, X=0, stale action
rejection, jump-start discard/departure, command-zone source inactivity and
commander tax, real upkeep/ETB/landfall bodies, and private effect-authorized
casting, plus native Adventure/prepared origins and locked prototype choices.
Native retained-state and compiler/runtime codec tests cover receipt
payload completeness, old unpriced JSON, historical occurrence rebinding and
actual retained costs. WASM/UI action-reference tests cover both selected keys
and face distinctions through JSON.

Deferred commands, **not run**:

```
cargo test -p ironsmith-compiler-runtime --test independent_casting_prices -- --nocapture
cargo test -p ironsmith-compiler-runtime retained_cast_payment_codec -- --nocapture
cargo test -p ironsmith-engine retained_cast_payment_state -- --nocapture
cargo test -p ironsmith-engine ordinary_effect_price_discovery_propagates_incomplete_continuous_state -- --nocapture
cargo test -p ironsmith-wasm independent_price_action_reference_tests -- --nocapture
node --test web/ui/tests/sync-commands.test.js
```

This cohort does not close the separately documented seven constrained-X mana
allocation candidates or the intrinsic Worldheart Phoenix entry-cost linkage.
