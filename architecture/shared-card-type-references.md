# Restored shared card-type references

This family was reconstructed from retained exact edit history on recovery
base `42066a7b336bf2a6c7e0a2f93a08bc26001c740e` (the tree of remote
`91fd9889`), incorporating current main `d51f4ee69b500462c8d70354cec324bf6d04677c`.
All builds, compiler probes, tests, formatters, and corpus execution remain
**UNRUN**. Fresh independent review clears three complete bodies through
`79debf90a`; combined integration review at `6afb57377` retains those owners.
The campaign ledger now counts these as source proposals. Four remain partial.

The seven complete frozen bodies and printing/oracle IDs were extracted from
`fixtures/card-failure-campaign/cards-20261003.json.xz` into
`fixtures/shared_card_type_references.json.fixture`, without oracle edits.

## Whole-body restoration candidates

- Cemetery Gatekeeper: First strike; actual non-targeted graveyard-card exile;
  exact linked-exile membership; every player's land-play or cast notice;
  intervening type checks and two damage to the original event actor.
- Cemetery Protector: Flash; the same exile owner; only its controller's
  land-play/cast notices; a white 1/1 Human per qualifying notice.
- Amareth, the Lustrous: Flying; another controlled permanent entering; private
  top-card observation; type comparison against that exact entering permanent;
  one optional public reveal and hand move of only the matching observed card.

The complete land/cast grammar uses existing typed `Either` triggers. Sharing
conditions preserve separate subject and comparison tags. Amareth's comparison
noun is appended, serde-defaulted presentation metadata; runtime meaning stays
in the relation and reference prelude, and the structural renderer uses those
fields rather than oracle-string substitution.

Checked card-type reads retain unknown evidence on the existing incomplete
execution latch. Canonical linked exile reads exact current membership and
characteristics; a source reference uses current or exact departure/phasing
information. Arbitrary captured tags remain historical. Known empty sets and
known typeless objects remain distinct from missing required evidence.

Main already supplied `ExternalEvaluationOptions.triggering_object_current`
and a normal-resolution Sigil Captain fix. This restoration preserves that
flag, strengthens the existing owner with checked error propagation, and
routes immediate resolution through the same checked current-state mode.
Admission uses exact completed entry receipts; resolution uses the current
exact incarnation or exact departure/phasing evidence. A malformed destination,
mismatched snapshot identity, or incompatible expected zone is incomplete
evidence rather than a false condition that an outer negation can authorize.
Explicit historical-tense conditions continue to read event LKI.

Land-play events append the checked original snapshot and explicit completed
destination. The original native entry receipt supplies that destination:
Battlefield for ordinary entry, or the actual final zone for redirected play.
The frame is captured before replacement additions. It is not inferred from a
later object or forced to Battlefield. Native completion, receipt, X-declaration,
target-reference, instruction-result, and payment fields from main remain
intact. Paid-cost conditions retain main's existing receipt interpretation.

Amareth's singular-entry reference capture shares the same checked evidence
selection as ordinary condition rechecks. Looking at a library card cannot
replace the separate entering-permanent reference. Grouped entry references
retain their existing whole-set owner.

## Authored verification, all UNRUN

`crates/ironsmith-compiler-runtime/tests/shared_card_type_references.rs` contains
complete frozen direct compilation and serialized artifact round-trip paths,
parse-loss checks, actual casts, entry/exile choices, repeated land/spell actors,
independent copied-source links, redirected effect-driven, priority and direct
special-action land plays (without a false ETB), changed
spell control, current/copied/departed/phased types, source loss, empty linked
sets and libraries, optional private/public reveal, and stack rollback/recovery.

`crates/ironsmith-engine/src/filter/shared_card_type_reference_tests.rs` contains
narrow current/retained/copy/type comparisons, negative and zero-count failure
paths, missing versus empty evidence, event-time/current-mode contracts, retained
historical tense, malformed destination/identity/zone cases, trusted destination
mapping, redirected-play evidence, legacy actor-only notices with unavailable
characteristic evidence under positive/negative gates, and singular-entry capture. Local grammar
contracts require complete phrases and preserve both operands. Existing native
original-play/additional-program tests on all three native owners now inspect
the recorded play frame to ensure it precedes added counters. Existing complete Sigil Captain scenarios
are retained unchanged.

## Four partial bodies, no restoration credit

- Creeping Dread needs a simultaneous disclosure boundary and exact per-actor
  successful-discard comparison. `DiscardEffect::prepare_simultaneous_player_action`
  advertises `SelectionRevealPolicy::Public`, documenting that peers open each
  selection before replay; delayed game-state marking in `DiscardProposal::commit`
  alone does not establish simultaneous reveal across participant choices.
  `ForPlayersEffect::finish_players_outcome` retains actor counts and affected
  memory partitions; zero counts must be distinguished from missing positive
  result evidence before binding only matching opponent recipients.
- Holistic Wisdom needs its prospective cost/target protocol and exact actual
  exile-cost receipt. No absent unpaid receipt may become an accidental target
  restriction or a successful type comparison. Completed/replaced/prevented
  payment and conditional return require their own whole-body closure.
- Reality Scramble needs an original target characteristic snapshot independent
  of the returned new-zone object, exact owned-target movement, complete reveal
  match/remainder disposition and random bottom order, plus retained retrace.
  Generic tagging currently favors explicit result-object IDs, which is correct
  for later movement but can replace old battlefield types with printed new-zone
  types if reused as the comparison reference.
- Wild Magic Surge shares the consult/reference owner, retaining the original
  target/controller and continuing the reveal procedure when legal destruction
  is prevented or the permanent is indestructible. Exact matching permanent,
  no-match/empty-library, entry completion, random remainder, missing evidence,
  and rollback/recovery remain required.

Historical source-clearance before filesystem loss was `809458b5` for the
Cemetery pair/shared recheck and `b9bcbcc9` for Amareth. Those dispositions do
not substitute for fresh review of this recovered code on the new baseline.
