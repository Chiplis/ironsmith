# Target-player and controller reference ownership

Status: **UNVALIDATED**. No builds, compilation, or tests were run. Source base
47d5644c5; exact identities are in
`fixtures/target_player_reference_bindings.json.fixture`. The frozen diagnostic
baseline remains stack07 at bc9e56e2.

## Proposed scope

Thirteen proposed full bodies, subject to source review: Barroom Brawl, Breaking
of the Fellowship, Cellar Door, Early Harvest, Ember Gale, Head Games,
Inquisition, Isildur's Fateful Strike, Jester's Mask, Meletis Charlatan, Mutiny,
Roiling Terrain, Volrath's Dungeon.

Two partials remain uncounted:
- Keeper of the Dead: its announcement-time target-player predicate must compare
  creature-card counts across two graveyards. The old parser instead constructed
  an object target in a graveyard. This patch does not claim that predicate.
- Alpha Brawl: the first damage instruction now binds the exact targeted source's
  controller and excludes that source from its recipient set. Its second
  instruction still needs independent review of the retained first recipient set,
  simultaneous distinct damage sources and each source's power. No full-card
  claim is made from the shared binding repair alone.

## Reusable boundaries

- An explicitly targeted grammatical subject establishes a local participant
  scope before its action's filters are lowered. It also contributes a real
  target declaration, including when the affected object set is empty. This does
  not supply a caster/default participant to an orphan reference.
- A damage-source target exports an exact target receipt before the recipient
  filter is resolved. Dependent controller/owner references and `another` use that
  same receipt during announcement and resolution; the source is not inferred
  from whichever target happens to be visited last.
- Restrictions scoped by a player target and fight instructions scoped by a named
  adjacent seat export that player role. An empty affected set cannot replace the
  player with a nonexistent result object's owner.
- Hand-size scalars resolve the same contextual player references as filtered
  counts. Qualified hand nouns use object filters instead of erasing their color
  or type into the whole-hand scalar. The tested subject of a singular hand-size
  comparison supplies the consequent's `they/their`, retaining the authored
  comparison boundary for `the difference`.
- Controller-of-target copy subjects declare the stack object explicitly and
  share its exact receipt with copy ownership and optional retargeting.
- Bottom-of-library sources use the existing bottom-only choice primitive before
  the real zone movement. Counted hand moves retain their grammatical actor as
  chooser, separately from the spell controller and destination owner.

## Authored, unrun verification

`cargo test -p ironsmith-compiler-runtime --test target_player_reference_bindings -- --nocapture`

The target exercises strict full payloads and artifact round trips, paid casts and
activations, three-player controller/owner separation, two dependent targets,
control changes and blink/fizzle, copied spell ownership, conditional hand-size
exile, whole-hand search count/chooser/library ownership, positional bottom-card
movement, tapped entry and self-sacrifice, discard versus hand-choice ownership,
and explicit orphan rejection.

## Deferred X payment constraints

The seven exact partial identities from `consumer_mana_spending.json.fixture`
remain partial: Atalya, Samite Master; Consume Spirit; Crimson Hellkite; Crypt
Rats; Drain Life; Emblazoned Golem; Soul Burn. The three source-producer spending
rules were reviewed independently; they do not implement these X constraints.

A correct next design must preserve the generic X portion through reductions,
constant generic and taxes, then prove actual-paid color allocation against that
portion. As-though spending cannot change actual colors. Convoke/delve may reduce
unpaid X without spending mana. Assist instead spends another player's mana and
requires a joint, locked allocation between helper and caster; blindly copying a
whole-cost restriction either forbids legal payments or admits illegal ones.
Emblazoned Golem caps each actual color on X at one. Soul Burn additionally needs
chosen black-on-X allocation as resolution evidence, separate from announced X
and total black mana spent; both Soul Burn and Drain Life have independent capped
actual-damage life-gain bodies. These are preserved design requirements, not
source-coverage claims.

## Review correction: announcement relation versus runtime receipt

The first draft attempted to read a newly generated source tag while announcing
the second target. Ordinary TargetOnly tags exist only during resolution, so that
was not an announcement-time receipt. The correction keeps the typed shared-player
relationship for the two adjacent explicit target slots, composing it with the
distinct-object constraint. A polynomial singular-pair feasibility check rejects
boards with only one creature per opponent. Runtime/mass recipients retain the
source tag once the declaration has actually executed.

Resolution rechecks the relation using the immediately preceding declared slot's
current controller, or that exact ObjectId's departure LKI if gone. It never
follows a stable-ID successor. The first slot itself remains illegal and produces
no runtime source tag; therefore ExecuteWithSource cannot deal damage from its
LKI. A surviving second slot permits the independent Ring instruction to resolve.
An authored source-only departure/blink scenario isolates this from all-targets-
illegal fizzle. No tests were run.
