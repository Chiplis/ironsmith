# Named-source counter removal and modal X

Status: source-complete proposals, **UNVALIDATED**. No build, compilation or test run performed.

Frozen stack07 identities:

- Marath, Will of the Wild: `fae87115-8749-4d25-a594-7139dd01a034`
- Ulasht, the Hate Seed: `796f1e1f-7fec-429d-82ae-125f541d6cc7`

Both frozen failures reject the complete `remove ... counters from <short self name>` activation-cost operand as an ordinary object filter. The correction gives removal the same contextual source recognizer already used by counter placement, sacrifice and exile. Only a complete self operand selects the existing source-counter payload. Other/among filters retain their existing selection semantics. No new core enum, artifact cost encoding, card-name dispatch or runtime action is introduced.

Direct modal headers previously bypassed the card's source normalization. The document parser now normalizes the cost prefix before its colon using the existing builder-aware alias rules. Triggered headers retain their existing handling; effect/target suffixes and quoted grantor scopes are not rewritten by this addition.

Marath has an independent full-program issue: each modal bullet ends with `X can't be 0.`. The sentence recognizer consumes that clause without an effect, but modal assembly did not preserve its announcement restriction. A compiler-only typed flag now carries a header restriction or the common restriction across all modes into the existing runtime activation minimum. Mixed per-mode restrictions without a global header restriction fail closed rather than restricting legal other modes. Ordinary activated-line handling is unchanged.

Existing engine primitives supply:

- Marath's entry count from actual mana spent to cast; X counter removal from the source; announced-X preservation in the cost context and stack entry.
- Ulasht's additive counts over other controlled red and green creatures; a multicolored creature contributes twice. Fixed source-counter removal and its two modal effects.
- X choice bounds from payable mana and the source's matching counters, minimum validation before mutation, modal target legality and paid-target fizzle handling.

## Authored regressions

`crates/ironsmith-compiler-runtime/tests/named_counter_removal_costs.rs` uses each complete frozen card through both direct strict compilation and a JSON typed-artifact round trip, then real casting/activation and stack resolution.

It covers taxed Marath entry (four mana paid, three printed), every mode, actual mana and source-counter decreases, X bounded separately by mana/counters, zero rejection and valid retry, effects retaining announced X after source counter changes/departure, Ulasht's controller/zone/other/multicolor entry rules, unrelated counter-rich permanents being unable to pay, token characteristics, and illegal target fizzle without a cost refund. Synthetic names prove the parser rule is contextual; mixed-mode X minima reject rather than silently dropping a clause.

Counter grammar unit tests preserve old built-in/fixed payloads, contextual fixed/X sources, non-source chosen filters and complete-consumption rejection.

Deferred command:

`cargo test -p ironsmith-compiler-runtime --test named_counter_removal_costs -- --nocapture`

The adjacent King Darien sacrifice alias and Dominion Bracelet granted-ability source/grantor scopes are outside this bounded cohort.
