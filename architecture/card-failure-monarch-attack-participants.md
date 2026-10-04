# Monarch-qualified attack participants (adjacent source review)

Status: UNVALIDATED. Exact full fixtures for Emberwilde Captain and The Spear
of Bashenga are in `fixtures/monarch_attack_participants.json.fixture`.
Both are source-complete proposals after independent full-body review of
ecb678428 and the typed attachment fixture correction.
No compiler, build, test or CLI execution has run.

These frozen failures predate integrated player-declaration and contextual
quantity/reference work. No new event recognizer or broad IteratedPlayer
permission is added here. The existing typed routes are:

- Captain: `PlayerAttackDeclaration` with pair grouping, opponent actor and
  controller defender. Its event-time monarch predicate wraps that declaration;
  the body uses the saved actor tag for its damage recipient and current hand
  quantity. Two attackers from one opponent create one relevant declaration.
- Spear: the equipped-object `Attacks` arm is restricted to a directly attacked
  player; an event-time `PlayerIsMonarch(Defending)` predicate qualifies it.
  The reference environment infers the event's defending player. The body's
  tapped nonland target remains constrained to that recorded player even after
  the designation or attachment changes. Attacking a planeswalker controlled
  by the monarch is excluded.

Authored direct/artifact scenarios keep all secondary bodies: actual entry
monarch instructions (including Spear's no-monarch intervening condition),
paid equip, +2/+2, vigilance and exact destruction target. Multiplayer cases
separate attack actor, defender and later monarch; current hand size is read
at resolution, while event-time qualification does not drift or trigger later.

The holder-change cohort (Custodi Lich, Garland, Knights of the Black Rose)
is a separate production boundary. Its setter/departure/history work is not
claimed here. Fealty to the Realm and the three Courts retain their unrelated
static/copy/target-set blockers.

Deferred focused command: cargo test -p ironsmith-compiler-runtime
--test monarch_attack_participants. Do not run during source-first campaign.
