# Parser/Effect Mismatch Loop

This document is the operating guide for the parser/effects/rendering loop we have been running.

It is optimized for:
- producing the *correct* mismatch report,
- fixing clusters through generalized primitive composition (not one-off card hacks),
- repeating until there are no semantic clusters with size `> 1`,
- and then implementing missing mechanics with rules-accurate behavior.

---

## 1. Ground Rules

1. Never edit oracle text to force a parse.
2. Keep strict mode as the source of truth:
   - do **not** rely on unsupported fallback behavior to claim success.
3. Any meaningful unparsed tail (`where`, `as long as`, `unless`, etc.) must fail with `ParseError`.
4. Fix by pattern cluster, not by individual card.
5. Prefer composition using existing primitives in:
   - parser AST/lowering (`EffectAst`, `ChooseSpec`, `Value`, filters),
   - runtime effects (combinators + existing effect executors),
   - renderer compaction/normalization.
6. **Do not use oracle-text rendering hacks to "pass" semantics.**
   - Do **not** render card text from parser-preserved oracle lines.
   - Do **not** use `oracle_like_lines` (or equivalent oracle-dependent normalization) as the semantic source of truth.
   - Do **not** add semantic round-trip parse guards that convert semantic mismatches into parse failures to hide mismatches.
   - Semantic mismatches must be fixed in parser/lowering/runtime/renderer behavior, not masked by oracle text.
7. **Do not use denylist suppression to hide semantic failures.**
   - Do **not** add card-name deny lists, oracle-line deny lists, or mismatch-name suppression as a way to make threshold reports pass.
   - Do **not** add failing cards (or their oracle fragments) to parser rejection lists such as `reject_known_partial_parse_line` just to force parse failures.
   - If a card remains below threshold, fix the underlying parser/lowering/runtime/renderer behavior or keep it as a real parse failure only when the mechanic is genuinely unsupported.
8. **No failure-by-rejection churn.**
   - Do **not** convert low-similarity cards into parse failures as a "fix."
   - Any new parser rejection must be tied to a genuinely unsupported mechanic (with rule citation in commit notes/tests), not to a specific failing report cohort.
   - Progress must be measured by improved compiled semantics for the same cards, not by moving cards out of the similarity denominator.

---

## 2. Generate The Correct Semantic Report

Use the full-card strict report as baseline:

```bash
cargo run --quiet --bin audit_oracle_clusters -- \
  --cards /Users/chiplis/ironsmith/cards.json \
  --use-embeddings \
  --embedding-threshold 0.75 \
  --false-positive-names /Users/chiplis/ironsmith/scripts/semantic_false_positives.txt \
  --min-cluster-size 2 \
  --top-clusters 200 \
  --examples 3 \
  --json-out /tmp/oracle_clusters_075.json \
  --mismatch-names-out /tmp/mismatch_names_075.txt
```

Notes:
- Do **not** pass `--allow-unsupported` for gating runs.
- Keep `--min-cluster-size 2` so you focus on reusable patterns first.
- Save both JSON and mismatch-name outputs every run.
- Semantic comparison for gating must use compiled output from the actual renderer path (never oracle-preserved text).

Quickly inspect actionable semantic clusters:

```bash
python - <<'PY'
import json
with open('/tmp/oracle_clusters_075.json') as f: d=json.load(f)
for i,c in enumerate(d['clusters'],1):
    if c['semantic_mismatches']>0 and c['size']>=2 and c['parse_failures']==0:
        print(i, c['size'], c['signature'])
PY
```

Interpretation:
- `parse_failures == 0 && semantic_mismatches > 0`: highest-priority true semantic/render clusters.
- mixed parse+semantic clusters: usually parser coverage issue first.
- parse-failure-only clusters: mechanics/grammar coverage work.

---

## 3. Cluster Fix Loop (Semantic First)

For each top semantic cluster (`size > 1`):

1. Reproduce 2-5 examples with:
```bash
cargo run --quiet --bin compile_oracle_text -- --name Probe --text "<oracle text>" --detailed --trace
```
2. Find the common abstraction gap.
3. Implement a generalized fix (not card names).
4. Keep fixes compositional:
   - parser should produce reusable AST forms,
   - lowerer should reuse existing effect constructors,
   - runtime should avoid bespoke behavior if existing primitives can compose it,
   - renderer should compact common effect sequences.
5. Re-run the semantic report.
6. Repeat until no semantic cluster with size `> 1` remains.

Stop condition for this phase:
- no cluster with `size >= 2`, `semantic_mismatches > 0`, and `parse_failures == 0`.

### 3.1 Raw Triangulation (`oracle` vs `--detailed` vs `--raw`)

For every semantic mismatch you investigate, compare **three** views of the same text:

1. Oracle text (`threshold_failures` / source card text).
2. Rendered compiled text (`compile_oracle_text --detailed`).
3. Actual compiled abilities/effects (`compile_oracle_text --raw`).

Use the **same** oracle text for both `--detailed` and `--raw`.

```bash
name="Electrosiphon"
oracle=$(jq -r --arg n "$name" '.entries[] | select(.name==$n) | .oracle_text' \
  /tmp/threshold_failures_070_iter30.json)

echo "ORACLE:"
printf '%s\n' "$oracle"

echo
echo "COMPILED (rendered path):"
cargo run --quiet --bin compile_oracle_text -- --name "$name" --text "$oracle" --detailed

echo
echo "COMPILED (raw abilities/effects):"
cargo run --quiet --bin compile_oracle_text -- --name "$name" --text "$oracle" --raw
```

How to decide where to fix:
- `--raw` semantically correct, `--detailed` wrong/missing wording:
  - renderer/normalization issue.
- `--raw` missing constraints/tags/amounts/branches:
  - parser/lowering/runtime semantics issue.
- both `--raw` and `--detailed` align with oracle semantics:
  - inspect similarity heuristics/false-positive handling.

Tip for faster raw inspection:

```bash
cargo run --quiet --bin compile_oracle_text -- --name "$name" --text "$oracle" --raw \
  | rg "Tagged|tagged_constraints|UnlessAction|IfEffect|ManaValueOf|EffectId"
```

---

## 4. Primitive Composition Policy

### 4.1 Parser/Lowering side
Prefer:
- `ChooseSpec` + `ChoiceCount` over custom target shapes.
- tag pipelines (`Tagged`, `it`, `triggering`) over ad-hoc references.
- `Value` expressions (counts, devotion, event value, etc.) over hardcoded string handling.
- compositional `EffectAst` sequences (`choose -> tagged follow-up`, `if`, `may`, `for_each`) over new top-level bespoke AST variants.

### 4.2 Runtime effects side
Prefer:
- existing effect executors + combinators (`ForEach`, `May`, `IfThen`, `UnlessPays`, etc.),
- filter/value specialization,
- minimal new primitives only when existing primitives cannot express rule-correct behavior.

### 4.3 When a new primitive is justified
Add a new primitive only if all are true:
1. Existing primitives cannot express the mechanic/rule correctly.
2. Composition would lose game-state semantics (timing, replacement, layering, target legality, etc.).
3. You can define stable reusable behavior beyond one card.
4. You add regression tests proving why primitive-only behavior is needed.

---

## 5. Unimplemented Mechanics Loop

After semantic clusters are exhausted, audit mechanic coverage:

```bash
cargo run --quiet --bin audit_parsed_mechanics -- \
  --cards /Users/chiplis/ironsmith/cards.json \
  --json-out /tmp/parsed_mechanics.json
```

This identifies:
- parse-success cards carrying unimplemented marker abilities,
- parse-success fallback lines (if any path still emits them),
- mechanic frequency for prioritization.

Then use parse-failure clusters from `audit_oracle_clusters` to drive mechanic implementation order (largest first, or easiest first depending sprint objective).

Stop condition for mechanics phase:
- no unimplemented-marker mechanics left in parse-success cards,
- no fallback-line mechanics left,
- and no size `> 1` cluster that is only “unsupported mechanic X”.

---

## 6. Mandatory Web Research For New Mechanics

Whenever implementing a new keyword/mechanic, first verify current official rules online.

Minimum required sources:
1. MTG Comprehensive Rules (latest version).
2. Official mechanic reminder/rules text (Gatherer/Scryfall/or set release notes).
3. If edge cases exist, official rulings/Release Notes examples.

Research checklist per mechanic:
1. Zones where the mechanic applies (battlefield only? all zones? cast-only?).
2. Whether it is a static ability, triggered ability, replacement effect, or special action.
3. Stack interaction (uses stack vs special action not using stack).
4. Targeting semantics and legality checks.
5. Duration and state-based interactions.
6. Multiplayer and controller/owner edge cases.
7. Interaction with copying/face-up-down/counters/cost reductions/replacements (as applicable).

Document the chosen interpretation in code comments/tests when non-obvious.

---

## 7. Regression Strategy

For each generalized fix:
1. Add at least one positive parse/compile/render test.
2. Add at least one negative test for partial/incorrect parse rejection.
3. Add runtime behavior test if mechanic semantics changed.

Recommended files:
- `/Users/chiplis/ironsmith/src/cards/builders/tests.rs`
- effect-specific test module under `/Users/chiplis/ironsmith/src/effects/**`
- renderer tests in `/Users/chiplis/ironsmith/src/compiled_text.rs` when text compaction changes.

---

## 8. End-Of-Loop Gating

At the end of each full loop:

1. Build checks:
```bash
cargo check -q
./rebuild-wasm.sh
```

2. Full tests:
```bash
cargo test -q
```

3. Full semantic audit at target threshold:
```bash
cargo run --quiet --bin audit_oracle_clusters -- \
  --cards /Users/chiplis/ironsmith/cards.json \
  --use-embeddings \
  --embedding-threshold 0.75 \
  --false-positive-names /Users/chiplis/ironsmith/scripts/semantic_false_positives.txt \
  --min-cluster-size 2 \
  --top-clusters 200 \
  --examples 3 \
  --json-out /tmp/oracle_clusters_075_final.json \
  --mismatch-names-out /tmp/mismatch_names_075_final.txt
```

---

## 9. Practical Prioritization Order

Use this order each cycle:
1. semantic clusters with `size > 1` and `parse_failures == 0`
2. mixed clusters (`parse_failures > 0` and semantic mismatches)
3. parse-failure-only clusters representing real mechanics
4. singletons (`size == 1`)

This gives maximum mismatch reduction per change while preserving generalized architecture.
