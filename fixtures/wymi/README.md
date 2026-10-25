# Wymi layer and token audit

Pinned upstream: `~/wymi`, commit `2a5c5360a97312f3453cbd44604f88b5d20c9136`,
`src/layers.test.ts`. The source SHA-256 is stored in `upstream.json.fixture`.

The upstream baseline has **261 passing tests, 10 skipped tests, and 543
`expect()` assertions**, plus **144 explicit throw guards**. This is a suite about copiable characteristics, faces,
merging, tokens, and catalogue queries. It is not an exhaustive Magic CR suite.

The full native audit is in [reports/wymi/findings.md](../../reports/wymi/findings.md).
Current result: **261 pass, 0 fail, 10 upstream skips**; all **144 guards pass**.
The per-assertion verdict is recorded in [results.md](../../reports/wymi/results.md).
The original source expectations remain frozen. Case 064 has one reviewed upstream
correction affecting two assertions, with original mismatch evidence retained.
The 70 focused regressions and additional compiler/runtime regressions are separate
from source counts. Failures are never silently skipped or marked expected-failure.

## Files and reproducibility

- `upstream.json.fixture`: frozen authored inputs and expected assertion arguments,
  original names, source lines, and the catalogue used by the final query.
- `expectation_errata.json.fixture`: explicit, rules-backed corrections identifying
  exact source assertion indices, original values, and corrected paths. Inventory
  verification rejects stale or overlapping corrections.
- `rules.json.fixture`: reviewed source AST-to-Oracle-text translations. The adapter
  compiles executable native abilities; it does not compare Wymi ASTs against themselves.
- `../../scripts/wymi/export.ts`: exports the pinned source with Bun. It records
  authored expected arguments, never replaces them with observed Wymi outputs.
- `../../scripts/wymi/inventory.py`: generates and verifies the one-to-one Rust test
  index, hashes of upstream code/data inputs, active/skipped counts, and assertion
  and guard counts.
- `../../crates/ironsmith-compiler-runtime/tests/wymi/adapter.rs`: native fixture
  translation and assertion execution.
- `../../crates/ironsmith-engine/tests/wymi_layers.rs`: the earlier focused suite,
  including six additional regressions. Its 70 tests remain separate from source counts.

From the Ironsmith root:

```sh
bun scripts/wymi/export.ts /path/to/wymi
python3 scripts/wymi/inventory.py
python3 scripts/wymi/inventory.py --check --upstream /path/to/wymi
python3 scripts/wymi/run.py --output reports/wymi
# Or execute only the native suite:
cargo test -p ironsmith-compiler-runtime --test wymi_complete --profile test-fast
```

Bun and the upstream repository are needed only to regenerate the frozen input.
The native suite consumes checked-in fixtures. `WYMI_AUDIT_DIR` writes one result
file per active source case, with a result for every assertion and guard, including assertions
after an earlier failure. Use a fresh output directory for each audit.

## Representation and setup adaptations

- Resolved source copy instructions map to the native continuous-effect primitives
  used by `BecomeCopy` lowering. Copy modifiers are input instructions, not edits to
  expected output. This does not exercise the named card's entire as-enters workflow.
- Wymi's `RuleSource` annotations have no gameplay counterpart in Ironsmith.
  Assertions compare executable ability payloads and preserve multiplicity, alongside
  name, mana cost, types, colors, P/T, loyalty, defense, and spell programs.
  Prototype and mutate casting metadata are checked separately as multisets.
  Comparisons use executable wire models, normalizing allocation/provenance IDs.
  Source representation guards that only assert a field’s structural type map to
  the corresponding statically typed native field; optional-value guards remain
  explicit runtime assertions.
- Wymi's layer-0 face-down placeholder is blank. Ironsmith's fused pipeline exposes
  the face-down characteristics immediately. The corresponding assertion uses the
  face-down instruction's authored characteristics, preserving the hidden identity.
  Copiable color comes from mana cost or color indicator; Devoid is retained as
  an executable ability and applies later in layer 5.
- Layer-1 token-copy assertions inspect native copiable snapshots before entry.
  Creating an actual token at that stage would run additional as-enters choices
  absent from the source assertion. Assertions carrying multiple faces also
  inspect linked characteristics and abilities. Actual token creation is exercised
  by the token cases and the earlier focused regression suite.
- Wymi checks Room door definitions before applying its layer-3 door designation.
  Those intermediate assertions inspect frozen native copied door definitions and,
  for merged Rooms, the retained native component graph before the locked-door
  overlay; they do not assert that a locked Room has active abilities.
- Some upstream mutate setups start with lands or Humans, despite those being
  ineligible mutate targets in that state. The adapter supplies reversible layer-4
  animation/removal of Human to model a previously authorized merge. These changes
  do not become copiable layer-1 characteristics.
- Source mana and type enums are translated into native enums. Catalogue queries
  use native object filters over the frozen catalogue, not today's card database.
- Fish and Octopus are Wymi convenience aliases, not predefined tokens in CR 111.10.
  Their frozen definitions supply the full ordinary creature-token instruction.
- In source case 263, `rule_lookup.get("Whenever equipped creature attacks, tap
  target creature defending player controls.")` returns `undefined`. The frozen source preserves that entry; it contributes no
  native ability. That source case asserts
  name, color, and types, not the missing ability's behavior. The same upstream
  fixture's `Equip {2}` lookup encodes an equip cost of one; this discrepancy must
  not be reported as an Ironsmith CR defect.

## Skipped source cases

Cases 216–219 are upstream-skipped dependency scenarios. Cases 220–225 are empty
upstream placeholders concerning copy exceptions and Vehicles/animated lands.
Their names and source locations remain indexed and explicitly ignored. They are
not counted as passing native coverage and are not invented as successful tests.

`run.py` executes the complete port, the compiler/runtime regressions, and the
70 focused engine tests. It saves a frozen result file with source/adapter/fixture hashes, the Git
revision and dirty-state flag, and the native test executable's SHA-256. It uses a
fresh result directory and exits nonzero while the suite is red. `report.py`
rejects missing assertion/guard results and requires new failure shapes to be
reviewed in `triage.py`. The test harness supplies its own 64 MiB parser stack;
no special `RUST_MIN_STACK` setting is required.

A red native test is evidence to investigate, not automatically a proven CR
violation. Compiler/setup failures, representation mismatches, upstream errors,
and confirmed runtime defects must be reported separately.
