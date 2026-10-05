# Toughness-based combat assignment

Status: **UNVALIDATED source proposal** on base `4121a105`. No builds, compiler
probes or tests have run. Exact frozen sources are in
`fixtures/toughness_damage_assignment.json.fixture`.

## Membership

Eleven proposed-complete sources:

- Ancient Lumberknot
- Arcades, the Strategist
- Baldin, Century Herdmaster
- Bark of Doran
- Bedrock Tortoise
- Bill the Pony
- Ghalta the Immovable
- Plagon, Lord of the Beach
- Solid Footing
- The Kingpin of Crime
- Walking Bulwark

The twelfth reserved source, **Tapestry Warden**, remains partial. Its combat
assignment line uses the same new reader, but its separate station-using-
toughness payment has no proven executable primitive in this branch. Its exact
full source stays in the fixture with an explicit partial status and is excluded
from full-card strict-compile assertions and the proposed-complete count.

All twelve were failures in stack07. Arcades reported the missing `rather-than`
semantic marker; the others failed the static/conditional or generic `assign`
clause. Expected after integration for the eleven complete proposals: strict
metadata-bearing compilation without loss, exact-target/duration semantics,
and both direct and restored-artifact execution. This is not measured coverage.

## Shared execution boundary

No new runtime marker, serialized enum or numeric value is introduced. The
existing `ThisCreatureAssignsCombatDamageUsingToughness` ability is consumed
from calculated characteristics by both `ToughnessCombatDamageSources` in the
unblocked-player fast path and `creature_assigns_combat_damage_using_toughness`
in ordinary combat assignment. Both read the receiver's current toughness
when damage is assigned. They do not rewrite actual power or change ordinary
noncombat damage.

The new complete shared suffix reader accepts mandatory singular/plural
assignment and the complete coordinated no-defender permission. It rejects
unconsumed tails and does not silently turn an optional assignment choice into
a mandatory rule.

Persistent static grants use ordinary `GrantStaticAbility` with the typed
subject filter and condition. The already-existing per-candidate
`PowerToughnessRelation::ToughnessGreaterThanPower` compares each receiver's own
current axes. Static membership changes as characteristics/controller change.
Attached possessive conditions use `AttachedToSourceMatches` and bind a following
`it` to this source's equipped/enchanted object, never to the grantor itself.

Resolving effects use the existing target grant or `GrantAbilitiesAll` with
`lock_filter_at_resolution: true`. Their recipient set is fixed at resolution,
while the damage stat remains live. Later arrivals are excluded, and a recipient
whose power later becomes greater than its toughness keeps the granted rule
until the authored duration expires. A resolving optional life payment remains
the ordinary `May`/result gate around the grant.

Arcades' full two-predicate line cannot be recovered as a plain no-defender
suffix: the plain permission reader now excludes an earlier `assign(s)` verb.
The complete assignment reader preserves both abilities with the same defender
filter. Compiler-model labels now retain the full `rather than its power`
meaning already present in the engine ability descriptions.

## Secondary bodies reviewed

- Ghalta's cost modifier already binds its where-X expression to typed
  `GreatestToughness`; the cost test uses actual reduced payment and an opposing
  high-toughness creature that must not count. Its defender attack permission
  remains executable.
- Baldin's target bound uses the existing grammar-common cardinal parser and
  `core::parse_cardinal_words`, which explicitly accepts `one hundred`. No new
  number primitive is necessary. His targeted X pump uses the current hand
  count on resolution and lasts only to cleanup.
- Arcades' defender-entry draw and both combat permissions remain separate
  effects. Bedrock retains its own-turn hexproof rule.
- Bill creates two Food tokens, then actually sacrifices one to pay the grant's
  cost. Plagon's entry draw includes itself and only current qualifying
  creatures. Bark retains its independent +0/+1 modifier and actual equip.
- Solid Footing tracks the attached object's current vigilance. Kingpin retains
  extort and the optional two-life attack payment. Walking Bulwark retains its
  sorcery-only activation and all three temporary grants to the same target.

## Authored, unrun regressions

Six grammar properties pin static filters, Arcades unique whole-line registry
ownership without loss, attachment antecedents, optional/duration exclusions,
locked resolving sets, and actual targeted grants. Twelve normal compiler-
runtime tests include the eleven-source direct/JSON aggregate and gameplay
scenarios for both combat paths, live P/T changes, mandatory use of lower
toughness, source/controller/attachment changes, public cleanup/turn advance,
real Food/equip/hybrid payments, Baldin's 0–100 target bound
and response-time hand count, Ghalta's real reduced white payment, optional
Kingpin payment and extort, and Tapestry's explicitly partial combat subset.
The ordinary tools integration target checks exactly the eleven complete
metadata-bearing payloads. No manual continuous-cache refresh is used in the
new runtime suite.

## Remaining constraints

Tapestry station payment is intentionally unresolved. Other optional
power-versus-toughness assignment choices are not admitted by this mandatory
suffix. Existing integer/resource and simultaneous-damage constraints retain
their independent campaign status; this source proposal does not close them.
