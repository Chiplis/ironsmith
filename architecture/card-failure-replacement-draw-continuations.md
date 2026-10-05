# Replacement-created draw continuation (UNVALIDATED)

Bounded follow-up to `99fd6d24`. No additional complete-card count; Wedding Ring remains partial.

CR 121.7 requires remaining unreplaced parts of a simultaneous event to happen before replacement-created draws. A life original replaced by a straight-line program now splits at its first typed DrawCards action. The non-draw prefix executes as part of that original; the draw and suffix resume after every remaining original commit. Deferring the entire program is incorrect when its prefix removes another original's qualifying permanent.

The continuation retains the full replacement child execution context: source/controller and source snapshot, affected player, replacement application history, prior result IDs, tagged exact objects, targets, and remaining instructions. It does not use the next participant's live context or chase a moved source by stable identity. Sequential wrappers retain their surface metadata; transparent wrappers can move intact when the first child action is a draw. Native completion and game/context checkpoint ownership handle pending suffix choices by rolling back the complete enclosing action before replay.

The existing direct `Instead(draw)` qualified-life regression is retained and now has a source implementation. New authored direct/artifact scenarios distinguish the non-draw-prefix negative, retained tags/result IDs/controller after source departure, and suffix-decision rollback of the prefix and all other originals.

## Remaining boundary

Opaque conditional/iteration payloads and transparent annotation scopes around a non-draw prefix cannot be split safely by this reader. Their existing complete execution is preserved; nothing is skipped or counted as a recovery. They require a resumable typed branch/scope frame so the selected prefix can execute without executing its draw. The active unignored `conditional_draw_inside_original_replacement_waits_for_remaining_life_originals` scenario records the concrete May(draw, remove partner) interaction. Wedding Ring stays partial until that general control-flow boundary is implemented.

No builds, compilation, compiler probes or tests were run. Rustfmt source parsing and whitespace checks only. Rules basis: CR 121.7 in the official September 25, 2026 Comprehensive Rules, linked in the preceding receipt document.
