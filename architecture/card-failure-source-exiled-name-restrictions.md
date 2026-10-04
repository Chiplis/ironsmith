# Source-exiled name restrictions

UNVALIDATED source proposals: Circu, Dimir Lobotomist (`c87d547b-00a4-4fdd-ade4-f3f032e5ea3b`) and Godsend (`dd91f4f2-ea46-48c5-8a80-0dc4ae5b79c4`). No compilation or tests executed.

The cast-restriction grammar now admits the typed “spells with the same name as a card exiled with this source” tail, preserving full consumption and ordinary source-name normalization. It uses the existing `SourceExiled` tag and `SameNameAsTagged` relation. No new card-name case or restriction approximation is added.

Source review also found that prospective cast-prohibition checks constructed a source ID context without its linked exiled-card snapshots. The correction reuses the existing source-exile context builder and restricts that builder to current exact Exile members. The map remains keyed to the source incarnation; historical links are not retargeted to a new source or to a later zone incarnation of an exiled card. Tags are built at the live cast check rather than frozen when the static rule was discovered.

Authored direct/artifact scenarios cover a real Circu cast trigger targeting another player's library, current-controller/opponent scope, exiled-card departure/reentry, real Godsend equip and blocking-trigger exile, source departure before trigger resolution, a returned equipment incarnation, split names, and nameless cards. Deferred command: `cargo test -p ironsmith-compiler-runtime --test source_exiled_name_restrictions -- --nocapture`.
