# Native replacement iteration continuations (UNVALIDATED)

No new identity count. Wedding Ring remains partial pending the concrete receipt boundaries below.

The existing ForPlayers action-major owner now retains its exact action-unit and participant cursor at the first draw. The retained state includes player-local result maps, object/player tags, optional-group acceptance and completion, selected APNAP participants, shared-team actor selection, and accumulated event receipts. Prefix choices and instructions are not rerun. The child restores its complete execution context inside the original source/optional scopes. Existing player-major, controller-first, and stop-after-first-success variants use a separate native cursor that preserves their ordering and tag-reset rules.

Each-player draws are sequential under CR 121.2c/d. A draw action unit therefore runs each participant's complete draw/replacements before the next participant; its events are captured at that boundary without opening a new simultaneous batch. Other simultaneous actions continue to prepare every original before committing and finishing receipts.

RepeatEffects retains its resolved count once, completed iteration outcomes, the paused current subtree, and remaining count. Each iteration preserves the native sequence status/output semantics and separate shared-structure operation scope. Fixed zero draws do not create a boundary; dynamic draw counts are evaluated when the deferred draw executes.

Authored, unrun direct/artifact scenarios cover per-player prior results; action-major versus player-major prefix order; optional acceptance/decline without reasking; controller-first offers stopping only after a completed accepted body; frozen repetition count; pending-input rollback of all original participants and prefixes; and an earlier participant's draw before a later participant's replacement removes qualification.

## Remaining receipt review

The literal-draw scan does not detect a draw introduced dynamically by a replacement of a non-draw leaf. Such a nested leaf currently completes its own deferred receipts before returning to its enclosing original. This is a separate runtime scheduling boundary, not a reason to drop the payload or count Wedding Ring complete. A second, bounded source seam is draw_cards.rs: an earlier direct draw segment must be published before a later draw replacement program mutates its event-time qualification. Neither gap is hidden by a skip or ignored test.

No builds, tests, compilation, or compiler probes were run. Source parsing with rustfmt and whitespace checks only.

## Direct-draw receipt follow-up

The earlier direct segment now finishes and captures its trigger receipts before either an Instead payload or added program for a later draw executes. This preserves physical history receipts and avoids matching them twice when the enclosing instruction reports them. The empty-library replacement scenario above covers the previously lost first draw. The remaining partial is the dynamically introduced draw in a non-draw replacement leaf; its active nested-life regression remains unignored.

## Deferred design: dynamically introduced draws

The active `draw_introduced_by_nested_life_replacement_waits_for_the_enclosing_originals` regression is deliberately unresolved. A's outer life gain becomes a life gain for D; D's life replacement draws for A; that draw's addition removes B's Ring. B's still-unreplaced outer gain must happen and match first. Literal DrawCards inspection cannot discover this boundary in A's initial payload.

A future implementation must expose a prepared completion as either a fully completed non-draw result or a completed prefix plus a draw continuation, preserving the ordinary source/controller, replacement-history, targets/tags and outcome alias scopes. Non-draw prefixes must execute in place. Merely enqueueing every completion is wrong because a non-draw prefix can itself change qualification before another original.

The native ForPlayers simultaneous path also needs a completion-stage cursor: retain all committed inner-original receipts, each participant's execution-context checkpoint, the current prepared completion and remaining completions. Pause only after every inner original has committed, then let the enclosing original owner finish its own unreplaced participants. Resume the current child and its suffix once, followed by remaining inner completions. Reclassifying a life-only action unit as player-major would lose its original simultaneous prestate and is explicitly excluded.

The implementation should share the life proposal owner's prepared typed event rather than duplicating player/amount/legality logic. Scope restoration, exact source snapshots, original-event quantities versus added quantities, receipt deduplication and rollback of all nested participants remain required. Extend the regression with a no-draw nested replacement (prefix remains immediate), nested simultaneous participants, pending/error rollback, and source/tag/result retention before promoting Wedding Ring. No global draw queue, placeholder, or coverage count is introduced by this note.
