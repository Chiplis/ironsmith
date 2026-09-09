# Recovered score-cohort work

Recovered from the original task execution log after `/private/tmp/ironsmith-score-cohort-457` disappeared. Its branch still pointed at `8c175a73fb11e6ec552b6e63b95f769841df0745`; the source changes had not been committed.

The reconstruction starts at that original base and replays the recorded local source edits. It does not replay builds, network operations, Git mutations, or external writes. All restored file operations were confined to a separate reconstruction directory. The current main checkout and ongoing `codex/score-cohort-recovery` checkout were not modified.

The candidate includes the last verified Heliod's Punishment implementation. The subsequent unfinished Dark Intimations test is preserved in `dark-intimations-test.rs.pending`; it had a known invalid `player.exile` access in the historical run. It is not registered as a completed regression test.

Historical evidence: the original cohort contained 457 cards; the final surviving Heliod report records 372 cards at or above 0.99. Those historical scores are not automatically a claim about current main. Fresh reconstruction checks and integration status are recorded in the sibling forensic report directory.

Replay audit: five source-edit scripts failed in exactly the same way as the historical tool outputs. Two patch hunks did not match during replay, but their intended final changes are present through subsequent recorded edits (tag-unwrapping helper and vote-count traversal). The complete file-edit journal and original result comparisons remain in `reports/lost-cohort-forensics` in the main working directory.

This restoration must be integrated with newer main/recovery changes before it replaces the current engine. Do not discard the ongoing recovery checkout or overwrite its uncommitted sacrifice-protection work.
