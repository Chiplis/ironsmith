# Typed control-transition family (UNVALIDATED)

Frozen stack07 candidates: Coffin Queen, Duplicity, Gustha's Scepter, Ogre Geargrabber, Risky Move, and Zidane, Tantalus Thief. The exact six Oracle identities, complete card text, original hashes, and diagnostics are in `fixtures/control_transition_triggers.json.fixture`. They remain provisionally partial until the complete secondary-body/watched-reference review finishes. This checkpoint adds no coverage count.

## Event and source ownership

The typed trigger separates the permanent subject from the gaining/losing player and the optional former-controller qualifier. Gain from another player uses NotYou, not Opponent, so teammates are not conflated. CR 603.10d requires source lookback both for loss of control and for an opponent gaining control from you. Ordinary gain triggers inspect the completed state. Loss also accepts a battlefield departure or leaving the game, using the original exact-incarnation snapshot; phasing is not a loss of control.

The existing derived-controller reconciliation now owns soulbond and summoning-sickness consequences. ApplyContinuousEffect no longer performs them speculatively before a false duration/condition is rejected. A completed control boundary snapshots each exact battlefield object and every relevant trigger source. Actual old/new differences publish one event with both snapshots; unchanged control, initial entry, and repeated refreshes publish none. GainControl and directional-adjacent executors reuse those receipts instead of emitting duplicates. Expiration and static control changes use the same owner.

Lookback-source completeness is explicit, excluding a control-loss ability newly granted by the same instruction. Trigger matches are captured before later instructions can remove a gain observer. Physical receipts and history remain available. An existing coordinated trigger-matching hold preserves the previous baseline until the enclosing operation completes. The snapshot baseline is stored with the cloneable battlefield state, not in a cache discarded on mutation. Sync import rebases it only after actual continuous effects have been restored and every effective controller validated, without generating historical events or rewriting saved sickness/soulbond state.

## Delayed subscriptions

The model appends control-change and untap delayed variants without shifting existing serialized enum ordinals. A source-subject untap/control-loss union explicitly watches the ability source. A previously chosen Equipment's control-loss watcher resolves and pins its existing tag, independently of the source creature. Existing source-control-loss subscriptions also see actual battlefield departures.

## Authored regressions and remaining review

All six full-card fixtures round-trip through direct/artifact compilation. Runtime scenarios are authored for old-versus-new controller ownership; Zidane seeing its own theft; cleanup expiration; static Aura control ending; no-op and phasing negatives; source departure; simultaneous removal/granting of the trigger ability; and false-duration soulbond/sickness preservation. The existing static-control sync scenario now asserts one actual post-import transition and matching before/after incarnation snapshots.

Complete-body review remains open for linked face-down exile choices/returns, Coffin Queen's reanimation plus source watcher, Ogre Geargrabber's selected Equipment follow-up, and Risky Move's independent object/player choices plus coin result. No tests were executed. Only source reading, rustfmt parsing, and git whitespace checks were used.

## Exact-incarnation delayed watcher and complete-body follow-up

A delayed subscription explicitly watching its ability source now pins `ctx.source`, or an explicitly supplied ability source, rather than following arbitrary stable-card moves before registration. Explicit moves performed within the resolving program already update `ctx.source`. Ordinary non-watcher delayed references keep their existing permitted source-zone exceptions, including the separately repaired Aura/SBA reference path.

Four identities are now proposed complete by source review, still unvalidated: Coffin Queen, Gustha's Scepter, Ogre Geargrabber, and Zidane, Tantalus Thief. Added authored direct/artifact scenarios pay the Queen's mana/tap activation, retain the reanimated controller and owner, exercise untap/theft/departure/phasing, and ensure a Queen blinked before resolution is not the new delayed subject. Scepter scenarios pay both tap activations, exercise its linked exile/return, then check theft or departure sends the linked card to its actual owner's graveyard. Geargrabber uses a real attack declaration, chosen Equipment attachment and end-of-turn control expiry, leaving the unchosen Equipment untouched.

Duplicity and Risky Move remain explicit partials until their separate complete-body review/scenarios finish. No compilation, tests, or compiler probes were run.

## Atomic control-boundary correction

A control-reversion observer that departs in the same operation remains in that operation's old source set. The publisher no longer filters old sources by post-event zone membership. Reconciliation of sickness/soulbond and event publication both wait while a simultaneous action, pinned simultaneous source lookback, or explicit matching hold remains open. A pending-boundary bit prevents a nested cache refresh from suppressing the first authoritative post-operation reconciliation.

The unrun public scenario sacrifices Zidane and a theft Aura together while the stolen creature survives: old Zidane receives one Treasure trigger when control returns. Its paired negative sacrifices Zidane in an earlier completed instruction, then the Aura; that departed observer receives none. The source baseline advances between those distinct instructions.

## Final two complete-body proposals

Duplicity's existing reference resolver already excludes `SOURCE_EXILED_THIS_RESOLUTION_TAG` from “all other cards ... exiled with” the source. A new direct/artifact scenario covers its actual entry, five-card exile, optional two-card hand exile followed by return of only the older five cards, end-step discard, and control-loss cleanup into the owner's graveyard.

Risky Move required a bounded shared-verb choice reader: “choose [object] and [player]” now composes the existing complete object and player choice parsers under one typed coordinated instruction. Object and player bindings remain separate; incomplete operands, object-only conjunctions, and multi-sentence tails are not consumed. Its authored three-player scenario uses the real upkeep control transfer, verifies that the new controller chooses their own creature and a distinct opponent, records the real coin event, and checks that only a lost flip transfers that selected creature. Fixed RNG seeds cover both actual outcomes; no test has been executed.

All six exact identities are now proposed complete by source review. This is not a measured recovery count. The four earlier proposals require the atomic-boundary correction documented above. The whole target remains unvalidated until the user authorizes compilation/test execution.
