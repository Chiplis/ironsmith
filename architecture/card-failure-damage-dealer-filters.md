# Current-turn damage-dealer filters

Status: source-authored, UNVALIDATED. Frozen bodies: Avenging Arrow,
Reciprocate, Restore the Peace and Red Guardian, Super-Soldier. The exact
oracle identities/texts are retained in `fixtures/damage_history_filter_bodies.json.fixture`.
No builds, compilation, compiler probes, tests, formatters or engine/corpus
execution were performed.

The frozen diagnostics reject these four bodies in destroy, exile and
return-all clause readers even though the underlying typed filters exist.
The destroy shape now distinguishes active damage dealers from passive damage
recipients; it consumes the complete current-turn suffix and preserves the
whole target subject. Exile accepts only a completely parsed recognized
history-qualified object target, retaining player scope, face-down and
source-duration parameters. Return-all admits the exact active dealer suffix
through its existing full filter reader. Unowned trailing qualifiers still
fail rather than falling back to an unrestricted move.

The existing runtime dealer predicates followed StableId aliases through zone
changes. That made a returned creature inherit its former incarnation's
damage history. All three current-turn dealer leaf predicates now match the
exact damage-source ObjectId; old public call signatures remain unchanged.
Control or other same-object changes do not reset history, while blink does.
Zero/prevented damage cannot qualify because the history requires a positive
completed DamageEvent. Combat-only queries retain their separate combat bit;
the four authored bodies accept ordinary noncombat damage too.

This correction operates on actual projected turn receipts and does not add a
second history store, a serialization carrier, a card-name rule or a rendered
text interpretation. Native savepoints already clone this history and each
new turn clears it. Current recovery uses native state and verified transcript
replay, not obsolete serialized gameplay checkpoints. Artifact 6, public audit
digest 3 and signed audit protocol 19 are unchanged.

Authored scenarios independently compile the direct and artifact routes,
validate the retained artifact, and actually announce/pay/resolve the complete
bodies. They distinguish dealer versus recipient, spell-controller-relative
damage, wrong players, zero damage, current creature types and owner hand
destinations, and real Flash casting on another player's turn with the full
entry trigger. Exact-incarnation/native-clone/turn-reset cases and announced
target blink/fizzle cases accompany strict unknown-tail negatives. Every
scenario remains unrun. Independent source review cleared 01cec8bf7; all nine
files integrate byte-identically at a2a06da2c. Four bodies are proposed and
unvalidated; no measured recovery is claimed.
