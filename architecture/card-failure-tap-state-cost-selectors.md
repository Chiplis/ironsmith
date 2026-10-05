# Written tap/untap cost selectors

Status: **UNVALIDATED** source implementation and authored regressions. No
compilation, build, or test execution was performed.

## Frozen identities and bounded proposed coverage

`fixtures/tap_state_cost_selectors.json.fixture` preserves the nine complete
frozen source inputs and Oracle IDs from `stack07-bc9e56e2`.

Seven complete-card proposals:

- Benthic Explorers
- Crackleburr
- Halo Fountain
- Earthlore
- Nature's Chosen
- Tourach's Gate
- Krovikan Plague

Two additional source closures now have authored, unrun gameplay regressions
in `prospective_cost_references`:

- Fishing Pole retains the specific granted ability's grantor context through
  both availability and actual chosen-object cost payment.
- Veteran's Voice binds its deterministic enchanted-creature cost identity
  before target announcement, then revalidates it when paying.

See `card-failure-prospective-cost-references.md`. All nine are source proposals;
none of these changes has received the deferred build/test/replay validation.

## Typed paths

- New compiler-only `UntapChosen { count: ChoiceCount, filter: ObjectFilter }`
  passes through grammar, semantic assembly, and lowering.
- The existing `TapChosen` now accepts explicit attachment/source operands
  without requiring the printed adjective `untapped`. Payment still requires
  an untapped permanent. The untap counterpart requires a tapped permanent.
- Lowering uses the existing typed object choice followed by a tagged TapEffect
  or UntapEffect. No serialized runtime payload changed.
- Attachment filters bind to the source's actual host, not any enchanted object.
  Explicit attachment identity does not gain an invented payer-control filter.
- The parser's separate `untap_cost_N` identity is shared with lowering and
  subsequent effect reference imports. Benthic Explorers' mana uses the land
  chosen for the untap cost.
- Named attachment objects within granted ability tap/untap costs retain the
  existing granting-source tag vocabulary instead of becoming the host source.
- Payability recognizes untap choice/consumer pairs. {Q} reserves the source's
  tapped state, symmetrical with the existing {T} reservation. Dynamic-X bounds
  retain the same distinction.
- Tagged tap-state cost validation requires live current-object membership,
  correct tap state, and legal untapping; unbound tags fail. It is side-effect
  free and does not impose symbol-only summoning-sickness rules.
- Existing TapEffect/UntapEffect executors retain payer context and inherit the
  concurrently implemented tap-state actor/snapshot/per-instruction event batch
  support. This branch does not edit those event producers.
- Cost rendering compacts chosen untap pairs and preserves dynamic-X amounts.

## Authored deferred regressions

`tap_state_cost_selectors` exercises direct definitions and typed artifact JSON:
Benthic's opponent land and mana antecedent, Crackleburr's {Q} reservation and two
blue-creature payment, Halo Fountain's exact fifteen count and cancellation
rollback before the win effect, Nature's Chosen's source attachment, written-tap
summoning sickness distinction, once-per-turn cap and resolution after Aura
removal, and Krovikan Plague’s damage / -0/-1 counter rider on the same host. Negative cost preflight assertions preserve life, mana and tap state,
reject missing tags, and reject a later zone incarnation of a selected object.
Grammar regressions retain exact counts, ownership/control filters and attachment
identity while rejecting incomplete operands.

Deferred commands:

`cargo test -p ironsmith-compiler-grammar --lib chosen_untap_and_attachment_tap`

`cargo test -p ironsmith-compiler-runtime --test tap_state_cost_selectors`
