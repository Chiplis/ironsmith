# Supplemental source-face audit

The full canonical baseline is unchanged. It compiles one selected payload per
source name. A successful front does not establish support for another face.

## Measured frozen scope

For the frozen `9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`
source (original engine `e8740178a7f7367ffa3147e7642607042079237c`):

- 32,209 source entries, 32,138 distinct top-level Oracle IDs
- 918 multiface source entries, 1,836 exact source-face contexts
- 915 distinct effective Oracle IDs among those contexts (face ID where present,
  otherwise parent ID); these are not 1,836 unique Oracle cards
- 1,752 unique face-name requests; the batch loader selects 1,678 exact faces and
  74 canonical payloads instead
- 158 omitted contexts: all 142 reversible faces and 16 prepared faces
- The product generator has 1,694 linked-face contexts in six layouts; its 71
  reversible sources take the single-card branch, so this is a separate unit

`fixtures/card-failure-campaign/face-route-scope.json` freezes compact membership
and every shadowed context. `source:<array index>/face:<index>` is scoped by the
source SHA-256. Names and Oracle IDs are intentionally not unique route keys.

## Verified code pathways and limits

`crates/ironsmith-tools/src/tooling.rs`:

1. `build_registry_card_record_with_explicit_includes` selects the canonical/front
   text and metadata. Linked metadata does not compile another face.
2. `load_card_payloads_by_names` traverses the original source order and records
   only the first payload for each normalized name. A canonical name can win
   before a face. Repeated prepared spells can come from a different parent.
   Reversible equal-name faces lose the second context even within one source.
3. `load_card_payloads_by_name` returns *all* matching faces in a source. An
   isolated unmodified source object therefore preserves repeated same-name
   faces, without reimplementing the Rust metadata builder.
4. Tooling linked metadata recognizes `transform`, `split`, and `flip`. Its face
   payload for `prepare`, `adventure`, and `modal_dfc` does not have the same
   linked metadata as the product baker. Face text acceptance is not verification
   of linked-card casting, transform, adventure, or prepare behavior.

`compile_oracle_text --compare-text --continue-on-error` calls the authoritative
snapshot API, but displays only text, score, and semantic mismatch. Strict vs
permissive compilation, parse loss, and unsupported flags are not exposed. An
exit of zero means the diagnostic process finished, including failed cards. For
multi-payload queries, one error also suppresses sibling output. The diagnostic
analyzer rejects those ambiguous queries instead of inventing sibling results.

The product path is `scripts/generate_baked_registry.py::collect_unique_blocks`
then `ironsmith-artifact-baker::compile_source`. The generator builds linked
blocks for split, flip, transform, modal DFC, adventure, and prepare. Prepare
spells deliberately do not claim standalone frontend routes, because they can
share real card names. The baker strictly compiles each linked face, wires both
relationships, distinguishes Prepare/Split/Flip and transforming DFC metadata,
serializes and materializes the definition. These are additional behavior and
artifact contracts, not consequences of the tools snapshot. The generator's
score lookup may use a combined-name score for either face; that is not evidence
that both exact faces previously passed.

## Frozen diagnostic batch

```sh
python3 scripts/card_failure_faces.py run-cli \
  --cards /tmp/ironsmith-campaign-cards.json \
  --compile-bin /path/to/frozen/compile_oracle_text \
  --build-manifest /path/to/compile-oracle-build-manifest.json \
  --out-dir /tmp/face-cli-baseline
```

The binary digest must be explicitly bound in its build manifest. The command
runs exactly one process, loads all names once, clears inherited `IRONSMITH_*`,
sets `RAYON_NUM_THREADS=1`, and supplies empty stdin. A new output directory holds
source membership, exact command, source/binary/names/log digests, a copy of the
harness and build provenance, stdout, stderr, and attributed diagnostics. Input
or membership changes fail closed. Selected output names and normalized Oracle
text are checked; errors expose only the selected name. Resolver context is
source-derived until the exact Rust exporter is executed.

Each exact route is failed, `cli_compiled_unverified`, or omitted. Omitted routes
never inherit the result of a same-name face or canonical card. Strict supported
route count remains unknown. Unique failing Oracle cards, source entries, face
routes, and query outcomes are separate. This report cannot satisfy a campaign
completion gate.

## Preserved baseline diagnostic result

The one frozen CLI run completed on 2026-10-03 with return code 0. Its 1,752
requests produced 113 errors and 1,639 compiled-looking outputs. Attributable
exact source-face contexts are:

- 108 failed routes across 95 unique Oracle cards and 95 source entries
- 1,570 `cli_compiled_unverified` routes
- 158 omitted routes
- Strict supported route count: unknown

Five query errors belong to canonical collision winners, not exact face routes.
Do not add the 95 to the canonical baseline's failing-card count without joining
Oracle identities. This is a diagnostic lower bound, not a complete failure set.

`fixtures/card-failure-campaign/faces-cli-e8740178.tar.gz` preserves stdout,
stderr, requested names, full membership/diagnostics, run/build provenance, and
the exact hashed harness used. Its sibling manifest binds the archive and all
members by SHA-256. That initial run predates some additions in the committed
harness; its exact executable source is included rather than retrospectively
claiming the later file ran. To replay after extracting to a fresh directory:

```sh
python3 /tmp/faces-cli-e8740178/harness.py analyze-cli \
  --run-dir /tmp/faces-cli-e8740178
```

## Exact authoritative evidence, prepared for a serialized build slot

`crates/ironsmith-tools/examples/audit_campaign_faces.rs` uses only the existing
public loader and authoritative snapshot API. It isolates each **unchanged**
source object in a temporary JSON array, groups equal face names, calls the
single-name API, verifies returned count/name/text/order, then emits all source
contexts and full snapshot fields to flushed JSONL. It writes a completion
footer only after exact coverage and unchanged source bytes are verified. A
partial file, duplicate route, context mismatch, or missing strict fields fails
analysis. `--inventory-only` verifies actual payload membership without compiling
and never counts as compile success.

No Rust build or exact run was performed during the concurrent baseline audit.
After the coordinator grants a build slot:

```sh
cargo test --locked -p ironsmith-tools --example audit_campaign_faces
cargo build --locked -p ironsmith-tools --example audit_campaign_faces
/path/to/frozen/audit_campaign_faces \
  --cards /tmp/ironsmith-campaign-cards.json \
  --out /tmp/exact-faces.jsonl \
  --expected-sha256 9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c
python3 scripts/card_failure_faces.py analyze-exact \
  --cards /tmp/ironsmith-campaign-cards.json \
  --jsonl /tmp/exact-faces.jsonl --out /tmp/exact-faces.json
python3 scripts/card_failure_faces.py compare-exact \
  --baseline /tmp/exact-baseline.json --current /tmp/exact-current.json \
  --out /tmp/exact-face-progress.json
```

For baseline use, build at original e874 engine sources with only the additive
example overlaid. Record original engine commit/tree, additive audit commit,
actual build command/toolchain/log, binary SHA-256, execution command/environment,
and stdout/stderr. Do not label a build at the changing integration branch as the
original baseline. The JSONL analysis validates coverage and snapshot content;
it does not independently establish binary build provenance.

`compare-exact` rejects changed source, membership, or payload context, rejects
permissive/lossy/unsupported results, and detects support loss, new semantic
mismatch, or score decrease. Definition changes are surfaced for review.
Gameplay tests and product-baker integration remain separate mandatory gates.

## Lightweight tests

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s scripts -p test_card_failure_faces.py -v
```

These cover name collisions, duplicate reversible names, separate card/route
units, output attribution, missing/extra/duplicate evidence, exact snapshot
identity and definition digests, strict/allow/loss rejection, and regressions.
The Rust example includes three route tests; they remain pending until its build
slot. None of these audit changes fixes a card mechanic or claims runtime safety.
