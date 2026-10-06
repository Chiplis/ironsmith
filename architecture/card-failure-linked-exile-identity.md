# Executable linked-exile ownership

Status: source-authored, UNVALIDATED. No build, compiler probe, formatter, test,
corpus execution, or artifact regeneration was run. Bishop of Binding and the
neighboring linked-card candidates remain partial pending independent review
and the deferred complete-card execution pass. Static linked characteristics
are outside this checkpoint.

Recovery note: the prior workspace became unavailable after source review of
93a0f55cc8212e79f15049ea49ab839ac46ab532. This packet reconstructs that design
from retained source and mutation receipts on base
69a946ec767deda59927d63f08dd17fabff470f3. It does not claim the old commit or tree
identity. The missing f28b959d prerequisite and its small metadata scenarios
were rebuilt from the final typed-definition design. The duration correction
and regression were imported exactly from stage77 commit
56e440a9b5216774e7e8f59354d0ad7e4c72597a. The complete reconstructed diff requires
fresh independent source review.

The scalar power/toughness/mana-value reader formerly used the union of every
card exiled by a source. A borrowed plain exile could therefore increase Bishop
of Binding's bonus. The duration return primitive separately used that union;
the imported duration correction restricts returns to its exact recorded
return-zone members.

Executable programs now carry an optional explicit pair descriptor. Its
immutable definition stamp is a SHA-256 of serialized typed abilities, never a
physical host CardId. Core CardId values are caller-local graph identifiers:
the front end can reuse raw 1 and the WASM compiler can rewrite them. A stamp
travels with copied rules text through model mapping and artifact decoding.
Compiler binding runs after the final builder and reference finalizers. It
requires exactly two executable ability bodies and no unclassified static or
nonmana/dynamic/alternative activation-cost scope: one recognized exile producer,
specifically a compatible source-leaves ExileUntil, and one distinct executable
scalar SourceExiled consumer.
Multiple recognized producers and unsupported/unknown pairing remain unbound.
Names, labels and Debug output do not decide semantic compatibility. Native
programs must explicitly supply their own definition stamp and pair descriptor.

Runtime membership is keyed by the exact host ObjectId, definition pair, and
ability acquisition. Printed member slots normalize into one printed scope;
effect acquisitions retain their complete AbilityEffectOrigin; borrowed
acquisitions retain their granting effect, exact donor ObjectId and recursive
donor origin. Temporary, counter and level scopes retain their occurrence and
parent identities. Only the pair member's own slot is normalized.

Ordinary and mana activation capture the existing origin before costs. The
choice-free activation path preserves the full program and its owner as well.
Trigger admission captures the calculated origin or exact historical snapshot
origin. Stack copies, immediate triggered mana, ExecutionContext checkpoints,
delayed registrations, and reflexive continuation state retain this owner.
Both characteristic calculation paths now preserve Borrowed origins when
copying triggered abilities. Source/victim/donor zone changes cannot adopt old
incarnation membership. Scalar evaluation still reads current characteristics
of every live member and sums checked wide values; face-down characteristics
therefore remain characteristic-free.

GameState native copy-on-write savepoints retain pair membership and pending
owners. A source-only exile-map import explicitly invalidates pair completeness
and the reader returns IncompleteEvidence until a full native state or genesis
replay is restored. Public claim snapshots intentionally omit executable
abilities and origins; they are not gameplay recovery checkpoints. A snapshot
with retained abilities but missing origins cannot authorize either member of a
linked pair: program entry fails before a producer mutates the game.
Existing verified recovery always replays from accepted genesis, with native
savepoints as the local branch mechanism. No partial wire encoding of an
acquisition namespace is introduced.

Authored scenarios cover frozen full Bishop bodies in direct/artifact paths,
an actually borrowed plain-exile activation, copied ETB abilities with distinct
victims, independent borrowed pairs and donor incarnations, victim/source blink,
face-down characteristics, a pending-reader native checkpoint, and explicit
unknown-native/source-only-import failure. Existing live characteristic and
incomplete-discovery scenarios now declare explicit native pairing. Native
activation scenarios cover immediate, pending, special-action and choice-free
routes. All scenarios remain unrun.
