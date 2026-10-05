# Random target announcement: proposed bounded ownership delta

Design checkpoint only. Goblin Test Pilot and Witch Hunt remain held. No
announcement implementation, protocol execution, build or test has run.
Reviewed resolving-selection checkpoint: `976e9e60c`.

## Existing owners and gaps

- `grammar/shared_util/target_semantics/reference.rs` consumes the random marker
  before its AnyTarget early return, losing ChoiceCount.random. Player subjects
  require the same typed-count preservation through player lowering.
- `decision/types.rs::TargetRequirement` retains the full ChooseSpec. Ordinary
  candidate construction already owns target legality, range, chooser-relative
  filters and target-set restrictions. Flagbearer is a separate player-choice
  requirement and must not narrow a random target domain (official source below). `TargetRequirementContext`
  drops the random method; ordinary selection presently goes to a player.
- `game_loop/priority_cast.rs` announces activation/cast targets before paying
  costs. `priority_apply.rs::apply_targets_response` already builds the final
  target assignments. `sba_triggers.rs::choose_trigger_targets` performs the
  equivalent procedure for triggered abilities. Random target selection belongs
  at these boundaries, not in an effect executor.
- `GameState::shuffle_slice` owns transcript-derived random authority and the
  FairRandom counter/operation. Coin work through central `88dd882d5` does not
  add a separate commitment service; its native instructions use checkpoints.
- `PriorityLoopState::rollback_action` restores the pre-action GameState and
  clears pending work. By itself it can rewind an already-observed random draw.
- `wasm_game_impl/payment_disclosure_transaction.rs` is the existing native
  cross-command commitment/failed-answer owner. It derives source/payer from
  native state, blocks cancellation after disclosure, preserves exact retry
  answers and uses RuntimeSavepoint for speculative rollback.
- `payment-disclosure-journal.js` already durably binds match, accepted prefix,
  sequence, actual decision actor, normalized command, immutable signed attempt,
  pre-action checkpoint, original timing and authenticated material. Recovery in
  `connections.js` verifies and replays the accepted prefix and retained attempt.
- `validation.js::lockFairRandomRevealIntent` already pins a revealed nonce to
  the sequence/actor/pre-state/command in memory. Its RNG request context binds
  the requirement and pre-action public checkpoint. This is distinct from the
  durable signed-attempt journal; random announcements must reuse the latter
  rather than rely only on the in-memory lock surviving reload.

## Proposed storage and admission delta

1. Preserve the existing random ChoiceCount in semantic/runtime ChooseSpec.
   Add a shared native random-target selection helper used by activation/cast
   announcement and trigger stacking. It receives the final legal native
   requirement domain and outputs ordinary targets/TargetAssignments.
2. Represent one engine-generated announcement receipt as immutable values:
   source ObjectId and source incarnation, ability origin or trigger occurrence,
   announcement controller, actual target-choice authority/context, requirement ordinal,
   typed specification, ordered legal candidates/legal groups, selected exact
   targets, and random-counter witness. Equality of the complete domain is the
   native check; an ordered-domain fingerprint accompanies external evidence.
   No frontend-provided target or candidate list is authority.
   Carry a distinct typed **pre-draw** announcement domain and actual authority
   from the native random operation through `CryptoRequirementView` and peer
   verification. The domain does not contain the selected result. Generic
   FairRandom's priority/active-player fallback cannot authorize this operation.
3. Store live receipts with PendingCast/PendingActivation and the matching
   trigger-announcement continuation. Clones own their own immutable values;
   speculative legality, previews and native savepoints must never share a
   mutable receipt cache. The signed attempt ID remains owned by the transport,
   not invented by a native filter or inferred from the source card name.
   For automatic trigger stacking, retain the native trigger occurrence and its
   triggering command/root separately from the trigger controller. The signed
   command actor must still be the current decision owner; neither trigger
   controller nor priority player substitutes for that actor during recovery.
4. Extend the existing native disclosure commitment with a typed random-target
   reason and native-derived receipt/domain metadata. Retain its existing
   same-answer retry, cancellation fence, actor validation, epoch Undo latch,
   RuntimeSavepoint and unsafe-wire-checkpoint rules. Do not create a second
   native journal or permit a Cancel result that silently retires the receipt.
5. Extend the existing durable journal's material envelope to admit verified
   random-target requirements/commit sets/reveals without requiring hand
   openings. Before any nonce or material-bearing message is released, validate
   the native requirement against the current accepted head and actual decision
   owner, and pin the canonical signed attempt/domain. Conflicting evidence for
   an existing requirement must fail; later stages only add consistent material.
   A storage failure blocks publication. Existing hand-disclosure behavior stays
   under the same owner.
   Replace shallow evidence merging with conflict-checked monotonic admission
   per requirement. Apply the same admission on both collector and nonce-response
   paths before release. Persist each exact local contribution/reveal and locked
   commit-set identity before publishing it: `rngCommitNoncesRef` alone cannot
   survive reload. Verified remote reveals and seed consumption also have exact
   per-requirement markers, so recovery neither regenerates a local contribution
   nor injects a consumed witness twice.
6. Gate the new typed domain and RNG-only evidence with an explicit audit-wire
   capability/version and journal schema version. The current audit version is
   18 and the current journal prefix is v1. Reject incompatible peers before any
   nonce or material release. Preserve readers for old accepted transcripts and
   old hand-disclosure journal entries; do not reinterpret an old generic random
   operation as a random-target announcement.

## Retry, recovery and completion

- Pending or failed native execution restores reversible physical work while
  retaining the immutable attempt/receipt obligation through the existing
  commitment owner. A changed source/actor/domain/command must fail admission,
  not request new randomness. Legitimate retry must consume the same verified
  material once and recreate the same assignments; no duplicate seed injection.
- Reload/resync verifies the retained canonical signature, original timing,
  accepted-prefix hash and RNG proofs, reconstructs the same native continuation
  by replay, then reinstalls the existing retry fence. It does not import an
  arbitrary chosen-target receipt or trust a host's reconstructed state.
- Only accepted transcript advancement retires the durable attempt. Forfeit
  remains the existing terminal alternative. Before disclosure, ordinary
  cancellation remains available. After successful announcement, later
  resolution uses normal target legality and never rerolls an illegal target.
- Genuine announcement impossibility is a separate, checked native terminal
  verdict, accepted in that same signed attempt's ordinary transcript. Restore
  reversible announcement/cost work, emit no successful cast/activation event,
  retain the consumed random receipt/witness in accepted history, and release the
  live continuation. Exact retry/reload before acceptance must reproduce that
  verdict with the same witness. Malformed answers, planner errors, resource
  exhaustion and incomplete evidence are **not** impossibility and remain
  retryable under the same-attempt obligation. The proof must include complete
  native payment possibilities and the existing authenticated private-source
  evidence: an opaque hand, incomplete mana analysis or an owner's unverified
  claim cannot certify inability to pay.
- A later distinct legal attempt follows ordinary rules and can obtain a new
  random result. There is no persistent same-gameplay-frame target cache or ban
  on future activation. The same-attempt fence prevents voluntary cancellation
  and altered retries; it does not turn an accepted impossible attempt into a
  permanent restriction on later gameplay.
- Empty legal domains follow normal activation/trigger legality without consuming
  a random witness. Selection is uniform over the legal domain; exact candidate
  allocation and constrained-domain work have checked incomplete failures.

## Required authored scenarios before source admission

Goblin: full flying/tap/damage body, legal creature/player/planeswalker/battle
population, protection/range legality and explicit Flagbearer exemption, source eligibility, announced
assignment retained through cost suspension/error/retry, cancellation/Undo fence,
illegal target at resolution and no reroll. Witch Hunt: life-gain prohibition,
real upkeep damage, real end-step trigger, current controller/opponents, random
announcement before response, departed/illegal opponent and source changes.

Shared boundary: same-seed owner/peer assignments; changed candidate order,
actor, signed attempt, pre-state, source incarnation or requirement rejected;
ordinary user targets unaffected; exact retry and no second consumption; resource
failure before/after selection; no speculative-clone publication; durable storage
failure; reload/resync with original timing; existing payment journal regressions.
Also cover automatic trigger roots with a different command actor, RNG-only
journal recovery on both collector/responder paths, incompatible peers before
release, invalid cost answers versus checked target-dependent impossibility,
hidden payment evidence remaining incomplete, accepted impossibility freeing
priority, and a subsequent distinct legal attempt with normal random authority.

## Verified Flagbearer boundary

The official [Mystery Booster release notes](https://magic.wizards.com/en/news/feature/mystery-booster-release-notes-2019-11-11),
under Enroll in the Coalition, explicitly exempt random target selection from
Flagbearer requirements. Accordingly the random branch must use the complete
legal domain and skip `enforce_flagbearer_targeting`; it must still enforce true
legality such as protection, hexproof, shroud, player status and applicable range.
The first implementation is limited to the two held single-target bodies.
Random authority is the existing verified RNG, not a player allowed to substitute
a target; transport actor ownership remains the actual native decision actor.
The same release notes' High Troller ruling forbids voluntarily abandoning an
announcement after random targets are selected, while allowing reversal when
there is no way to complete it. That exception motivates the checked terminal
verdict above; it supplies no rule pinning a later distinct attempt to the old
target.
