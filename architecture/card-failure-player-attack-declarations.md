# Declared attacking and directly attacked players

Status: **UNVALIDATED** source implementation. No builds, compilation, tests, or
compiler probes were run. Exact frozen identities, full Oracle programs, and
baseline diagnostics are in `fixtures/player_attack_declarations.json.fixture`.

## Bounded source proposal

Five complete printed-program candidates: Curse of Bounty, Curse of Disturbance,
Curse of Opulence, Curse of Verbosity, and Curse of Vitality. The sixth candidate,
Jolene, the Plunder Queen, has its attack trigger implemented here but remains
partial until the separately owned token-creation replacement and secondary-body
batch is integrated. Its exact full-card fixture remains unignored; the separate
first-clause scenario isolates this batch's event behavior without replacing the
full-program regression or counting it as complete.

## Event and reference contract

The real declaration commit produces one typed notification for each unique
attacking-player/directly-attacked-player pair. Both player identities and the
turn/combat phase identity are captured before attack triggers can resolve.
Declaring two creatures at one player does not create two player notifications.
Attacking a planeswalker or battle does not directly attack its controller or
protector. Entering already attacking does not declare an attack.

The typed trigger chooses grouping by attacker, by defender, or by pair. Thus a
passive `enchanted player is attacked` condition triggers once for that defender
even in shared-team combat with several attacking players. An active `a player
attacks one or more of your opponents` condition triggers once for each distinct
attacking player, even if that player attacks several qualifying opponents. The
attacker and defender filters remain independent. This producer supplements the
existing creature-level notifications, which retain their object identity and
remain available to the established creature-event shapes.
Serialized TriggerKind appends its variant, preserving previous ordinals.

Captured actor and defender roles have reserved typed player tags. Active
subjects export the attacking player as their anaphoric participant; passive
subjects export the defender. `That attacking player` uses the explicitly
captured combat actor even after its creatures leave. Runtime matching never
parses card text.

## Present-tense Curse rewards

The second sentence's `each opponent attacking that player` is a present-tense
relation, not the declaration's frozen set of actors. It evaluates current
controllers of creatures still attacking the original event defender in that
same turn and combat phase. Moving the Curse to a different player cannot change
that defender. Creatures removed from combat cease qualifying; creatures that
later enter attacking can qualify without generating another declaration
trigger. The resulting player set excludes the ability controller and every
teammate, not merely the controller's seat.

The relation is represented by the existing typed player-filter composition and
a reserved context-bound player set. A ForPlayers consumer without the required
declaration context fails closed rather than silently doing nothing. The
compiler retains either `does the same` actions or the explicit third-person
action, including Bounty's nonland/controller filter. The original controller's
first sentence stays first; qualifying opponents then perform the separate
second sentence through the existing simultaneous participant protocol.

## Rules and deferred regression target

[Comprehensive Rules, September 25, 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt):
508.3b defines `is attacked`; 508.3e distinguishes directly attacked players and
declarations from entering attacking; 508.6 distinguishes present-tense
`attacking` from declaration history.

Authored grammar negatives retain unknown players/tails and reject quantified
actors whose grouping is not modeled here. Direct/artifact runtime scenarios
cover all five full Curse bodies, both event roles, shared-team grouping and
teammate exclusion, two distinct defenders, attacker departure, Aura movement,
later attacking entrants, planeswalkers/battles, and frozen Jolene actors. Gold's
actual sacrifice-for-mana activation is retained as a secondary-body regression.
The full six-card serialization fixture includes Jolene's pending integration.

Deferred command: `cargo test -p ironsmith-compiler-runtime --test player_attack_declarations`.
No command in that validation category has been executed for this batch.
