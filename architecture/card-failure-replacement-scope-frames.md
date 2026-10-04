# Replacement branch/scope continuation frames (UNVALIDATED)

Follow-up to the straight-line CR 121.7 continuation. No full-card promotion: Wedding Ring remains partial at a quantified-action boundary described below.

The continuation now retains a runtime tree of selected program and scope frames. A first draw captures its execution context; resumption completes that subtree, unwinds enclosing scopes, and then runs the remaining program. Previously completed prefix outcomes are included exactly once, not appended twice to the final replacement receipt.

Supported owners: ordinary/coordinated sequences; May; typed conditions (including hidden optional-reveal guards and announced-mode branch selection); result-based If (including per-player result partitions and chosen-number repetitions); result-ID, tagged-object and explicit-source wrappers. Normal and continued May/conditional/If execution share preparation helpers, so selection is not guessed by the continuation reader. Accepted/declined optional bookkeeping, original player bindings, hidden identity guards, result aliases and source snapshots remain owned by their scopes. A zero draw performs no deferral and its following instruction remains in the original program.

Authored exact-card scenarios retain the previous May regression and add acceptance/decline without reasking, a condition that changes after other originals, a result-ID scope around an optional prefix/draw, tagged finalization identifying the actual drawn incarnation, and zero-draw nondeferral. All direct and artifact paths remain unrun.

## Remaining boundary

ForPlayers owns action-major simultaneous preparation/commit, optional program groups, per-player result/tag partitions, APNAP choices and aggregate counts. A nested quantified-player program containing a non-draw prefix before its first draw cannot be correctly replaced by a new player-major loop or reconstructed from only merged result counts. It needs its native action-iterator state retained at the draw unit and resumed after the enclosing originals. The active unignored `quantified_replacement_draw_prefix_keeps_remaining_original_life_changes_ahead_of_draws` scenario records this precise gap. The capability check preserves complete existing execution for unsupported owners; it never drops a draw or marks an unsupported owner complete.

No compilation, tests, builds or compiler probes were run. Rustfmt parsing and git whitespace checks only.
