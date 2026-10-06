# Fixed-beneficiary private exile permissions

Status: source-authored, UNVALIDATED. Base:
2c70dc1180ca6b283f80a674487b291ca24db12d. This packet proposes only the complete
frozen Intet, the Dreamer body in the existing eight-card fixture. No builds,
compilation, compiler probes, tests, formatters, or engine/corpus execution
were run. Corpus baselines, ledgers and public versions were not changed.

The source-presence play duration is a new typed value, separate from the
existing control-dependent duration. GrantPlayTaggedEffect creates an
EffectWhileSourceOnBattlefield grant with the resolving player as its fixed
beneficiary and the exact source ObjectId as its lifetime owner. Control and
ability loss do not transfer or cancel this effect-owned permission. Leaving
and returning creates a new source identity. A source absent or phased out
when the permission resolves cannot start the duration (CR 611.2b). The
phasing transition eagerly removes these grants, including indirectly phased
attachments, so phase-in cannot revive them even without an intervening query
(CR 702.26f). Static play scopes retain their separate lifetime owners.

The free alternative price and land permission share the existing tagged-play
owner. Each targets only the actual exile ObjectId, never a StableId fallback;
leaving and reentering exile cannot restore play authority. Ordinary timing,
land limits and spell additional costs remain with their existing owners.
The new duration currently rejects flexible-mana, library-top and
counter-on-source extensions at native execution instead of silently losing
those extra scopes. None of those extensions occurs in the proposed body.

LookAtObjectsEffect now distinguishes an immediate look from a persistent
exile inspection entitlement. The new permit_while_exiled mode requires one
exact tagged exile antecedent and grants the resolving viewer durable private
access without prompting or revealing the card. This access survives source
control loss, ability loss, departure and phasing. It ends when the card leaves
exile, and CR 406.3 also ends it when the card becomes part of a shuffled
face-down pile. The engine has no owner for that exile-pile shuffle operation;
that interaction remains explicitly held. This packet does not add it.

The source-lifetime grammar reuses the existing complete source-presence
predicate grammar. A typed four-sentence composition proves optional mana
payment, its successful-result branch containing a single face-down top-card
exile, and the exact inspection/play readers. The readers stay inside the
payment branch. Its tag is allocated on the actual producer and rebound onto
both readers. Single-ability instruction references use that resolution's
exact tagged collection rather than the cross-ability source-exiled union.
Copies and independently acquired abilities retain independent execution
contexts. This route does not broaden any CR 607 definition/acquisition pair.

ExileTopOfLibraryEffect now establishes a known-empty moved-tag receipt before
attempting moves. Empty libraries and replaced destinations therefore do not
reuse an earlier captured collection. Missing native antecedents and attempted
source-union readers are explicit IncompleteEvidence errors on the new scopes.
Its existing transactional owner restores those receipts on failure/pause.

Both serialized enums are append-only. A source comparison against the base
verified all seven existing GrantPlayTaggedDuration variants and all nine
GrantSource variants retain their complete ordering; new ordinals are 7 and 9.
The added LookAtObjectsEffect false field and GrantPlayTaggedSurface None field
are omitted from serialization, preserving old artifact payload/checksum
shapes. Runtime/artifact mapping uses the existing typed effect carriers.
Artifact 6, public checkpoint 3 and audit 19 are unchanged.

Authored, unrun direct/artifact full-body scenarios cover real combat damage,
exact {2}{U} payment, decline/wrong-color payment, own versus damaged player's
library, face-down confidentiality without an immediate view, free spells,
additional spell costs, lands and timing/land limits, unrelated source links,
control changes before and after resolution, ability loss, source absence,
source and victim blink without intervening queries, permanent phase-out
expiry, copied pending triggers, native recovery, pending payment rollback,
replacement failure, failure after both entitlements exist, arrival-ID reuse
without an inspection reader, known-empty replacement receipts, missing native
antecedents, old artifact/default serialization, and canonical full-body
rendering independently reparsed with both lifetimes. Grammar scenarios cover
complete valid and invalid duration/inspection tails.

Intet stays partial until independent source clearance; the later complete-card
execution gate also remains. Intellect Devourer, Rogue Class and Elder Brain
remain partial, including their permission-specific flexible-mana and/or
cross-level acquisition obligations.

Rules references used in independent source review:
https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf
https://magic.wizards.com/en/news/feature/double-masters-2022-release-notes-2022-06-24
