# Varolz, the Scar-Striped - Single-card fix pass blocker report

## Live verification run

Command used:

`cargo run --release -p ironsmith-tools --bin compile_oracle_text -- --name "Varolz, the Scar-Striped" --compare-text`

Current result:

- parse fails in strict mode
- error: `granted ability compiled to unsupported static ability fallback KeywordFallbackText: scavenge`

## What the compiler is doing now

- The static line parses, but the granted keyword `scavenge` is lowered as a keyword fallback marker.
- Strict compilation rejects that fallback marker, so the card does not reach `strict_compiled`.

## Why this is blocked in single-card scope

Varolz requires a reusable grant capability that does not exist yet:

- **Needed model:** grant an activated ability to cards in a zone where the granted ability's activation cost is derived per recipient card (`its mana cost`).
- Existing grant paths support:
  - static ability grants,
  - alternative cast grants (including derived-from-card mana cost),
  - fixed parsed activated ability grants.
- Existing grant paths do **not** support recipient-specific dynamic activation cost derivation for granted activated abilities.

Because the dynamic cost is the core mechanic (`scavenge` cost equals each card's mana cost), any parser-only or text-only workaround would be semantically wrong or rely on fallback placeholders.

## Required reusable engine capability

Implement a generic runtime/IR path for **derived granted activated abilities** (not card-specific), including:

1. Grant model support for an activated ability template whose cost/effect can be materialized from each recipient card.
2. Runtime materialization and legality checks per recipient object in the relevant zone.
3. Parser/lowering rule for clauses of the form `has <keyword>. The <keyword> cost is equal to its mana cost` to target that reusable model.
4. Compiled text rendering from structured fields (no oracle-text patching).
5. Regression coverage for at least one dynamic granted keyword-ability case (Varolz) and one non-Varolz structural variant if available.

Until that capability exists, this card cannot be made strict without either fallback markers or card-specific hacks.
