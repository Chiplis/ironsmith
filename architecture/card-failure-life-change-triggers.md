# Filtered life-change events and temporary listeners

Status: **UNVALIDATED** source implementation. No compilation, tests, or compiler
probes were run. The exact frozen identities and complete Oracle texts are in
`fixtures/life_change_triggers.json.fixture`.

## Five complete printed-program proposals

Kavu Predator; Punishing Fire; Moonstone Harbinger; Wax-Wane Witness; Vizkopa
Guildmage. The broader life lane's six per-unit quantity candidates (False Cure,
Cradle of Vitality, Lich's Tomb, Lich's Mastery, Oath of Lim-Dûl, Transcendence)
remain separate and are not counted by this commit.

The event grammar consumes a complete singular player subject, life action, and
optional turn qualification. `You gain or lose life during your turn` becomes
two typed event arms with the same turn guard and one enclosing triggered
ability. The gain and loss arms therefore share any once-each-turn trigger
limit. Unknown subject/tail text is not consumed as a broader life event.

A new serialized PlayerGainsLife variant appends to TriggerKind; old YouGainLife
forms and causal source filters remain available. Both affected-player binding
and the numeric event-amount capability are retained through references and
lowering. A new appended DelayedTriggerSpec::LifeChanged variant carries player,
action direction, and optional turn qualification. Existing temporary-listener
ownership, captured controller, repeatability, and end-of-turn expiration remain
responsible for Vizkopa's activated ability. The delayed registration survives
its source's departure without changing its controller.

## Actual events and player separation

Matching consumes the existing real LifeGainEvent/LifeLossEvent producers. Zero
life changes do not trigger. The affected player's actual post-replacement
amount supplies Kavu's counters; the amount is not recomputed from their later
life total. Each separately affected opponent generates its own trigger even
when the gains share one simultaneous instruction. Gaining life is distinct from
losing life, and payment uses the existing real loss notification.

The generic matcher uses the typed player-filter evaluator. The existing loss
matcher previously treated every other seat as an opponent and accepted unknown
filters by default; it now uses that same evaluator, preserving teams, captured
players, and fail-closed unknown references. A `during their turn` qualifier is
bound to the event's affected player, not the ability's controller.

Punishing Fire reuses source-derived graveyard functional zones and exact
non-zone-event source identity. Its paid return cannot follow a different
incarnation that has independently left and reentered the graveyard.

## Deferred scenarios

`crates/ironsmith-compiler-runtime/tests/life_change_triggers.rs` contains exact
full-card direct and artifact round trips, replaced/zero/forbidden gains,
multiplayer/team separation, simultaneous independent recipients, live source
controller changes, both gain/loss arms and the shared limit, payment-generated
loss, cleanup and subsequent turns, graveyard-only optional paid returns, exact
zone identity, repeated paid Vizkopa registrations, controller retention after
source departure, listener expiration, real lifelink from its other activation,
and Punishing Fire's real paid damage spell. Public grammar negatives keep
unrecognized qualifications rejected. All regressions are authored and unrun.

Deferred command: `cargo test -p ironsmith-compiler-runtime --test life_change_triggers`.
