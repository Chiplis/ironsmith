# Mathemagics: exponent semantics and bounded draw execution

## Status and scope

This is a **design proposal, not an implementation or a supported-card claim**.
The source review uses commit `7eaaaad26412ef40f622912dd6d95eebba84a4fa`.
No Rust builds or tests were run for this investigation. Keep this work separate
from the recovered-card publication batch until the execution contract below is
chosen and tested. Reminder cleanup alone does not repair Mathemagics.

The frozen baseline is `e8740178a7f7367ffa3147e7642607042079237c`.
The exact projected input is already retained in
[`fixtures/lossy_metadata.json.fixture`](../fixtures/lossy_metadata.json.fixture):

- Name: **Mathemagics**
- Mana cost: **{X}{X}{U}{U}**
- Type: **Sorcery**
- Oracle text: **Target player draws 2ˣ cards. (2⁰ = 1, 2¹ = 2, 2² = 4, 2³ = 8, 2⁴ = 16, 2⁵ = 32, and so on.)**
- Recorded baseline reason: `oracle_only_fallback: parse input failed before oracle text fallback: rewrite lexer encountered an unsupported token "⁰" on line 25 at 33..36`

The fallback's compiled instruction is “Target player draws two cards.” This is
real semantic loss: the printed exponent has disappeared. The original metadata
route also fails on a reminder glyph. These are independent defects.

## Source and grammar boundary

Relevant source paths:

- `crates/ironsmith-compiler-syntax/src/lexer.rs`, `is_word_char` and
  `push_normalized_token_words`
- `crates/ironsmith-compiler-grammar/src/preprocess.rs`, `authored_rules_tokens`
- `crates/ironsmith-compiler-source/src/document_cst.rs`, `structural_nodes`,
  `capture_delimited`, and `reminder_text_decision`
- `crates/ironsmith-compiler-grammar/src/grammar/shared_util/value_expr.rs`,
  `parse_value_expr_tokens`
- `crates/ironsmith-compiler-grammar/src/grammar/effects/zone_move_shapes/draw.rs`,
  `parse_draw_head_shape`

The lexer explicitly treats a modifier letter without a case mapping, including
`ˣ`, as a separator in its normalized word view. Thus an authored `2ˣ` token can
become the count word `2`. The shared value reader currently creates a
`TokenWordView` before recognizing values; recognition added after this step is
too late to recover the exponent reliably.

`authored_rules_tokens` lexes the entire authored line *before* excluding
parenthetical reminders. Superscript digits are unsupported there; accepting
those digits alone would next encounter the unsupported `=` signs. Broadly
accepting arbitrary punctuation or deleting unsupported glyphs is not a repair.

Proposed boundary changes, to be implemented only with the runtime plan:

1. Recognize a bounded numeric-superscript expression as a typed lexical/count
   form before lossy word normalization. `2ˣ` must produce base 2 and exponent X.
   Other unsupported superscript expressions must fail closed, never fall back
   to their numeric prefix. There is no need to introduce ASCII caret syntax in
   the first change.
2. Exclude genuinely nonsemantic appended reminder spans *before* semantic
   lexing, retaining the complete raw CST and provenance. Lex semantic slices
   with original byte/line offsets, or use an equivalent offset-preserving
   mechanism. Preserve standalone parenthesized abilities and functional
   parentheticals such as “it's not a creature.” Respect nesting and quotations.
3. Do not trust `ReminderTextDecision::Preserved` as proof that an appended
   parenthetical is rules text. The current CST classifier receives the
   delimited substring, including its surrounding parentheses, and consequently
   marks an ordinary appended reminder `Preserved` too. Classification needs
   whole-line context. The existing source spans are useful; the current enum
   values alone are not an exclusion policy.
4. Feed the typed number through the existing draw count grammar and existing
   `DrawCardsEffect`; do not add a Mathemagics card-name exception or replacement
   draw effect.

## Smallest proposed typed primitive

A candidate shared representation is:

```rust
Value::IntegerPower { base: u32, exponent: Box<Value> }
```

It represents a nonnegative integer literal base with a dynamic nonnegative
integer exponent. Mathemagics uses `base: 2, exponent: Value::X`. A renamed card
and `3ˣ` should use exactly the same path. The representation is a proposal; it
has not been added to the model.

This is smaller than introducing arbitrary algebra or a card-specific effect.
However, even this node crosses several real contracts:

- `crates/ironsmith-core/src/value_model.rs` defines the shared `Value` enum with
  serde support and derived `TagKeyWalk` traversal.
- `crates/ironsmith-engine/src/effects/helpers/value_eval.rs` is the shared
  execution/continuous interpreter and returns `Result<i32, ExecutionError>`.
- `crates/ironsmith-engine/src/effects/helpers/value_eval/context.rs::x`
  currently casts an execution X from `u32` to `i32`. A huge X must be checked
  before this cast; otherwise it can wrap negative before exponent validation.
- `resolve_continuous` in the shared interpreter turns an evaluation error into
  a panic. A generic exponent must not accidentally introduce a new user-input
  panic through this adapter. Restricting initial grammar ownership to draw
  counts, or defining the continuous error contract, must be explicit.
- Manual recursive visitors in compiler-resolve's `reference_helpers.rs`,
  `reference_resolution.rs`, `tag_support.rs`, and
  `effect_ast_normalization.rs` must descend into the exponent. The derived
  walker does not replace those existing responsibilities. Review dependency,
  mana-analysis, filter, and other numeric consumers as well; wildcard matches
  may compile while missing nested references.
- `crates/ironsmith-text/src/compiled_text/normalize_common/value_rendering.rs`
  owns `describe_value`. The rendered text must retain exponent meaning and
  round-trip through the chosen grammar; it cannot render as a fixed 2.
- Core `DrawCardsEffect` already serializes its `Value`; the artifact decoder's
  `zone_library.rs` decodes that core effect. A new custom effect decoder is not
  necessary. `crates/ironsmith-compiled-artifact/src/lib.rs` currently pins
  format version 5 and an engine schema hash. Explicitly decide schema/version
  compatibility for the new variant; a same-build serde round-trip alone does
  not establish old-reader compatibility.

Checked exponentiation must have defined zero/one-base, zero-exponent, missing
X, negative-exponent, and overflow behavior. `0^0` needs an explicit design
choice if the generic grammar admits it. For base 2, `2^30` fits `i32` and
`2^31` does not. Returning a typed evaluation failure is safer than wrapping or
saturating, but it does **not** mean the full printed card works at X = 31.
Widening only this node to `u64` merely moves the boundary and does not fix the
existing count/outcome interfaces.

## Actual draw execution constraints

Read `crates/ironsmith-engine/src/effects/cards/draw_cards.rs`, especially
`DrawCardsEffect::execute`, `execute_draw_instruction`,
`finish_direct_draw_segment`, and `commit_draw_original_with_reveal_mode`.

The requested count is evaluated as `i32`, clamped at zero, and converted to
`u32`. `execute_draw_instruction` loops once for every requested card. It runs
replacement processing for each positive proposal before consulting the
physical library. A replacement can redirect, multiply, replace with another
program, add instructions, ask a decision, or replenish state used by later
proposals. Empty-library proposals are therefore not generally ignorable.

The lower-level `GameState::draw_cards_with_dm` in
`crates/ironsmith-engine/src/game_state.rs` loops over the final physical count.
On an empty library it only sets `attempted_draw_from_empty_library` and
continues. This flag is an SBA observation, not a per-attempt counter. No
`CardsDrawnEvent` is emitted when no physical card reaches hand.

Consequences:

- Even a representable `2^30` asks for over a billion outer iterations, each
  potentially refreshing replacement state and cloning checkpoints.
- Replacing the requested count with library length is incorrect: it loses the
  empty-library attempt, replacement effects on that attempt, and possibly
  repeated replacement programs after the library is empty.
- Returning early on an empty library before replacement processing is also
  incorrect. Existing tests include an empty-library draw replaced by gaining
  life, and a replacement that wins rather than drawing.
- Direct draws accumulate into a segment. Segments must be published at the
  current boundaries before intervening replacement programs inspect history;
  automatic reveal decisions are part of the same transaction. A shortcut must
  not flush a pending direct segment earlier merely to simplify its own loop.
- `DrawEvent` includes `first_of_instruction`, `is_first_this_turn`, and
  `first_of_draw_step`. The first attempt and subsequent attempts can match
  different replacements even when no physical card was drawn.
- Replacement program totals and some physical/outcome counters use `i32`;
  several additions are currently unchecked. Adding a checked exponent alone
  does not make all downstream arithmetic safe. A work guard must include
  nested/replacement-generated draws, not only the original count.

## Existing budgets and continuations

There is no general effect-resolution fuel or draw cursor in
`ExecutionContext` / `ExecutionError`. The following mechanisms are relevant,
with narrower scopes:

1. `decision/mana/resumable.rs::ManaAnalysisSession` retains exact search
   frontiers across bounded node-pop slices. This budgets mana analysis on an
   immutable snapshot, not mutation during spell resolution. It cannot simply
   be wrapped around the draw loop.
2. `StaticEffectDiscoveryLimits` in `static_ability_processor.rs` bounds static
   discovery at 128 rounds and 16,384 generated effects by default. It fails
   explicitly when the snapshot is incomplete. It does not limit repeated
   complete replacement discovery calls for a billion draw proposals.
3. `DrawCardsEffect::execute` clones game/context and restores them on an error
   or pending decision. `game_loop/stack_resolution.rs::resolve_stack_entry_full`
   also restores the complete resolution and trigger queue on error/pending
   input. These are valuable transactional boundaries.
4. Decision continuations retain answers and replay the transaction from its
   checkpoint. They do not retain a draw-loop program counter and partial
   private result. Treating fuel exhaustion as `awaiting_choice` would invent
   a player decision; replaying the same fuel-limited prefix without a retained
   frontier would never progress.

A real resumable resolution mechanism would need private owned game/context,
remaining instruction count, direct segment buffers and events, nested effect
frames, replacement application history, pending reveal/commander decisions,
source/target context, RNG/provenance, and deterministic serialization/replay.
It must not expose partial mutations or allow priority/SBAs between chunks of
one instruction. That is broader than the proposed compiler leaf repair.

## Conservative fast paths worth evaluating separately

These are source-level proposals, not proved-by-tests implementations.

### A. Physical empty-library tail

Inside `draw_cards_with_dm`, after recording an empty-library attempt, break
instead of continuing its remaining physical iterations. In this function's
current empty branch there are no commander choices, replacement checks,
provenance allocations, events, or other mutations. No intervening operation
can replenish the library. This is the smallest independent optimization.
It does **not** eliminate `execute_draw_instruction`'s outer proposal loop.

### B. Prohibited-draw tail

At the outer loop's `!game.can_draw(player_id)` check, the current iteration
performs no operation at all. With no intervening mutation or decision, later
iterations have the same result; terminating that tail is a candidate constant
work path. Preserve the existing returned outcome and any already-pending
segment. This does not justify bypassing replacements when drawing is allowed.

### C. Proven observer-free empty-library tail

A deliberately conservative first guard would require all of the following:

- At least one real normal empty-library proposal has completed, including
  replacement processing; no choice or error is pending.
- The target library is still empty, the empty-draw flag is already set, and
  the player is still the same valid recipient.
- Static/replacement discovery has completed after the last relevant mutation.
  The **whole** registered replacement list is empty, not merely a cached list
  of currently matching draw effects.
- `ExecutionContext::additional_replacement_effects()` is empty too. Event-local
  replacements cannot be ignored just because the global manager is empty.
- Context provenance is a valid existing node. With valid context provenance,
  ignored draw proposals reuse that node. With missing/invalid provenance,
  `ensure_event_provenance` can allocate a root on each attempt; skipping these
  needs a separate explicit provenance policy rather than assuming byte-equal
  replay state.
- Pending direct draws retain their original flush/reveal boundary.

Why this narrow guard is promising: `find_applicable_trait_replacements` reads
only the registered and additional replacement lists. With both empty, no
proposal can cause an intervening program or decision. The physical empty
branch is idempotent. Repository readers of `attempted_draw_from_empty_library`
are the SBA rule and tests; it is not a general authored predicate used to
activate another replacement. No new card reaches hand, so no draw-trigger or
reveal notification is produced by the skipped tail.

The proof still needs differential tests comparing ordinary finite execution
against compressed execution, including persistent state, outcome/events,
provenance/allocator behavior, and the subsequent SBA. Discovery errors must
propagate, not authorize the fast path. Runtime instrumentation/cache counters
should be distinguished explicitly from observable game state.

The guard intentionally forgoes optimization when any replacement exists,
including unrelated replacements. A broader guard based on “none matched this
proposal” would need to prove invariance of all matcher inputs, including
first-of-instruction flags and refreshed source abilities. Do not start there.

Persistent empty-library draw replacements defeat this guard. For example,
“instead gain life” may apply once per requested draw forever without drawing
any card. A replacement can also refill a library. Therefore this optimization
cannot by itself establish bounded execution for every legal Mathemagics cast.

## Decision needed before implementation

Recommended ordering:

1. Validate the conservative draw-tail optimizations independently on existing
   draw semantics, without adding an exponent node or changing card status.
2. Choose an explicit execution policy for observable huge tails. The smallest
   fail-closed option is a transaction-wide work budget that returns a distinct
   engine-resource failure and rolls back the whole stack resolution. It must
   count nested draws/replacement work and must not report successful partial
   resolution, a game loss, or an illegal cast. An arbitrary cap is an engine
   limitation, not an interpretation of the card.
3. Decide whether limited, explicit resource/number failures are acceptable
   for the campaign's supported-card contract. If not, retain Mathemagics as
   unresolved while building a real resumable/large-count execution design.
   Checked `i32` powers plus an empty-library fast path are insufficient for
   unrestricted X and observerful replacements.
4. Only after that decision implement the typed source/grammar/value/renderer
   path together with executable evidence. Keep the original loss visible
   until the actual exponent semantics are represented; do not bless a
   reminder-free fixed-2 artifact.

A larger alternative is an exact symbolic/large integer count with resumable
per-card work and special observer-free tail compression. It avoids immediate
`i32` overflow but crosses count comparisons, outcome metrics, draw-event
modifications, serialization, and resource policy. It should be designed as its
own runtime family rather than smuggled into a lexical fix.

## Required regression plan

Use normal compiler-runtime/tools integration targets and existing engine unit
coverage. No custom global Rust flags or separate success-marker audit.

### Typed source and serialization

- Compile the exact frozen raw input with full {X}{X}{U}{U} cost and Sorcery
  metadata. Raw and reminder-free inputs have equivalent typed draw semantics.
- Assert `IntegerPower(2, X)` within the actual draw effect, not only a rendered
  substring or a support marker. Rename the card and use `3ˣ` as metamorphic
  cases. Unsupported superscripts must reject without accepting the base.
- Check UTF-8 spans and retained authored reminder provenance. Preserve quoted
  parentheses, nested reminders, standalone parenthesized mana abilities, and
  functional noncreature instructions.
- Renderer -> parser preserves the typed exponent. Artifact encode/decode and
  materialization preserve it and metadata; test the chosen schema/version
  rejection contract for incompatible readers.
- Test exponent reference traversal independently if the model allows exponent
  expressions beyond X. Validate missing X, negative exponent, 0/1 base,
  exponent 0, representable boundary, and overflow without panic or wrapping.

### Actual casts and resolutions

- Cast at X = 0, 1, 2, 5 through the real game loop: pay 2X + UU and draw exactly
  1, 2, 4, 32 for only the chosen target player. Include self and opponent
  targets, copied spell X, and illegal-target resolution.
- With too few cards, draw the physical cards, record the empty-library attempt,
  and apply the loss only at the proper SBA boundary. Include loss prevention
  and existing win-instead-of-empty-draw replacement behavior.
- Exercise “can't draw,” first-card restrictions, first-in-instruction
  replacements, static and one-shot replacements, redirects, optional choices,
  double draws, additional programs, and library-refilling programs.
- Differentially compare ordinary and compressed finite tails. Include pending
  reveal/commander decisions, an event-local-only replacement, a static source
  gaining/losing replacement text, and invalid provenance. Assert no duplicate
  cards, notifications, consumed one-shots, or premature segment publication.
- Exercise a huge representable exponent with a finite observer-free library;
  establish bounded executed work, not just eventual correctness. Exercise a
  huge observerful tail, nested replacement expansion, X = 31, and u32::MAX
  under the chosen resource/overflow contract. Verify the transaction's state,
  stack entry, history, trigger queue, and context remain intact on failure.
- If resumable execution is selected, small chunk sizes must produce the same
  final state and event order as uninterrupted resolution, with no restart
  livelock or visible partial state.

Completion requires typed semantics, artifact integrity, and this executable
contract. A strict compile count alone cannot close the frozen Mathemagics loss.
