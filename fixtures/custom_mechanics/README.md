# Custom mechanic audit fixtures (schema version 1)

These fixtures preserve source evidence; they are not declarations that a rewrite
is semantically equivalent or a mechanic works in gameplay. Never infer either
claim from a successful compile or rendered-text match.

Each cohort JSON contains `schema_version: 1`, a `provenance` object identifying
actual source files and retrieval date, and a `mechanics` array. Each mechanic
has:

- `id`: stable, unique identifier within its cohort (include reminder variant).
- `keyword`: mechanic identity, also attached to the compiler builder.
- `original_reminder`: exact sourced reminder, without inventing missing text.
- `executable_reminder`: reviewed candidate string, or `null` pending a rewrite.
- `representative_cards`: nonempty array of `{id, name, original_text,
  executable_text}`. Both texts are full compiler inputs including metadata;
  executable text may be `null` pending a rewrite. Preserve original card text;
  executable text must contain the expansion actually passed by the frontend.
- Optional `semantic_equivalence`: `{status: "unreviewed" | "reviewed", evidence}`.
  Reviewed means human/rules analysis, with substantive evidence, not compilation.
- Optional `behavioral_assertions`: array of `{id, status: "pending" | "passed" |
  "failed", test_reference, observable}`. Name the actual native test and the
  observed behavior; a passed status needs evidence from an actual test run.
- Optional `rule_reference`, `rule_text`, `scope`, and source card references retain
  provenance and restrictions absent from reminders.

An unchanged candidate may use the original reminder and text explicitly. Null
means no candidate exists, not an empty ability. Keep variants separate even when
readers are shared. Parameter substitutions and import expansion belong in the
frontend fixture-generation path; the native harness must not implement its own
mechanic-specific paragraph rewrite.

Run the structural batch audit from the repository root:

```sh
cargo run -p ironsmith-compiler-runtime --example audit_custom_mechanics -- fixtures/custom_mechanics/COHORT.json > reports/custom-mechanics/COHORT-native.jsonl
```

The executable emits one JSONL row per mechanic, including each representative
card's original and candidate result. It continues after parse failures, panics,
and malformed mechanic/card rows. File/JSON/schema errors fail the invocation.
Both word and object-filter coverage are enforced; unsupported parsing is off.
Parse-loss diagnostics fail the structural gate. Successful candidates undergo
artifact JSON equality and runtime materialization/render checks. These checks
protect transport structure, not behavioral equivalence. Missing candidates are
reported as `pending_rewrite`. The semantic gate always remains
`requires_behavioral_verification`; fixture status claims are merely echoed.

Type-line fidelity is part of the gate too: the compiled card must preserve all
recognized supertypes, card types, and subtypes. Unknown type-line words must fail
instead of disappearing. A historical structural pass alone is insufficient:
earlier audits could report success after dropping an unknown custom subtype.
`custom_metadata_fidelity` covers this boundary, including collective planar names.

A complete release gate must additionally run referenced native scenarios and
review every original-to-candidate semantic difference. Inventory completeness
requires comparing fixture IDs against source inventories; the harness cannot
establish completeness of a corpus it was not given.

## Native behavioral scenario patterns

Use the smallest harness that demonstrates an observable rule. Existing examples:

- `custom_mechanic_word_coverage.rs`: Ambush's alternative method must be legal
  only during its controller's declare attackers step, discount only that method
  by the number of attackers (including untapped attackers), and enter tapped
  attacking only when that method was used. Assert offered legal actions, mana
  payment, and combat membership after normal stack resolution. Exercise zero
  attackers, a different phase, and normal casting as negative cases.
- `custom_statuses.rs`: Bribe's reusable status effect must preserve its name and
  duration, trigger a listener exactly on gaining the right status, avoid another
  status/controller/zone, grant conditional characteristics, expire at cleanup,
  and disappear across a zone change. Assert trigger entries and calculated
  characteristics as well as status identity; mere parsing is insufficient.
- `src/reminder_identity_tests.rs`: attach custom keyword identity independently
  from its expanded behavior. Verify interactions with official keyword-specific
  permissions rather than equating superficially similar reminder wording.

For new mechanics add positive and negative scenarios for costs, target filters,
choice branches, timing, zones, duration and trigger scope actually present in the
source rule. Use ordinary cast/entry/trigger resolution when that integration is
the behavior being tested. Do not substitute direct state mutation for the event
that the rule is intended to observe.

The current native cohort and regression targets can be run together:

```sh
cargo test -p ironsmith-compiler-runtime --no-fail-fast \
  --test custom_casting_costs --test custom_revolution_counter \
  --test custom_revolution_gorge --test custom_revolution_standard_effects \
  --test custom_token_actions --test custom_resource_tokens \
  --test custom_token_quote_dedup --test custom_statuses \
  --test custom_player_actions --test custom_departure_actions \
  --test custom_serial_event_triggers --test custom_satrapy_activation \
  --test custom_deception --test custom_selected_cast_redirects \
  --test custom_abound --test custom_rediscover --test custom_metadata_fidelity \
  --test custom_parameter_programs --test custom_revive \
  --test custom_mechanic_word_coverage
cargo test -p ironsmith-compiler-runtime --lib reminder_identity_tests
```

These targets are a growing set of scenarios, not proof that the entire inventory
works. Preserve failures and pending variants in the coverage report; never
replace the complete inventory with the subset covered by these tests.

## Frontend expansion adapter

`audit_custom_mechanic_expansions` accepts the existing native-worker JSONL
protocol: `{layout, token, faces: [{name, parse_input, customKeywordIdentities,
customKeywordPayments}]}`. Feed actual `compileDraftInput` results through the
existing `native-worker-client.mjs` supervisor, with the freshly built executable.
It preserves custom payment attribution and linked-face metadata. Coverage is
always strict, unsupported parsing is always disabled, parse loss rejects
`canCreate`, and artifact JSON decode/materialization is checked. Its output is
compatible with the existing frontend probe scripts; `canCreate` is only a
structural admission result. It provides no gameplay certification. Fatal panics
terminate the worker after an output row so the supervisor can restart it.

## Rule boundary and terminal unsupported status

`unsupported_requires_new_rules` is an authorized terminal category when the full
source rule requires adding a new game rule. Record its specific rule obligation
and implementation boundary in `fixtures/custom_mechanics/rule-boundaries.json`.
A parse failure, missing reader, missing generic plumbing, or absent scenario is
never evidence for this category. Generic runtime plumbing for existing operations
is allowed. Status labels alone are not new rules: the generic custom permanent
status implementation already supports named markers, conditions, gained-status
triggers and cleanup. Rules requiring a genuinely new game-wide subsystem remain
unsupported; never silently replace them with a similar triggered ability.

`scripts/custom-mechanics/coverage.py` checks the complete source inventory and
merges candidate overlays by stable ID. It reads the versioned rule boundaries;
generated reports do not supply authoritative classification decisions.
`candidate_full_card` requires materialized frontend input for every representative
card and every supplied or layout-required face. It does not certify compilation
or gameplay; manually written equivalent text remains a reminder candidate.

For a materialized frontend cohort, run
`node scripts/custom-mechanics/audit-cohort.mjs FIXTURE OUTPUT_JSONL EXECUTABLE`
with a freshly built `audit_custom_mechanic_expansions` executable. This preserves
the frontend's custom identity and payment metadata and records input/binary
hashes. Missing frontend expansion or linked-face input remains pending, even
when a manually written reminder compiles. A structural pass is never gameplay
certification.

The versioned boundary authority is `rule-boundaries.json`; ignored report files
are derived review/readability artifacts. It includes complete source rule objects,
source hashes, implementation evidence, unresolved analysis and terminal decisions.
It is a boundary inventory, not a card-cohort fixture, and is not input to the
`audit_custom_mechanics` cohort executable. Rule-only cases (such as Idolize when
no source card occurrence exists) are recorded separately without inventing cards.

To populate full-card candidate text from reviewed `executable_definitions`, run
`node scripts/materialize-custom-mechanic-cohort.mjs FIXTURE.json`. This uses the
actual frontend `compileDraftInput` path and records unverified materialization,
binding failures and linked-face pending status. It never certifies semantic or
gameplay correctness. Every linked face must be represented before a full-card
claim; single-face materialization explicitly leaves such cards pending.

The materializer reads candidate definitions from sibling cohort JSON files only for the same stable `source_card_key` (falling back to `id` when no key exists). This allows different action cohorts to expand the complete shared source card without installing a global keyword library. Display names never establish identity. All imported definitions are validated; different reminders, layouts or forms under the same normalized keyword produce a visible `source_definition_conflict` and no executable card output. Original text and linked source faces remain the materialization input. This merge does not certify structural or gameplay support.
