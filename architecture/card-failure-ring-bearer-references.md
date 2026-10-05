# Ring-bearer event and designation references (UNVALIDATED)

Reserved cohort: nine direct roots (Aragorn, Company Leader; Call of the Ring; Dúnedain Rangers; Faramir, Field Commander; Galadriel of Lothlórien; Gandalf, Friend of the Shire; Galadriel, Elven-Queen; One Ring to Rule Them All; Lord of the Nazgûl) and two adjacent body-tail candidates (Frodo Baggins; Sauron, the Necromancer). Exact full Oracle fixtures and frozen identities are in `fixtures/ring_bearer_references.json.fixture`. No identity is promoted by this initial producer checkpoint.

## Historical choice evidence

The existing RingTemptsYou keyword event now retains an exact selected-object snapshot under RING_BEARER_CHOSEN_TAG. Its existing action source remains the effect that caused the temptation; it is not replaced with the selected creature. A legal selection of the already-designated creature still records a choice. With no eligible creature, the Ring still tempts the player but no selection tag is created. Phased-out creatures cannot be selected.

A typed RingBearerChosen trigger reads that event evidence. The intervening condition YouChoseAnotherRingBearer compares the recorded chosen ObjectId with the ability's original source ObjectId, and verifies the recorded actor. It never consults the player's current Ring-bearer. Losing the creature, blinking it, or choosing a different bearer before resolution cannot rewrite that historical fact. Current-source SourceIsRingBearer remains a separate condition.

The action now checkpoints its temptation count and designation before asking for a choice. Pending/error paths restore the original game/context, preserving the actual decision channel for replay rather than incrementing the count twice.

Authored native regressions cover nonchoice, same-bearer reselection, actor separation, historical source identity after designation change/departure, and pending rollback. Grammar source-context tests cover authenticated named-source aliases and reject an unrelated name. All are unrun. Current designation references, protection/LKI and complete secondary bodies remain under implementation and review.

No builds, tests, compilation or compiler probes were executed. Rustfmt source parsing and git whitespace checks only.

## Current designation and last-known protection

ObjectFilter::ring_bearer is a designation constraint, separate from creature type and from the historical selected-creature tag. The implicit noun is a battlefield permanent; `your Ring-bearer` additionally constrains its controller. It selects the currently designated exact ObjectId, excludes phased-out objects through ordinary selection rules, and remains valid when that permanent stops being a creature. Scalar characteristics use an aggregate over this unique current designation: its characteristic when present, zero when absent. They do not invent a new target or borrow a stale temptation event.

ObjectSnapshot and RetainedObjectSnapshot carry optional historical Ring-bearer evidence. Actual game snapshots capture Some(true/false); synthetic public placeholders carry None. Old JSON without the new field decodes to unknown, matching the existing payer-evidence migration policy. Unknown evidence can consult only a still-live exact ObjectId; it cannot follow StableId to a later incarnation. This is an explicitly incomplete historical fact, never a fabricated false designation. New snapshots preserve the original fact after redesignation, departure and blink. Retained schema regressions cover native and JSON transport, including missing-field migration.

The new constraint disables characteristic-class dedup and uses the contextual layered matching path. Protection from the designation therefore reads live source state when present and captured designation for a departed damage source through the existing protection subject interpreter.

The cohort remains uncounted pending complete body review and authored direct/artifact scenarios.

## Complete-body source proposals

After source review, ten of the eleven exact fixtures are proposed complete: Call of the Ring; Dúnedain Rangers; Faramir, Field Commander; Galadriel of Lothlórien; Gandalf, Friend of the Shire; Galadriel, Elven-Queen; One Ring to Rule Them All; Lord of the Nazgûl; Frodo Baggins; Sauron, the Necromancer. This is eight direct roots plus two formerly adjacent partials, not eleven recoveries. All remain UNVALIDATED.

Two bounded body closures were required. The existing conditional self-blocking reader now accepts the typed SourceIsRingBearer predicate. The existing next-step unless owner now admits proven state predicates inside the delayed payload, and the ordinary unless readers defer to that complete delayed owner. Sauron's condition is consequently checked at the next end step, against the original source incarnation; it is not an immediate registration gate or a payment choice.

The semantic target interpreter treats an unannounced, untagged `your Ring-bearer` as the unique current set, with no invented resolution choice. If there is no bearer, counter placement and scalar quantities safely select the empty set and later instructions continue.

`ring_bearer_references.rs` authors direct/restored-artifact scenarios for all ten complete bodies: real temptation choice/nonchoice/reselection/actor and historical other-source evidence; Call's payment/decline; Faramir's token and death-history end-step draw; Galadriel's scry/reveal/tapped land; Rangers' intervening recheck; Elven-Queen voting and exact counter recipient; all three Saga chapters; protection from live and departed bearer damage sources; real casting through Gandalf's flash permission and Lord's ninth-Wraith/base-size/cleanup body; Frodo's conditional blocking rule; and Sauron's actual attack/copied attacking token/delayed unless with blink-negative.

Aragorn, Company Leader remains partial. The first body needs `a counter from among` choice grammar. Its second body requires exactly the counter kinds from the triggering placement event, not all kinds currently on the permanent. The exact full fixture and an unignored full-card regression remain, with zero complete credit.

Deferred command targets, only after the user opens the validation gate: `cargo test -p ironsmith-compiler-runtime --test ring_bearer_references`; grammar Ring-bearer/predicate/delayed-state regressions; engine `ring_bearer_filters_read_current_designation_but_snapshots_keep_exact_lki` and retained historical schema tests. The Aragorn regression is intentionally an explicit outstanding failure until its genuine secondary bodies are implemented.

## Review correction: phasing and authoritative history

The earlier optional-field compatibility proposal above is superseded. Missing Ring-bearer evidence cannot be migrated to a false protection match for a departed source. Retained JSON now requires the field, and the authoritative occurrence snapshot encoder/decoder recursively rejects any battlefield snapshot whose designation is unknown. This deliberately rejects older incomplete saves; no historical value is inferred from a later incarnation. Public/synthetic unknown snapshots remain display/reference structures, not accepted authoritative saved battlefield history. Native/schema regressions now assert rejection, including nested unknown attachment evidence.

SourceIsRingBearer is a current-state condition: it now excludes a phased-out source even though current_ring_bearer intentionally preserves the stored designation. The Sauron scenario includes designation followed by phasing before delayed resolution; the token is exiled and the stored source designation remains intact.
