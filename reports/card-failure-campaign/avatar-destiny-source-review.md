# Avatar Destiny: complete-body source evidence

The stat/type-addition ambiguity is repaired generically by `d2018867afdddc5243c0852c7a12446028ade5f7`. This checkpoint adds complete frozen-body evidence for Avatar Destiny without changing that parser or introducing a card-name recipe.

The previously implemented owners cover its remaining instructions:

- `parse_sentence_return_multiple_targets` parses the whole coordinated return as two complete, independent return clauses. The existing local grammar scenario already uses this exact return sentence and retains owner-hand versus controller-battlefield destinations.
- The mill lowering records an exact result collection; `PriorEffectAction::Milled` references bind to that producer rather than the Aura returned by the intervening branch. The printed optional creature is chosen on resolution, without creating an announced target.
- The enchanted-creature death trigger retains the host's death characteristics for the mill's power quantity.
- `resolve_source_object_id` uses `aura_source_graveyard_incarnation` for the precise attached-Aura/SBA exception. It requires the captured attached Aura and its first recorded battlefield-to-graveyard state-based transition, and only that exact destination object remains eligible. It does not chase a card through an additional exile/graveyard transition.

The full raw fixture includes the enchant restriction, both static predicates, mill, Aura return, and optional reanimation. Independent strict direct and serialized/restored artifact scenarios cover own-controller graveyard count and Avatar subtype, attachment legality and loss, an Aura owned by a different player, actual creature death plus Aura SBA, exact death-power LKI after the host returns as a new object, correct four-card mill, optional zero/one creature choice, exclusion of old graveyard/host cards, return to the Aura owner's hand, reanimation for the trigger controller, rejection of a newer Aura incarnation, and successful Aura return when no creature is milled.

These are source-review candidates and authored unexecuted scenarios. No builds, tests, compiler probes, formatters, corpus execution, matrix edits, or publication occurred. Independent review is required before treating Avatar Destiny as complete source coverage.

## Independent-review correction: required Aura departure evidence

After the helper has established the captured attached Aura and the dying host, an absent first battlefield-departure record or missing destination-object mapping now records `IncompleteEvidence`. Explicit known other destinations or non-SBA causes remain ordinary no-return; a recorded graveyard destination that changed incarnation still remains ineligible. The source resolver otherwise stays unchanged. A new full direct/artifact death-trigger scenario removes each required piece of actual recorded evidence and requires rollback of the earlier mill, graveyard mutations and pending events while keeping the unresolved trigger on the stack. Existing newer-incarnation scenarios remain the complete known-no-return control. Execution remains deferred.


## Independent-review correction: exact public mill destinations

CR 701.17c in the official September 25, 2026 rules permits the original public destination of a milled card, including a replaced exile arrival. The compiler now records that policy as the additive, default-false `ObjectFilter::match_captured_public_destination` field. It retains the exact producer tag, suppresses only inferred destination defaults, keeps explicit authored zones, and lowers public-result returns through the existing generic move owner. Candidate discovery considers public zones; matching requires the exact object ID, stable ID and captured destination rather than following a newer incarnation. Missing producer collections latch `IncompleteEvidence`.

CR 406.3a separately makes a face-down exiled card characteristicless. Public origin matching does not exclude face-down identities. The existing characteristicless hidden-agenda projection is moved to the general zone/characteristic owner and also covers face-down exile; filter reads use that projection and its absent mana cost/value. This keeps printed creature types out of the selection even when a player can look at the card.

Authored native direct/artifact full-trigger scenarios use actual mill replacement receipts for face-up exile success, hidden-hand exclusion, face-down exile exclusion from the creature-card choice, and a later exile incarnation. A separate real tagged mill scenario proves unqualified face-down public identity remains findable, an explicit graveyard restriction stays restrictive, and later incarnations are excluded. Local grammar assertions retain the typed public-origin policy and explicit zones. All remain unexecuted.

Primary source: https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf, rules 701.17c and 406.3a (verified October 5, 2026).

Required captured collections are now validated at the query/choice boundary before candidate enumeration, including the Value snapshot fast path. A present empty collection remains valid; an absent collection latches `IncompleteEvidence` even with no public objects, and negation/count cannot convert that absence into success. Public-origin Values read the exact live successor, so a later move or face-down change cannot qualify through old snapshot types. The derived target candidate view and choice-zone owner also honor public origin enumeration. Native checked-sequence scenarios distinguish missing versus explicit-empty evidence for a move query, a count, and an outer negated count, requiring rollback of earlier life gain. No scenarios were executed.

Final bounded independent source review clears this complete body at its recorded corrective head and additive central `ad6bc1d17f0ea42a0dd742072ecdefaf507a921f`. All authored scenarios remain unrun; this is source-proposed coverage only.
