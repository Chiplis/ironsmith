# Targeting participants and independent stack identities

UNVALIDATED source-only proposal. No build, compiler probe, or test execution.

Current status: all eight exact identities are proposed complete after bounded
independent source review through `62c077967`. The initial four-held split below
records why grammar alone was insufficient; the pre-cost transaction and complete
first-target history/continuation-owner corrections described later close those
specific blockers. Every authored scenario remains unrun; no measured recovery
is claimed.


Exact frozen stack07 identities and full Oracle fixtures are retained in
`fixtures/target_event_participants.json.fixture`.

## Earlier checkpoint: four complete, four held for semantic closure

- Partial player-only target event grammar: Amulet of Safekeeping and Dormant Gomazoa.
  Typed target player, targeting controller, and spell/ability kind are separate.
- Existing contextual source-alias normalization adjacency: Anthousa, Setessan
  Hero; Brigone, Soldier of Meletis; Cleon, Merry Champion; Rosnakht, Heir of
  Rohgahh. These use the existing spell-cast target relation, not a targeting
  transition or a new card-name production branch. Full-body scenarios cover
  land animation/cleanup, counter-removal draw activation, exile/play permission,
  token creation and actual battle-cry declarations.
- Skophos Maze-Warden now has typed “ability of [physical source filter]”
  grammar/matching, frozen event participants, and an authored actual paid
  Labyrinth activation/fight/pump scenario. It remains partial for pre-cost
  participant capture: tapping a source as a cost may change characteristics
  after targets were selected.
- Kira, Great Glass-Spinner is partial pending event-time first-target history
  plus transactional pre-cost observer capture. The present cast/activation
  producer matches after costs. Sacrificing Kira while paying for an already
  targeted creature's spell must retain the granted trigger from CR 601.2c.
  This observer-timing gap also applies to Amulet and Dormant; sacrificing
  either observer to pay a cost must not lose the already-occurring trigger.
  All four remain partial until the common transaction boundary closes.
  Fixing only the first-target grammar cannot earn full-card credit. The pending
  cast/activation and retained checkpoint must capture the original observer set,
  publish on successful completion, and discard it on cancellation/rollback.

## Independent identity boundary

BecomesTargetedEvent records an exact ability stack target ID independently of
its physical source. Spell and ability copies publish all distinct final targets,
including players. Copied abilities keep the original physical source and their
own new stack ID. Targeting-source tags, ward and stack-filter matchers select
that exact entry; sibling activations cannot substitute. Ability controller is
independent of later control changes to its source. Provisional copy-target
pruning is scoped to the exact entry, and changing one target slot to a target
already present does not produce a second becomes-targeted transition.

New core TriggerKind is appended, retaining existing serialized ordinals. Artifact
JSON round trips, player/controller/kind negatives, actual casts and activations,
copy targeting, departed-source countering and copied-entry identity scenarios
are authored but unrun.

## Pre-cost transaction closure (source-cleared, unrun)

PendingCast/PendingActivation now retain the target-trigger queue matched after
all target/distribution announcements and legality checks, before any payment.
The captured matcher sees an exact announced StackEntry and pre-cost participant
frames. An activated ability reserves its stack ID once, before matching, and
reuses it at finalization. Queues publish only on successful payment; the existing
whole-action checkpoint rolls back event history and pending queues on failure.
Crime remains a completed-cast/activation event. Native runtime savepoints clone
these receipts; the continuation-free JSON sync format explicitly requires replay
rather than dropping an in-progress targeted announcement.

The granted-trigger composition shortcut now delegates frequency-bearing heads
to the existing complete granted-line reader, which retains FirstTimeThisTurn.
Authored scenarios cover Kira sacrificed as a cost after choosing a protected
creature, previous targeting before the grant existed, duplicate target slots,
next-turn reset, and Dormant sacrificed to pay its triggering spell. Native
capture/rollback and WASM runtime-savepoint scenarios are included.

Kira remains partial pending a precise checkpoint policy/carrier for completed
first-target history: generic TurnHistory event records currently have no JSON
sync transport. This is separate from native savepoints, which preserve them.

## Completed first-target checkpoint carrier (source-cleared, unrun)

The continuation-free wire checkpoint now retains the exact set of objects that
completed an unqualified becomes-targeted event this turn. This is a separate
plain fact set, not reconstructed or fabricated prior events. The bare self-target
FirstTimeThisTurn matcher consults it; source-filtered first-event histories are
not widened to this simpler fact. Live native history remains authoritative for
new events. New object incarnations do not inherit an old marker, and the set
clears at the actual new turn. Grand Melee retains independent lane-local sets
and validates the focused carrier against the main one.

Current exports always carry an explicit (possibly empty) set. Older authoritative
wire checkpoints without this completeness carrier require accepted-transcript
replay. Provisional, still-unmatched copy targets also block continuation-free
export. Native savepoints remain lossless for these in-progress states. No
coverage status changes are made until this source boundary is reviewed.


The final continuation guard covers announced transactions, root and Grand Melee
trigger-ordering queues, effect-driven deferred entries, raw pending target
notifications, and root/all-marker stacks with a captured target event. Native
resolution rollback restores a stacked trigger while a choice is pending. The
scope never treats an executable ability omitted from JSON as an empty ability.
