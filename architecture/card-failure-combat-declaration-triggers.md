# Combat-declaration participants

UNVALIDATED source work. No builds, compiler probes or tests were executed.

Four exact frozen unsupported-trigger identities are retained with their full
metadata and bodies in `fixtures/combat_declaration_triggers.json.fixture`:
Hezrou // Demonic Stench, Neyith of the Dire Hunt, Righteous Indignation, and
Yuriko, Blade of the Mighty. All four are proposed complete after independent source review through
`2b890badf` with the final Fight prerequisite. All remain runtime-unvalidated. Icingdeath and Seraphic Greatsword are separate, unclaimed identities.

## Typed events and grammar

Three appended core/semantic variants add grouped becomes-blocked, grouped
object-qualified keyword actions, and attacks-a-player-alone. Existing
`BlocksObject` already models the directional blocker/blocked pair. Its grammar
now accepts ordinary filtered blockers and color unions inside the blocked
participant without interpreting that union as separate event arms. A complete
shared-subject owner preserves Neyith's one-or-more scope on both fight and
becomes-blocked alternatives. Unknown participants are rejected.

Becomes-blocked filters read the retained attacker snapshot. Exact native
subscription metadata forwards grouping keys through the alternative trigger;
one fight action and one completed blocker declaration each have their own
one-or-more occurrence. The multiplayer blocker owner already commits every
player's declarations before one event publication. No duplicate producer was
introduced. Fight depends on `fc20beb94`, `dcd2e1bcc`, and the final checked-owner
correction `2efce3710` (initial local prerequisite copies
`f0914cc12`/`251238e23`): each fighter has a unique notice provenance, one common
fight batch, and an exact pre-fight snapshot captured before replacement-added
programs.

CR506.6 directly attacked players are distinct from their planeswalkers and
Battles. The alone matcher counts the event's retained declaration, including
any other controller's declared attacker of that player; it neither reads later
combat nor counts objects merely put onto the battlefield attacking. Missing
per-defender declaration evidence cannot be replaced by a global attacker count.

Righteous's definite “the blocking creature” retains its exact Blocking event
identity, without the redundant live blocking flag. Leaving combat does not
change the referent; blinking cannot substitute the new object. Hezrou's
unqualified “each blocking creature” remains a current-state set.

## Full-body source and scenario gates

Authored direct/artifact scenarios exercise real multiplayer attack/block
declarations, per-pair multiblocking, snapshot/controller qualification, leaving
combat versus blinking, union fight grouping and separate operations. They also
cover Neyith's actual optional hybrid payment, current-power doubling, retained
counters and must-block-if-able restriction, and Yuriko's combat-only spell and
nonmana-ability prohibition while allowing mana abilities.

Both Hezrou Adventure faces compile and serialize independently with linked-face
metadata. A real Adventure cast after combat uses the completed blocked-this-turn
history, moves to exile, then permits the normal creature cast. The card is not
counted by its front trigger alone. All authored scenarios remain unrun in
`combat_declaration_triggers.rs`, the native matcher tests and grammar tests.

The runtime contract inventory describes the actual new event kinds and their
existing reference evidence. It does not suppress diagnostics or claim an
unsupported body/producer as complete. Generic unreproduced history/program
checkpoint guards continue to provide the native/replay boundary.

### Definite participant type continuity

The exact untargeted phrase “the blocking creature” lowers directly to its
recorded Blocking identity. Pump lowering does not impose a fresh Creature test
on that definite event reference (CR608.2k); ordinary targets keep their type
legality. An authored direct/artifact regression animates a land, blocks, ends
the animation before the trigger resolves, and reanimates it afterward to check
that the +1/+1 continuous effect was retained. Blink remains a separate identity.
