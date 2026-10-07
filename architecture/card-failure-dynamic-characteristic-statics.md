# Dynamic static characteristic definitions

UNVALIDATED source proposal on `7e84aa867a8625083be3d79858e80686cbc152f2`.
No builds, compiler or engine probes, tests, formatting, or generated artifacts
have run. No ledger counts or publication changes are included.

The exact frozen bodies and metadata are retained in
`fixtures/dynamic_characteristic_statics.json.fixture`: Aettir and Priwen,
Angry Mob, Duelist of the Mind, and Roiling Horror. Duelist's draw expression
already has a shared lexical owner; the proposal must close its runtime
semantics rather than merely claim that preexisting grammar. Roiling was
previously partial because its dynamic characteristic expression was missing.

These four are proposed-complete source bodies, not measured recoveries. The
static X/X setting binds the entire authored definition through the
existing typed controller-state value capability and layer-7b setting owner.
Source, attached recipient, and filtered-set subjects preserve their existing
typed roles. Resolution targets, unbound X, event receipts and unconsumed
instruction tails are rejected. This is a live quantity, not a frozen cast-X.

Timed source definitions use two independent turn predicates and layer 7b.
They are not CDAs and do not acquire all-zone functionality. A proper source
possessive is required; equipped/enchantment recipients cannot be rebound to
the source by a suffix match. Roiling's opponent maximum reuses the existing
MaximumLifeTotal(Opponent) plus signed Add/Scaled expression, keeping global
maxima, source life, opponents, teammates, and negative results distinct.

The new subject and value readers require complete token spans. Explicit
source/attachment and complete object-filter grammars cannot recover a
trailing subject after unknown leading words. Unknown instructions, mana
symbols embedded in ordinary scalar prose, malformed timing delimiters, and
foreign recipients cannot fall through into an unconditional source CDA.

## Checked runtime boundaries

Single-axis and paired settings in both layer 7a and layer 7b now use the
existing checked characteristic scalar boundary on the manager, direct and
batch calculation routes. Overflow and unavailable required player evidence
are retained as errors, so checked reads cannot publish provisional numbers.
Scalar player reads return an error when no single player is available;
the opponent maximum still has its genuine empty-aggregate identity.

Duelist's typed draw-history count and the sibling maximum-per-player count
sum retained card-vector lengths in checked i64 arithmetic until the existing
scalar boundary. This avoids both u32 summation overflow and an early signed
cast. Representable arithmetic over larger intermediate counts remains exact.
Missing required maximum-player selection is an error, while an actual empty
draw history for a valid player is zero. Legacy u32 history accessors used by
non-numeric callers retain their existing API.

The crime target helper now uses live opponent relationships, including teams.
It checks current control for battlefield permanents, spells and stack
abilities, ownership for graveyard cards, and excludes other zones. Pending
crime loot keeps the existing once-per-turn and optional-draw result gates.

## Authored, unrun regression coverage

- Four exact metadata-bearing full bodies compile directly and through JSON
  artifact restoration, with all other abilities and functional zones retained.
  The normal tools target independently checks strict complete-card admission.
- Aettir pays the real equip cost, excludes opponent targets, moves between
  hosts, follows the Equipment controller's live life total independently of
  its owner/host, and obeys modifiers, counters and a later layer-7b setting.
  Phasing, ability removal, departure and a new unattached incarnation remove
  or restore only the appropriate contribution.
- Angry Mob retains both timing predicates, excludes your own/teammate and
  off-battlefield Swamps, observes phasing/controller changes, applies counters
  after base assignment, and returns to its printed 2/2 in hand.
- Roiling has live all-zone signed statistics and checked extreme differences.
  Real positive-X Suspend pays its full cost and rejects zero without mutation.
  Upkeep and last-counter handling retain the free cast, haste and targeted
  life body; multiple removed counters produce separate triggers whose
  controller survives departure and a differently controlled new source.
- Duelist's real casts distinguish teammate targets from crimes. Its optional
  draw/discard, cap consumption on decline, turn reset, current controller,
  off-battlefield CDA, flying and vigilance are exercised.
- Engine tests cover all checked setter routes and single/pair axes, required
  player errors, draw totals above u32, wide overflow, arithmetic cancellation,
  staged-event publication without double counting, and actual crime zones,
  teams, ownership and independent stack-ability control.

## Compatibility

No serialized variants, carrier fields or stored scalar widths are added or changed.
TurnEventRecord, TurnEventRecords, CardsDrawnEvent, native savepoints, snapshots
and public-audit encodings are unchanged. Existing event card vectors feed new
local aggregations only. Existing Value, static payload, player-filter, layer
and error variants carry the entire proposal; no compiler/artifact/runtime
transport substitution or schema-version change is required.

The later, separately gated ordinal correction adds one native-only
`Option<TurnEventRecords>` field to non-serialized TurnHistory. Full-state native
savepoint and lane clones retain that owner automatically; no serialized
carrier or numeric width changes. Its presence, ordering, promotion and unknown
history contract are documented in `card-failure-draw-ordinal-history.md`.

The frozen inventory scan found no further unaddressed static X/X settings
sharing this owner. Linked-exile statistics (Sutured Ghoul, Drach'Nyen), paid
entry statistics (Minion of the Wastes), token result quantities and unrelated
body tails remain separate. Existing proposed Malignus, Scourge, and dynamic
activated P/T identities are not new coverage.

## Bounded prior-owner correction: dynamic draw modifiers

Knowledge Is Power was already proposed under signed controller-state dynamic
anthems. Its actual owner converts the typed draw quantity to
ModifyPowerToughnessValue in layer 7c. Both direct/batch and manager routes now
use the same checked scalar boundary as settings before applying their existing
checked signed P/T addition. This is a correction to that prior proposal, not
another new identity. A generated draw-history anthem regression carries a
retained count above u32 through that exact static-to-modification owner, rejects
publication, then recovers after a real turn boundary.

The You and Specific player forms of the draw count now require an available
player before treating an empty retained history as zero. Other role/filter
bindings retain their existing owner and are not newly claimed by this guard.
Authored tests distinguish missing source-controller evidence from valid empty
history. Dynamic modifier tests cover ordinary negative values on all three
calculation routes and a real effect sequence that gains life before registering
an overflowing or missing-evidence modifier. Failure must roll back life,
history, registered effects, context receipts and the checked cache; a later
valid transaction must still succeed.

Cerebral Vortex and Molten Psyche already route their per-player quantities
through the same repaired numeric draw leaf. Their full-card source scenarios
remain in the prior-damage and zone-shuffle suites. The nine existing dynamic
base-P/T bodies evaluate their RHS fallibly at resolution and install Fixed
values, so they do not share the newly found live layer-7b failure path. This
audit does not establish a full mathematical audit of unrelated history APIs.
