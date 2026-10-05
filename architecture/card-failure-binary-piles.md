# Binary card pile programs

Source-authored implementation and regression scenarios; no build, formatter,
compiler probe, test, or corpus run was performed under the campaign policy.

## Bounded subset

The exact frozen bodies of Atris, Oracle of Half-Truths; Curator of Destinies;
Epiphany at the Drownyard; Fortune's Favor; Riddles in the Dark; Steam Augury;
and Truth or Tale use the finite library-pool program in
`grammar/effects/divvy_shapes/binary_program.rs`. Counts, the private viewer and
partitioner, public/private exposure, whole-pile disposition, and selecting one
card from a chosen pile are independent typed facts. Remaining sentences must
compile as complete instructions.

The program uses existing look/reveal, tagged capture, subset choice, player
choice, resolving mode choice, and multi-card movement primitives. Empty piles
remain legal choices. Both sides of the partition are captured before any move.
Tagged pool membership remains visible to the existing hidden-zone chooser
protocol; a second exact-object constraint retains each original incarnation.
The coordinated hand/graveyard destination is one native move with disjoint
tagged destinations. Its union, source snapshots and destination memberships
are frozen by `PreparedMoveSelection`; the existing native move proposal,
original commit, draw continuation and completion owners process that batch.
All original arrivals precede any added replacement program. Truth or Tale
captures the remainder of the entire revealed set before attempting the chosen
card's hand movement, and orders that full remainder on the library bottom.

Targeted opponents are declared by the private look's authored subject. The
partition refers to that same target without declaring another. Untargeted
opponents are selected by the controller during resolution after partitioning.

The existing WASM dispatcher discovers private/public view requirements from
ReplayDecisionMaker's view callbacks and authenticates openings. The shared
top-card look now hydrates only already-verified replay openings for its finite
top set and entitled viewers, preserves the view callbacks for that discovery,
and rolls back tags/hydration on a pending view. Public completion uses the
existing owner-authenticated exact-card reveal and rejects remaining opaque
identities. It does not add a new opening protocol or inspect the rest of a
library.

## Evidence authored for later execution

`binary_card_piles.rs` independently compiles every proposed complete body,
serializes and validates its artifact, then repeats native announcement and
resolution through each route. It covers every legal partition size with both
pile choices, empty/short/full/long libraries, multiplayer target versus chooser
roles, private/public views, X=0 and other X values, bottom ordering, untouched
cards, pending partition replay, prevented hand movement, and replacement-added
programs observing both completed original destinations. Local grammar tests reject changed
domains, cardinalities, randomness and nonword payloads. Negative compiler cases
require unsupported tails to fail or report loss rather than compile a prefix.

Hostile Negotiations uses two sequential finite exile captures, private
inspection of each capture and a controller choice of which pile to expose.
Its disposition uses the same native grouped movement and then the complete
life-loss continuation. The exile scenarios cover both exposure/pile choices,
short and empty libraries, a prevented first-group exile that reappears in the
second producer's actual top set, and pending exposure rollback/replay.
The exposure is a native turn-face-up action, not an added reveal instruction.
Its selected exile identities use the existing authenticated opening helper;
unopened and pending cases roll back, while an unselected pile stays opaque.
Native owner/peer controls require no `CardRevealed` events from this action.

Intrude on the Mind uses an explicit `original_destination` result query.
`OriginalZoneMoveCards` captures actual arrival memory before added replacement
programs. The query reads only the exact producer's instruction result, so a
redirected/prevented move contributes zero, an arrival subsequently moved away
still counts, and auxiliary movements contribute nothing. A completed empty
receipt is distinct from missing evidence. The source-authored full-body tests
cover those cases, token/counter producer binding, empty piles, and pending
replacement continuation replay. Private outcome sanitization covers the new
arrival memory.

## Rules basis

The six initial hand/graveyard bodies, Hostile Negotiations and Intrude on the
Mind each have one put action over two piles. The implementation treats that
action as simultaneous under CR 608.2f. Truth or Tale repeats put after an
explicit then, so its two actions remain sequential under CR 608.2c. This is an
application of the exact Oracle clauses and the current official
[September 25, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf),
pages 104–105, rather than a separate card-specific ruling.

Persistent ordered exile piles, random pile assignment, combat piles and
unrelated partial bodies are outside this subset.
