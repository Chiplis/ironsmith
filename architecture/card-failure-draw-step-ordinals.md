# Actual draw-step ordinals (UNVALIDATED)

Source prerequisite for Wiretapping, without an additional coverage count.

- Retain one actual-draw counter per active participant through the entire draw step, including priority and resolving abilities. Shared-turn partners do not overwrite each other.
- Clear at the real step exit in both turn schedulers, including an immediately adjacent additional draw step, and at the next turn. Turn-based draw completion is not a step boundary.
- Replacement/prevention proposals do not increment the counter. The existing physical draw notification records the actual count.
- Checkpoints retain lane-local draw counts, including Grand Melee marker stores. Missing legacy state is safe outside a draw step, but a legacy checkpoint inside a draw step is rejected with an explicit diagnostic. Neither zero nor the turn-wide draw count can reconstruct extra-step ordinals.
- Authored native tests cover ordinary turn draw plus priority, adjacent extra draw, shared-turn alternation, and nonactive draws. Authored wire tests cover public checkpoint/JSON round trip, lane-local helper state, duplicate/invalid/zero counters and legacy rejection. Existing Hullbreacher fixture uses the new counter representation.

No compilation, test execution, or compiler probe was run. Rustfmt parsed changed source and git diff whitespace checks were used only.
