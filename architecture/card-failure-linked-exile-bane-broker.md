# Standalone inspection and paired return

Status: source-authored, UNVALIDATED. Base:
a266965c02a93131be463f96d1dcc9f14745c5ef. This increment proposes the complete
frozen Bane Alley Broker body from the existing eight-card fixture, after the
reviewed static play, private inspection and Colfenor packets. No builds,
compilation, compiler probes, tests, engine/corpus execution or formatters
were run. No corpus baseline, ledger or public version was changed.

A standalone typed LookAtSourceExiledCards static payload grants inspection
without granting play/cast authority. Its immutable pair is carried through
the common core model mapping; runtime inspection uses the same exact
acquisition/member collector as the previously implemented combined reader.
The new StaticAbilityId and StaticAbilityPayload variants are both appended.
A source-only comparison against the base verified the complete ordered prefix
of all 361 prior IDs and all 218 prior payload variants is unchanged. In the
payload, PlayerSkipsDrawStep remains ordinal 123, PlayersSkipExtraTurns 124,
and FirstCoinBatchHeadsWin 217; the new inspector is ordinal 218. No existing
variant fields or old default serializations change, preserving the prior
artifact/checkpoint carrier shapes. Artifact 6, public checkpoint 3 and audit
19 stay unchanged. The initial review caught and corrected an insertion in
the middle of the payload enum before this packet could be integrated.
An inspector whose pair or acquisition is missing explicitly fails checked
admission. Conditional wrappers retain the typed inspector accessor while
the existing active-condition owner decides when it applies. Inspection keeps
the prerequisite's CR 406.3 lifetime, including the separately held shuffled
face-down exile-pile interaction.

A separate bounded compiler proof recognizes exactly three members: one
mana/tap-only activation drawing one card then choosing/exiling a card from
its controller's hand face down; the standalone inspector; and one mana/tap-only
activation returning exactly one card from the source-linked exile pool.
The choice tag's typed producer/consumer relation is checked; its spelling,
labels and card names do not select the link. Result-tag wrappers are inspected
through a shared compiler helper. Additional abilities, exiling costs,
alternative costs, unknown programs or selectors remain unbound. The existing
Bishop and static play binders retain their separate contracts.

The hand producer uses the same explicit no-prior-zone-viewer policy as Kheru.
The return activation captures its owner before costs. At stack resolution and
in ExecutionContext filter queries, a captured owner supplies exact current
pair members. Known-empty pairs replace stale source-wide tags. Missing native
ownership history is surfaced as IncompleteEvidence. Unmarked legacy contexts
keep their earlier source-wide behavior and are not claimed as repaired.
Normal singular selection, card-owner hand destinations, tap and colored-mana
payment remain with their existing runtime owners.

Direct/artifact full-body scenarios cover all three members; real draw/exile
and return activations; tap/UB costs; exactly one chosen return; unrelated
source links and known-empty sets; changed source controller versus card owner;
read-only inspection; copied pending activations after source blink; separate
borrowed activated pairs without a copied inspector; pending-choice rollback
including the preceding draw; native recovery; source-only import and missing
owner rejection; direct native filter contexts with polluted source tags;
victim blink; missing inspector metadata; extra-body/exiling-cost negatives;
insufficient-color mana with preserved untapped source and pool; and canonical
rendering independently reparsed into a complete three-member pair.
All scenarios remain unrun. Independent review cleared the corrected complete
body through 2c70dc118, followed by an additive central inspection at 50bf0de5a.
Bane is proposed and unvalidated; the complete-card execution gate still applies.
Intellect Devourer, Rogue Class, Intet, the Dreamer, and Elder Brain remain partial.
