# Actor-scoped and mixed-recipient counter replacements

UNVALIDATED source work. Three proposed whole-card identities: Doc Samson,
Super Psychiatrist; Lae'zel, Vlaakith's Champion; and Loading Zone. Complete
frozen metadata/text and Oracle IDs are in
`fixtures/scoped_counter_replacements.json.fixture`. No build, compilation,
test, formatter or corpus replay was run. `git diff --check` is the only
executed code check. Measured recovery remains 40 and unresolved unique cards
remain 3,193.

## One existing counter-event driver

Two complete named grammar productions extend the existing counter-replacement
reader: actor-qualified additions of each counter kind, and filtered passive
multiplication of every kind. The actor addition retains an object filter, an
independent optional player recipient, the placing actor and the scalar addition
in an appended core payload. The native factory reuses the existing
`AddCountersPlacementReplacement` action/matcher, with actor and combined
permanent/player scope explicitly supplied. Legacy factories retain their
previous defaults; no old wire variant fields or ordinal positions change.

Doc Samson requires both a permanent controlled by the host's controller and
that controller as the placing actor. Lae'zel narrows the permanent arm to
creatures/planeswalkers and adds that controller as a player recipient. The
actor comes from captured event causation, not the current controller or even
continued existence of the effect's source. Host control/presence is still live.
The rules are not effect-only: applicable counter placements made as costs
remain covered. Zero placements do not acquire a new counter.

Loading Zone uses the existing multiplier with a proper disjunction of Creature
card type, Spacecraft subtype and Planet subtype. Its shared controller scope
qualifies every arm, and there is no placing-actor or effect-only restriction.
The pre-existing filter domain-union parser preserves mixed type/subtype arms
instead of conjoining them. The existing Warp alternative-cast representation
and stack-resolution exile/later-turn permission path are retained unchanged.

Entry counter processing uses the existing prospective-entry matcher and waits
for the entry contributions to be prepared. The additive action coalesces
multiple contributions of the same counter kind before adding once to each
positive kind. A global static on an entering host does not modify that host's
own entry. Existing replacement identity, ordering, prevention-of-counter
placement and lifetime handling remain authoritative.

The official [Marvel Super Heroes release notes](https://magic.wizards.com/en/news/feature/marvel-super-heroes-release-notes)
confirm that Doc Samson modifies entry counters and that multiple copies each
add one. The [Edge of Eternities release notes](https://magic.wizards.com/en/news/feature/edge-of-eternities-release-notes)
describe Loading Zone and its Warp ability. Source-supported secondary paths
also include Doc Samson's power-valued mana ability and the existing matching
Background commander construction rule for Lae'zel; the full fixtures retain
those bodies rather than reduced replacement-only card text.

## Deferred evidence and exclusions

Two grammar scenarios and six direct/restored-artifact runtime scenarios are
authored and unrun. They cover full bodies, actor versus recipient distinctions,
permanent/player scope, both causes, zero quantities, departed action source,
live host control/phase-out/leave, Creature/Spacecraft/Planet positive and
negative controls, and multi-kind entry contributions.

Zabaz remains partial: EventCause currently identifies source/controller and a
resolving spell, but not which triggered ability occurrence placed the counters.
Matching a source merely because it has modular would incorrectly include its
other abilities. A real captured modular-trigger identity is required before
that card can count. Mowu was already proposed by another batch and is not
recounted here. This change makes no new measured recovery claim.
