# Actual milling transitions

Status: **UNVALIDATED** source implementation; no compilation, tests, or compiler
probes. Exact fixture identities and frozen blockers are in
`fixtures/milling_transition_triggers.json.fixture`.

The first source batch proposes six complete printed programs: Glowing One; Lo
and Li, Royal Advisors; Mirelurk Queen; Screeching Scorchbeast; Infesting Radroach;
Zellix, Sanity Flayer. The follow-up supplies The Wise Mothman's target-count
binding, bringing the printed-program proposal to seven. The required simultaneous multi-player replacement-addition interaction now has
a source closure and remains an unignored regression. Every claim remains a
source proposal, with compilation and runtime validation deferred.

## Typed boundary and actual producer

The compiler retains active/passive subjects, a card filter, and singular versus
one-or-more cardinality. An explicit player subject groups that player's cards;
a passive clause groups all matching cards in one simultaneous instruction.
Shared discard/mill alternatives inherit the authenticated player subject while
keeping discard singular and mill grouped. Core event/trigger variants append to
their serialized enums; there are no card-name dispatches.

Only actual MillEffect execution emits CardMilledEvent. An arbitrary library to
graveyard zone move, surveil, or a prevented/replaced-away mill does not fabricate
this action. The notification retains the exact original library ID, exact
result ID, acting player, and public completed-state characteristics. Public
replacement destinations (including exile) retain the mill; hidden destinations
supply no characteristic snapshot, so a nonland/creature filter cannot reveal
private information. Unknown characteristics never imply nonland.

Each-player mill proposals freeze the count and original top-card identities
before any player's proposal commits. A departed original is not replaced by a
new top card. Notifications use the existing simultaneous action identity.
Their snapshots finalize at the outer action boundary, including lookback-owned
batches, so intermediate type/control changes from another participant cannot
leak into the completed batch. Later instructions do not refresh older batches.

The runtime exposes 1 per matched card; existing coalescing sums only matching
cards for “that many”. This supports Scorchbeast's filtered token count without
confusing it with a count of all cards. Its existing DoThisMaxTimesEachTurn
predicate counts accepted optional actions; Mirelurk's trigger cap is separate.
Radroach retains its graveyard functional zone and intervening-if recheck.
Zellix uses the existing generic authored ability-word path.

## Rules and deferred checks

The [current comprehensive rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt)
701.17 define the mill action, partial milling, and finding milled cards in public
replacement destinations. The [official Fallout release notes](https://magic.wizards.com/en/news/feature/magic-the-gathering-fallout-release-notes)
explicitly distinguish non-mill library moves and require Mothman's passive event
to trigger once across simultaneous players.

Authored direct/artifact scenarios cover all seven exact cards; actor/opponent
separation; singular/per-player/passive grouping; independent instructions;
non-mill moves; public/hidden/prevented destinations; zero/empty libraries;
prepared dynamic counts; exact ID references; radiation; actual paid Lo and Li
and Zellix activations; optional-action versus trigger limits; and Radroach's
source departure/reentry. Separate secondary-body scenarios cover rad counters
from actual entry, attack, and combat damage. Runtime target:
`cargo test -p ironsmith-compiler-runtime --test milling_transition_triggers`.
All commands remain deferred until the campaign validation gate.

## Exact filtered triggering-count proof

A filtered prior-action query retains its Milled action, card predicate, source,
and metric. Only an unresolved affected-object Count query with no additional
player/counter qualifier can use a grouped mill trigger's amount, and only when
its entire ObjectFilter is semantically equal to the trigger's filter. The
predicate proof survives compiler reference frames. Mixed event alternatives,
differently filtered alternatives, singular mill events, and mismatched body
filters have no such proof. A compatible local mill producer still wins before
this event fallback. No general numeric fallback discards a card predicate.

Authored Mothman scenarios assert the target-set cap at announcement, one trigger
across three simultaneous players, excluded lands, optional zero targets, exact
count after a milled card leaves, and its separate entry/attack rad-counter body.
Unit/public negative scenarios retain incompatible event predicates and local
producer precedence. They remain unrun.

## Original commit followed by retained replacement programs

ForPlayers now asks each proposal to commit its original mutations separately.
Existing proposals preserve their established one-phase default. MillProposal
returns its replacement receipts as an optional completion. All original
participants finish and the original simultaneous action closes before the
owner freezes every completion's exact destination IDs and snapshots. Only then
are added programs run in APNAP participant order.

Each completion restores its captured execution context and the owner's original
player/optional/source/rewrite scopes. Added programs retain their separately
captured replacement source and controller. They execute outside the original
action batch, so an additional mill forms a later event and cannot consume a
card before another participant's original mill. Ordinary single-player milling
keeps the existing receipt owner and does not defer into an absent outer loop.

The existing full ForPlayers game/context checkpoint owns preparation, every
original commit, and all completions. A pending choice or error rolls back cards,
life, replacement consumption, and queued notifications without clearing the
decision maker's prompt. Optional completions are frozen together, before an
earlier added program can move a later participant's result object.

Unrun public regressions assert original-then-additional milling groups, captured
participant versus replacement controller, a pending-choice rollback and replay,
a post-mutation error rollback, and later receipt LKI after an earlier addition
exiles its result. No source-level producer gap remains known for this bounded
seven-card cohort; the broad campaign validation gates are still required.
