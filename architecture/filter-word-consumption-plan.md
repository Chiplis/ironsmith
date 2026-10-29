# Object-filter word consumption — implementation plan

Status: proposal, 2026-10-29. Prerequisite: the uncommitted keyword-filter work
(`ResolutionProgram.keyword`, `OFFICIAL_KEYWORD_ABILITIES`, the trailing with/without
check in `reference_tag_stage_reference.rs`, the named-source guard in
`anthem_grant_lines.rs`) is committed first, so its corpus baseline is the starting point.

## Problem

The object-filter grammar never records which input words it read. Unknown words are
silently dropped, and the card compiles to something else:

| Input | Compiles to |
|---|---|
| Blorp creatures you control get +0/+1. | Each creature you control gets +0/+1. |
| Creatures you control without flyng get +1/+1. | Each creature you control gets +1/+1. |
| (before 2026-10-29) …without echo… | every creature (the keyword was not in the table) |

The 2026-10-29 interim fixes only catch a trailing `with/without <word>` and set-shaped
subjects falling back to "this permanent". Everything else still drops silently, which
matters most for custom cards, where typos and unsupported phrasing are common.

### Why a cheaper check failed

"Removing word *w* leaves the filter unchanged ⇒ *w* was ignored" was tried on the full
corpus and rejected: 691 cards newly failed and corpus compile time rose about 1.9×. A
word can leave the filter unchanged legitimately:

- **Redundant:** "*any* creature card", "of *their* choice", "a *single* permanent".
- **Read by the caller from the same tokens:** "exiled with *this*", "your *second* spell
  each turn", "*if* this card".

Only the parser knows which words it read, so the parser has to record it.

## Shape of the code today

- `crates/ironsmith-compiler-grammar/src/grammar/filters/` plus `object_filters.rs` is
  about 52k lines.
- The core is `parse_object_filter_inner_unchecked` in
  `reference_tag_stage/reference_tag_stage_reference.rs`: one 3,055-line function. It
  peels token-level stages (relations, attachments, unions, recursion on sub-phrases),
  then scans words in about 780 places.
- It works in three index spaces at once: tokens, `all_words_with_articles`, and
  `all_words` (articles removed), joined by `map_non_article_index`-style closures.
- 52 helpers take `&mut Vec<&str>` and consume a clause by `drain`ing it out of the
  shared word list. The removal is the only trace that a word was read.
- `parse_object_filter` has about 40 early-return peeling stages, and the filter parser
  is called from about 1,300 sites in 213 files. 42 files probe it speculatively
  (`probe_shape`, `parse_best_object_filter_suffix`).
- Useful existing pieces:
  - Every `OwnedLexToken` has a `TextSpan` into the authored line; rewritten tokens use
    `TextSpan::synthetic()`.
  - `parse_phrase_anywhere` and friends already return a `PhraseFact` with a `WordSpan`.
  - `crate::parse_loss` is a side channel that the sentence memo replays on a hit.

## Design

### Claims are source positions

A claim is the source span of a token the grammar read. Keying by `TextSpan` rather than
by slice index survives everything the filter code does to its input: `to_vec`, slicing,
article removal, `drain`, sub-phrase recursion and token rewrites. Recursion into a
sub-phrase claims the same spans the parent sees, so no offset mapping is needed.

Synthetic tokens have no authored position. They are claimed implicitly; whoever
inserted them has already read the authored words they replace.

### A scoped recorder, not a new parameter

Threading a claim set through 1,300 call sites is the refactor we can't afford.
Instead, add a side channel modeled on `parse_loss`:

```rust
// crates/ironsmith-compiler-grammar/src/word_claims.rs
pub(crate) fn claim(span: TextSpan);            // no-op outside a scope
pub(crate) fn claim_tokens(tokens: &[OwnedLexToken]);
pub(crate) fn scoped<T>(f: impl FnOnce() -> T) -> (T, ClaimSet);
```

- `scoped` pushes a fresh set on a thread-local stack, runs `f`, then pops it.
- A successful nested scope merges into its parent; a failed one is discarded, so
  speculative attempts can't leak claims.
- The sentence memo records the claims made under a cached call and replays them on a
  hit. This is the same rule `parse_loss` already follows, and missing it would produce
  phantom unclaimed words only on memo hits.

### The check is per filter invocation

`parse_object_filter` (and `_lexed`) runs its body inside `scoped`. On success it
compares the input's authored spans with the claim set. Anything unclaimed is reported in
audit mode (stage 3) and is an error once enforced (stage 5).

Because the check is local to one invocation, probes the caller later discards don't
matter. Each invocation answers only "did I read my own input?".

### Callers that pass more than a filter

Some callers hand the filter words they read themselves ("exiled with *this*"). Two
options, chosen per site during the burn-down:

1. **Trim:** the caller passes only the filter's words. This is the right fix, and most
   sites can do it.
2. **Pre-claim:** the caller wraps the call in `word_claims::caller_owned(&tokens[..], ||
   parse_object_filter(…))` to declare which words it owns. This is for sites where
   trimming would duplicate the grammar.

## Stages

Each stage is gated on a release-corpus A/B run: `sync_card_status_db --db-path
<scratch>.sqlite3 --cards cards.json < /dev/null`, about 9 minutes, diffing
`latest_card_observation` ⨝ `card_compilation` on `parse_status` and `compiled_text`.
Stages 1–4 must show **zero** card changes.

| Stage | Work | Size | Behavior change |
|---|---|---|---|
| 1 | Recorder, scopes, memo replay, `parse_object_filter` audit hook (env-gated report) | 1 day | none |
| 2 | Claim in shared helpers (`parse_phrase_*`, `PhraseFact`, `word_primitives`, keyword/type/subtype/color readers); turn the 52 `drain` helpers into drain-and-claim | 2–3 days | none |
| 3 | Burn-down rounds over the corpus report: claim at the remaining scan sites, ordered by how many cards each unclaimed word affects; claim redundant words explicitly as no-ops ("any", "single", "both") | 3–5 days | none |
| 4 | Caller hygiene: trim or pre-claim at the sites the report attributes to callers (the 691-card list from the rejected check is a ready-made start) | 1–2 days | none |
| 5 | Enforce: unclaimed authored words are an error. Retire the interim trailing with/without check. `parse_best_object_filter_suffix` may drop a prefix only if the caller claimed it or it is a connective | 1 day | intended only |

Total: about 1.5–2 weeks of focused work.

### Stage 3 report format

One TSV row per (card, filter invocation): the unclaimed words, the full input, and the
innermost recorder site that ran. Aggregating by word gives the burn-down order;
aggregating by caller (`#[track_caller]`, as `parse_ledger` does) separates "filter
didn't read it" (stage 3) from "caller passed extra words" (stage 4).

### Stage 5 acceptance

- The corpus diff lists only cards whose old compiled text was wrong. Each one is
  reviewed: either the new error is honest or the phrase gets real support.
- The three inputs in the problem table above are errors.
- `cargo test -p ironsmith-compiler-grammar --lib` stays at the 23 known HEAD failures.
- Corpus compile time is within 5% of the stage-1 baseline.

## Risks

- **Memo replay.** Forgetting to replay claims on a sentence-memo hit makes unclaimed
  words appear only on hits. Stage 1 includes a test that compiles the same card twice
  and compares the reports.
- **Claims that are too broad.** A helper that claims its whole input when it matched
  only part would hide real drops. Claim exactly what a rule matched, never the input.
- **Token rewrites.** Normalizations that turn authored tokens into synthetic ones must
  leave the originals visible to the check, or a dropped word could be hidden behind a
  rewrite. Audit the rewrite helpers in stage 2.
- **Concurrent WIP.** The main checkout routinely carries other sessions' uncommitted
  grammar work. Run stages in a dedicated worktree with an APFS-cloned `target/release`
  (`cp -cR`) so corpus baselines are stable.

## Out of scope

- Sentence-level consumption outside object filters (effects, triggers, costs). The
  recorder is reusable there once filters are done.
- Custom keyword names in filters; see `custom-keyword-references-plan.md`.
