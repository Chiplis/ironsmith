# X-count chosen tap activation costs

Status: **UNVALIDATED**. Implementation and authored regression coverage only.
Per the implementation-first campaign workflow, no build, compilation, or test
execution was performed for this change.

## Frozen candidate identities

The `stack07-bc9e56e2` full-corpus snapshot contains eight unsupported records
whose root diagnostic is `rewrite tap chosen parser does not yet support` for
`Tap X untapped … you control`. The exact card names, Oracle IDs, and full frozen
parse inputs are retained in `fixtures/x_tap_costs.json.fixture`:

- Aryel, Knight of Windgrace
- Belisarius Cawl
- Glacian, Powerstone Engineer
- Hazel of the Rootbloom
- Merchant's Dockhand
- Necron Overlord
- Resonance Technician
- Secluded Starforge

These are eight proposed recoveries, not eight measured successful compiles.
Whole-card residual failures remain possible until the deferred validation pass.

## Typed correction

`TapChosen` had a fixed `u32` count across the recognition CST, semantic cost
algebra, and lowering. This change uses the existing `ChoiceCount` algebra for
that one cost kind. The grammar recognizes an exact variable X count and retains
the ordinary object filter. Lowering still emits the established
`ChooseObjectsEffect` followed by a tagged `TapEffect`.

The modified compiler-only count representations are not serialized transport
schemas. Fixed-count costs produce the same `ChoiceCount::exactly(N)` and the
same runtime effects as before. The typed artifact schema, runtime constructors,
and fixed-count display remain unchanged; X uses an already-supported runtime
count representation.

The generic X-bound calculation excludes the source from an untapped-object
choice when another component pays `{T}`. This is a query-only narrowed copy of
the filter, not an alteration of the printed filter or final paid objects. It
prevents an artifact source such as Necron Overlord from counting twice.

## Authored validation (not run)

- Grammar tests preserve exact X, subtype/type/controller/untapped filters,
  existing fixed/another counts, and fail-closed malformed variants.
- Public compiler/runtime integration tests strict-compile all eight complete
  cards and materialize a JSON artifact round trip, including Belisarius's
  fixed-count sibling.
- Necron Overlord activates through real priority, X announcement, target
  selection, chosen tapping, and mana payment at X=0/1/2. Its bound accounts for
  resources and mana; wrong type, opponent ownership/control, already-tapped
  cards, and its separately tapped source cannot pay.
- The Necron ability is resolved after its source leaves and paid permanents
  untap, requiring the announced X rather than a recomputed current tap count.
- Secluded Starforge checks a nonmana X with a fixed mana cost, correct power-only
  amount, and end-of-turn expiration.
- Actual mana-payment cancellation must restore all taps, mana, action legality,
  and an empty stack.

Deferred commands:

`cargo test -p ironsmith-compiler-grammar --lib tap_x_untapped_costs`

`cargo test -p ironsmith-compiler-grammar --lib rewrite_activation_cost_token_entrypoint_parses_tap_return_and_exile_variants`

`cargo test -p ironsmith-compiler-runtime --test x_tap_costs`
