# Targeting participants and independent stack identities

UNVALIDATED source-only proposal. No build, compiler probe, or test execution.

Exact frozen stack07 identities and full Oracle fixtures are retained in
`fixtures/target_event_participants.json.fixture`.

## Corrected bounded proposal: four complete, four still partial

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
