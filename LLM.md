**Parser Fix Loop (Agent Handoff)**
1. Scope only parser/effects/render internals (`builders`, `EffectAst`/compile path, `compiled_text`, `wasm_api`, static ability display). Do not edit oracle text to “help parsing.”
2. Treat every clause as a semantic contract: if any meaningful tail is unparsed (`where`, `if`, `as long as`, etc.), return `Err` with a clear message. Never silently accept partial parses.
3. Fix by pattern clusters, not cards. Group mismatches by shared oracle shape and add/extend reusable primitives/helpers.
4. Preserve composability: reuse/extend `ObjectFilter`, `PlayerFilter`, `Value`, `ChooseSpec`, `EffectAst` (and keep `Battle` support where type parsing is touched).
5. Reproduce with CLI first:
   - `cargo run -q --bin compile_oracle_text -- --name "Probe" --text "..."`
   - Add `--stacktrace` or `IRONSMITH_PARSER_STACKTRACE=1` when parse path is unclear.
6. For each cluster, verify both:
   - Parse/compile semantics (AST/runtime)
   - Render output (compiled text + wasm UI text) is close enough and not misleading.
7. Add regression tests per pattern:
   - Positive test for supported form
   - Negative test for unsupported tail/partial parse rejection
8. After each cluster: run tests (`cargo test -q`), fix regressions, rerun mismatch audit, move to next biggest real semantic mismatch cluster.
9. Prioritize true semantics over pretty wording. Unsupported behavior should fail loudly, not compile “approximately.”
10. Keep owner/controller distinctions explicit in filters/displays when semantics require it (same for commander identity and other stateful qualifiers).

Use this loop with the four binaries in `/Users/chiplis/ironsmith/src/bin`.

**1. Cluster semantic mismatches (primary tool)**
```bash
cargo run --quiet --bin audit_oracle_clusters -- \
  --cards /Users/chiplis/ironsmith/cards.json \
  --allow-unsupported \
  --use-embeddings \
  --embedding-threshold 0.17 \
  --false-positive-names /Users/chiplis/ironsmith/scripts/semantic_false_positives.txt \
  --min-cluster-size 6 \
  --top-clusters 40 \
  --examples 5 \
  --json-out /tmp/oracle_clusters.json \
  --mismatch-names-out /tmp/mismatch_names.txt
```
What it gives you:
- Ranked clusters by shared oracle semantics.
- Parse-failure counts and semantic-mismatch counts per cluster.
- Examples per cluster.
- Optional machine-readable JSON + a flat mismatch-name list.

Useful flags:
- `--parser-trace` and `--trace-name "<substring>"` to trace only matching cards.
- `--limit N` for quick runs.
- `--embedding-threshold` controls strictness (you’re using `0.17`).
- `--false-positive-names <path>` excludes vetted embedding false positives from mismatch counts.

**2. Reproduce one card exactly**
```bash
cargo run --quiet --bin compile_oracle_text -- --name "Card Name" --detailed <<'EOF'
<oracle text>
EOF
```
Debug flags:
- `--trace` for parser flow logs.
- `--stacktrace` for parser stacktrace.
- `--allow-unsupported` if you want to inspect partial support behavior.

**3. Parse-failure triage (non-clustered, fast)**
`parse_card_text` reads `stdin` blocks separated by `---`, with first line `Name: ...`.
```bash
cat /tmp/card_blocks.txt | cargo run --quiet --bin parse_card_text -- --allow-unsupported
```
Optional:
- `--pattern "unsupported trailing"` to find examples matching a specific error text.

**4. Fallback/ObjectFilter audit**
Also reads `stdin` card blocks; writes JSONL.
```bash
cat /tmp/card_blocks.txt | cargo run --quiet --bin audit_compiled_cards -- \
  --out /tmp/compiled_audit.jsonl \
  --limit 5000 \
  --examples 30
```
Use this to spot suspicious generic compiled output patterns.

**Recommended cluster loop**
1. Run `audit_oracle_clusters`.
2. Pick biggest real-mismatch cluster.
3. Reproduce 2-5 examples with `compile_oracle_text --detailed --trace`.
4. Implement generalized parser/effect/render fix.
5. Run `cargo test -q`.
6. Rerun `audit_oracle_clusters` and repeat.

**Parser Architecture (Current)**
- `src/cards/builders.rs` defines shared parser data types (`LineAst`, `EffectAst`, token/span structs, keyword enums, parse annotations) and includes parser submodules.
- `src/cards/builders/parser.rs` is the orchestration layer:
  - reads/normalizes lines
  - handles metadata + modal/level bookkeeping
  - calls `parse_line(...)`
  - applies parsed `LineAst` into `CardDefinitionBuilder`.
- `src/cards/builders/parse_parsing.rs` is the parsing frontend:
  - tokenization + normalization helpers
  - line classification (`parse_line`)
  - trigger/static/activated/effect clause parsing
  - emits `LineAst`/`EffectAst` only (no game mutation).
- `src/cards/builders/parse_compile.rs` is the lowering/backend:
  - lowers `EffectAst -> Vec<Effect>`
  - resolves tag pipelines (`it`, `triggering`, attached tags)
  - composes control-flow effects (`with_id`, `if_then`, `for_each`, `unless_pays`, vote/mode lowering)
  - returns concrete runtime effect graphs + target choices.
- `src/cards/builders/tests.rs` holds parser-focused regression coverage.

**Parser/Effects Boundary**
- The parser never executes game logic; it only builds semantic IR (`EffectAst`) and lowers to effect graphs (`Effect`).
- Runtime behavior lives in `src/effects/**` (`EffectExecutor` implementations) and game loop execution.
- Static abilities (`StaticAbility`) are parsed/attached directly and are not effect execution; they are applied through static/replacement/continuous systems at runtime.
- If a clause cannot be represented faithfully by existing parse IR + effect primitives, parsing must fail loudly (unless explicit unsupported fallback mode is enabled).

**Effect Primitive Gate (Blocking Requirement)**
- Any new mechanic/effect support is blocked unless one of these is true:
  1. It is implemented as a composition of existing primitive effects/composition wrappers (`if`, `may`, `for_each`, `choose`, `tag`, `unless_*`, etc.), or
  2. It is a truly new primitive semantic that cannot be represented by existing primitives without loss of rules correctness.
- For every new primitive effect, add a short justification in the PR/commit notes:
  - why composition is insufficient,
  - what invariant/rule requires a primitive,
  - and one regression test proving the primitive-only behavior.
- Prefer deleting specialized wrappers when they are equivalent to existing compositions, so the parser lowers to fewer, more orthogonal effect primitives.
