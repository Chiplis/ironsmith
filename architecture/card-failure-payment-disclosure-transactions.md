# Payment disclosure transactions

Status: UNVALIDATED source implementation and authored regressions. No build, compilation, JavaScript execution, or test execution was performed. Coordinated source review admits twelve related exact-card source proposals in stage40; the final section records that admission and the remaining execution gates.

## Scope and evidence

The existing completed-action Undo guard cannot undo information already opened to peers. This draft covers legitimate private-hand activation and Public selection cost flows for Knollspine Invocation, Krovikan Sorcerer, Sanctum Spirit, Kozilek, the Great Distortion; the separately held Illuminated Folio/Sphinx of the Chimes/Ormos, Archive Keeper family; and Glamorous Outlaw, Masked Bandits, Rakish Revelers, Shattered Seraph, Spara's Adjudicators. The held group-relation implementation is not included here.

Full-payment deferral is unsuitable for the current protocol: a signed public opening can be needed to validate or resume an actual replacement decision before the enclosing cost is finished. The implementation therefore retains an explicit irreversible disclosure commitment while allowing the same payment to resume. Public-selection proof requirements and all existing signature, opening, shuffle, RNG, quorum, and payment legality validation remain in place.

## Native boundary

`payment_disclosure_transaction.rs` wraps the actual public dispatcher's typed route. It derives payment source/payer from the live pending activation/cast or retained root, and derives hand disclosure from the legal native action or Public selection context. It does not trust a frontend assertion that an action is a payment.

Before disclosure, the existing cancellation behavior is unchanged. First hand activation/selection records the exact hand incarnations. Accepted intervening commands retain the ordinary authoritative pending state, which owns source, payer, X, targets, cost order, prepared mana, replacement prompt, and previous answers. A genuine engine error restores a whole per-command RuntimeSavepoint and returns an error. It does not accept a neutral command that has silently rolled back the whole announced action. The same normalized failed decision answer must be retried; its choices cannot be substituted. Cancel and completed-action Undo are blocked across this commitment.

When costs complete, the active commitment is consumed into an epoch Undo latch and a checkpoint-relative generation fence. The latter protects old replay checkpoints, including a prevented hand movement with no public zone-change event, while allowing a later safe action to cancel back to a newer checkpoint. Ordinary mana-only Undo is retained. Replay checkpoint restoration does not erase knowledge; full speculative RuntimeSavepoints and crypto previews restore their own commitment and generation state.

Exact disclosed hand identities remain visible after a failed command restores their original zones. The snapshot overlays only those cards onto the existing hand view, without replacing a simultaneous library/replacement view or changing the accepted public audit hash. Wire checkpoints explicitly mark live disclosure continuations and refuse unsafe import; recovery must use a lossless runtime branch or replay the accepted transcript. The optional marker defaults to false for old checkpoint data.

## Verified transport boundary

`payment-disclosure-journal.js` stores the match, accepted-prefix hash, sequence, actor, exact wire command, verified public openings, and signed evidence before the first material-bearing send. Storage failure prevents publication. A retry cannot change actor, source, X, targets, or chosen incarnations. Repeated material stages merge openings/evidence. Only accepted transcript advancement retires the corresponding sequence; a cancel or transient failure does not.

The optimistic publicClaims path and prepared provisional-opening path are bypassed for these payments. Payment progress broadcasts are count-only before acceptance, so labels and preview card names cannot disclose information before the commitment. Local previews can still show the acting player their own card.

Incoming material is authenticated and classified using the native engine after opening-proof validation, before speculative requirement/payment execution can fail. A sender-supplied boolean is never authorization. Existing cryptographic validation remains mandatory. This applies to both material requests and signed quorum/action envelopes.

Validation rollback restores its accepted-prefix runtime and crypto caches, then verifies/reopens the retained envelope and pins the same native retry answer. Full verified resync replays the accepted prefix, retires accepted pins, then performs the same recovery for the first unaccepted command. A retry reuses its retained signed intent, preserving the existing attempt-bound proof and timeout context; it does not deadlock waiting for its own pending intent. Forfeit remains available. No automatic alternative choice, new transaction, or replay-validation bypass is introduced.

## Authored controls

- `payment_disclosure_transaction_tests.rs`: actual typed dispatch, exact Knollspine body with a test-only transient fault after the real discard instruction; same-choice retry, no double mana/discard/event/stack payment, X/target/source/payer state retained, other choice rejected, disclosed card visible to opponent but unrelated hand identity hidden, speculative savepoint rollback, stable public audit state, wire checkpoint rejection, checkpoint-relative Undo.
- Earlier exact four discard and five SNC regressions now route through the same typed boundary. The discard target is exercised with one and two eligible candidates: hidden-zone selections may not be auto-filled even with one candidate (`make_decision.rs`), and forced private-hand reveals themselves require an explicit Public choice (`hidden_hand_choices.rs`). Their public-selection/zone-change assertions remain.
- `payment-disclosure-journal.test.js`: durable reload, exact retry, actor/prefix mismatch, merged immutable records, storage refusal, accepted-only cleanup, publication and recovery wiring controls.
- Existing ordinary mana-only Undo and private owner-view controls remain.

Deferred commands, not executed: `cargo test -p ironsmith-web-session payment_disclosure -- --nocapture`; `node --test web/ui/tests/payment-disclosure-journal.test.js web/ui/tests/opening-preparation-progress.test.js`; exact SNC compiler/runtime target and the held group target after its separate implementation is admitted.

## Source review corrections

The non-dispatch `cancel_decision` command bypasses native UiCommand metadata decoding, while the existing native cancellation and journal compatibility checks remain active. Incoming/recovered envelope authority is checked against the actual native decision owner (including an opponent's replacement choice) and the accepted action-history/hash head before any durable pin. Stale or unpayable mana Confirm proposals and illegal manual mana sources are rejected before a retry command is latched; a current valid proposal remains usable.

The journal also retains the original pending signed intent, first-observed time, maximum observed match-clock elapsed time, request evidence/timing, and timeout confirmation. Reconstructing the same head restores the native choice and this pending-intent record/timer. Repeated publication merges timing without granting a fresh hard deadline. These controls and their source/behavioral regressions are authored and unrun.

Once a signed payment attempt is pinned, its attempt ID and pre-action public checkpoint fingerprint are immutable. The journal owns one canonical signed intent used by both signing/retry and recovery. Timing and material evidence are checked against it before mutation; a different signature over the identical normalized payload does not replace the canonical record. Pending-intent map updates also reject a changed fingerprint, even when the underlying game command is unchanged.

## Coordinated source review admission

Stage40 integrates the transaction plus corrections for non-dispatch Cancel, actual decision-owner authority, stale mana plan admission, original intent timing across resync and immutable canonical signed attempts. The independent source pass found the reported defects addressed. Group selections use the opened pending decision game and complete relation/individual filters before commitment. Twelve identities are source-proposed only; none is a newly measured compile or runtime recovery. Rakish Revelers retains its separate token-cap gap. All authored execution remains deferred.
