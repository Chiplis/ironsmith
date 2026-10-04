# Job select: typed token creation and attachment

## Evidence and scope

The authoritative frozen `e8740178` sharded snapshot has exactly 12 failures containing `unsupported static ability fallback KeywordFallbackText: Job select`. Full metadata and unmodified Oracle text from the same frozen `cards.json` are preserved in `fixtures/job_select_materialization.json.fixture`:

- Astrologian's Planisphere
- Bard's Bow
- Black Mage's Rod
- Dragoon's Lance
- Monk's Fist
- Paladin's Arms
- Red Mage's Rapier
- Sage's Nouliths
- Samurai's Katana
- Thief's Knife
- Warrior's Sword
- White Mage's Staff

This is a measured baseline family, not a claimed post-change support delta. Other Job select cards can have different primary failures.

## Root cause and implementation

The front end already recognized the Job select keyword head but deliberately converted it to `KeywordAction::MarkerText`. Lowering produced the unsupported static fallback, correctly rejected by validation.

Job select now has a typed `KeywordAction::JobSelect` and compiler-owned creation tag. Both compiler lowering and the legacy runtime builder expand it into the existing primitives: an enters-battlefield trigger, one tagged 1/1 colorless Hero creature token, and source attachment to the result. No card-name cases, new runtime effect recipe, or support-validator exception are added. All new enum variants are appended; no serialized runtime enum changes are required. Structural rendering recognizes the Hero create-and-attach program as `Job select`, including after artifact serialization and materialization.

The shared `AttachToEffect` formerly selected the first tagged object. Token-doubling replacements require the Equipment's current controller to choose which one it equips. The executor now offers exactly one legal tagged attachment destination when several are available, preserving the existing attachment-legality/protection checks and treating no legal destination as a no-op. This also benefits Living weapon and For Mirrodin. The choice uses the Equipment's current controller, while token creation continues to use the resolving triggered ability's controller.

## Rules evidence

- [Wizards FINAL FANTASY release notes](https://magic.wizards.com/en/news/feature/final-fantasy-release-notes): Job select's token characteristics, enter-before-attachment timing, Equipment surviving token death, normal subsequent equip, and only one attachment when token creation is doubled.
- [Wizards Comprehensive Rules, April 17, 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260417.pdf), CR 702.182a: the triggered create-and-attach definition.
- [Wizards Comprehensive Rules, June 19, 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf), CR 301.5c: when an effect would equip multiple creatures, the Equipment's controller chooses one.

## Regression coverage

- Bare/reminder-text grammar recognition and keyword display identity.
- Runtime builder's typed Hero creation and matching attachment tag.
- All 12 complete frozen cards through direct runtime compilation and JSON-serialized, validated, registry-materialized artifacts, checking unchanged unsupported-content validation, structural display, and actual ETB creation/attachment.
- No token until the trigger resolves; one colorless 1/1 Hero owned and controlled by the trigger's controller.
- Equipment owned by one player, entering under another's control, then changing controller before resolution.
- Source removal and blinking in response: the token is still created, and the old trigger cannot attach a returned new object identity.
- Bard's Bow increases the Hero's stats, grants reach and the Bard subtype, and stays on the battlefield when the Hero dies.
- Doubled token creation offers one attachment choice to the current Equipment controller, testing Job select, Living weapon, and For Mirrodin. Suspending that choice rolls back uncommitted tokens and preserves the trigger, then resolves once after the choice is supplied.
- Shroud does not interfere with the nontargeting attachment. Creature Equipment cannot attach, but still creates its Hero.
- An explicit unknown fallback remains unsupported.

## Validation status

Worker checks completed: every changed Rust source parses under rustfmt; `git diff --check` passes; all 12 fixture names and complete Oracle texts exactly match the frozen snapshot/corpus. No Rust builds or tests were executed in this worktree, as requested by the coordinator.

Coordinator serial commands:

```sh
source /workspace/shared/ironsmith-card-env.sh
cargo test -p ironsmith-compiler-grammar job_select
cargo test -p ironsmith-engine job_select
cargo test -p ironsmith-engine effects::permanents::attach_to
cargo test -p ironsmith-compiler-runtime --test job_select_materialization
```
