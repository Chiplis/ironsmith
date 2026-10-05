# Exact Aura source movement after an enchanted permanent leaves

UNVALIDATED source follow-up. No compiler, build, test or corpus execution.
This closes the source-review prerequisite held for Ghoulish Impetus in the
complete-anthem-tail fixture, without claiming executed correctness.

The official September 25, 2026 [Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt),
400.7e–f, distinguish a triggering zone-change arrival from an Aura's subsequent
unattached-Aura state-based move. Both exceptions refer to a particular new
object. An additional exile/return creates another object and cannot be followed
merely because it is the same physical card.

`resolve_source_object_id` already uses explicit result IDs when the source
moves in the triggering event. The follow-up replaces its broad stable-card
fallback for a different object's zone-change event with a bounded 400.7f proof:

- The source snapshot was an Aura on the battlefield attached to a departing
  event object, or the event retained that exact attached-source snapshot.
- Immutable actual turn receipts show that exact old Aura object moving from
  battlefield to graveyard via a state-based action.
- Only the result object recorded by that receipt is accepted, with the same
  owner and physical identity and still in that graveyard.

A later move makes the recorded result unavailable; an unrelated Aura, an
unrelated cause or a replacement that sends it elsewhere cannot authorize a
new return. Copied Auras may lose their Aura characteristics in the graveyard;
the source snapshot proves the relevant earlier status. Existing same-event
400.7e behavior and the engine's in-resolution movement permission remain.
The delayed-trigger scheduler already pins the resolved current source object,
so a later move after registration also remains a different object.

No persistent or wire state is added: the proof uses retained completed zone
receipts and source snapshots. Four source-helper unit controls and the existing
full-card direct/artifact normal-return plus extra-zone-change integration
scenario are authored and unrun. Full replay, native/peer execution, source
movement regressions and retained-checkpoint validation remain mandatory.
