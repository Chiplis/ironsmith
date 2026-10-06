# Counted counter transfers: source proposal

Base: `9a8bf2b6d16112ec22c39e2b240281193fdf0dd5`.
Validation: source inspection and Git/JSON bookkeeping only. No build, compiler
probe, formatter, test, engine, replay, browser, or corpus execution was run.

## Exact frozen bodies

`fixtures/counted_counter_transfers.json.fixture` retains complete corpus bodies
and their exact printing/oracle identities for Aetherborn Marauder, Scrounging
Bandar, Spike Cannibal, and Black Panther, Wakandan King. All four belong to the
frozen original unresolved set and were absent from the inspected source ledger.
No source-count or measured-recovery claim is made by this worker.

The counted-transfer owner now preserves an entire donor set. The grammar owns
the plural donor reading; typed AST cardinality lowers to `ChooseSpec::All`.
`CounterMoveAmount::All` samples every named counter on each donor. Optional
quantities are chosen per donor before any proposal commits. A move still has
one bound destination; this change does not claim distributed destination
support. Forgotten Ancient, Goldberry, Resourceful Defense, and Slippery
Bogbonder remain outside this proposal.

Every donor removal and the combined destination placement receives its own
ordinary replacement processing against the pre-mutation state. Their original
actions commit before appended replacement programs. Native game/context
checkpoints roll back the complete instruction on pending choices or errors.
Zero allocations, self transfers, absent/phased endpoints and prohibited
counter recipients do not consume a replacement. Combined placement and
recipient storage overflow return checked errors; there is no silent cap.

A `CountersMoved(counter kind)` result gate binds to the preceding matching
counted-transfer owner. The result is the amount actually removed by the
original removal actions. It excludes doubled placement and appended effect
results. The separately proposed placement budget remains independent of
removal replacement, consistent with the existing decomposition of CR 122.5.

Wizards' Marvel Super Heroes release notes, p.68, explicitly say that Black
Panther's life gain uses the number of counters removed from the land when
placement is multiplied. They also specify that either illegal target and
same-object transfer produce no move, life gain, or draw:
https://media.wizards.com/2026/downloads/MSH_Release_Notes_N6hMZoes90/zfGb4BJ8Bf_EN_MTGMSH_ReleaseNotes_20260513.pdf

## Prior consumers and compatibility

The counted ledger has two single-endpoint consumers: Tester of the Tangential
and Cytoplast Manipulator (graft). Neither reads the movement count. Both inherit
the correction preventing appended removal programs from running before the
placement original. No already-counted plural transfer body was identified.
The distinct MoveAllCounters/MoveOneCounter receipt contracts are not migrated
by this counted-transfer change.

Appending `CounterMoveAmount::All` and
`PriorEffectAction::CountersMoved(CounterType)` changes typed artifact schemas.
Plural transfer lowering and original-removal receipts also change executable
definition semantics. The coordinator must include these changes in the next
reviewed artifact/schema/protocol boundary; this work does not independently
modify version/hash constants or introduce serialized gameplay recovery.

## Deferred scenarios

Local grammar scenarios cover donor cardinality, named-all quantities, exact
counter-result kind, invalid descriptors, incomplete trailing text and
unsupported aggregate fixed quantities. Native owner scenarios cover complete
and partial/zero donor allocations, combined placement, one-shot consumption,
appended programs observing all originals, native pending retry, injected
completion failure rollback, replacement-modified removal/placement quantities,
and checked combined overflow.

Full-body runtime scenarios use independent direct compilation and validated
artifact JSON materialization, preserving Flying/Lifelink, first strike, both
entry-counter abilities, owned-upkeep timing, optional donor amounts, both
Black Panther triggers and its real paid activation. Stack clones retain native
state, targets and exact effect programs before resolution. Black Panther cases
cover distinct and same-object targets, each illegal endpoint, placement
doubling, prevented removal, and life/draw follow-ups. An unlisted name verifies
that the grammar is not gated on any card identity.
