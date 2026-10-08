# Source-owned must-be-blocked requirements

Status: source-only proposal. All authored tests are **UNRUN**. No build, test,
compiler probe, corpus/audit run, code generation, formatter, publication, or
catalog regeneration was performed. This packet contributes no measured fixes
or measured supported-card count. No claimed full-card credit can survive a
failed sibling-body, strict/loss-free, lowering, or runtime gate.

## Baselines and frozen evidence

- Source base: `ad0b0056cb8f9fe92ee9bb336b8184e1e9d101a8`.
- Audit evidence: the immutable Oct8 `1dd81cd` refresh, whose
  `analysis/current-unresolved-entries.json` records all three exact Oracle IDs
  as parser failures at the normalized `this creature must be blocked ...`
  clause. Its SHA-256 is
  `3e89d38bd63c3d906ce6cd46835c0aeed3c4e57a6af3cdf0368cb60fd0ff1e9f`.
- Official metadata and full Oracle bodies were selected unchanged from the
  frozen `reports/current-refresh-20261008/data/cards-current.json` in the
  refresh checkout, SHA-256
  `bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750`.
- Frozen feed: Scryfall default-cards, updated
  `2026-10-08T09:05:44.946+00:00`, download URI
  `https://data.scryfall.io/default-cards/default-cards-20261008090544.jsonl.gz`.
  This proposal made no new network request and did not replace that feed.
- Fixture: `fixtures/source_must_be_blocked.json.fixture`, SHA-256
  `03f15def6d0a6449cb12aa5078745ceccf4324dcd03fd2d33054b108f06e4806`.
  It retains the complete selected official rows, including metadata, body,
  print ID, Oracle ID, and source URI. No hand-authored reduced replacement body
  is used for candidate compilation.

## Exact candidate bodies

### Anzrag, the Quake-Mole

Oracle ID: `4adcd967-9ff7-4940-b8b9-0c4215bbcb75`.
Mana cost `{2}{R}{G}`; Legendary Creature — Mole God; 8/4.

> Whenever Anzrag becomes blocked, untap each creature you control. After this phase, there is an additional combat phase.
> {3}{R}{R}{G}{G}: Anzrag must be blocked each combat this turn if able.

Scope includes the entire blocked trigger, all controlled-creature untapping,
one additional combat per becomes-blocked occurrence (not per blocker), repeat
triggering in additional combats, and the exact seven-mana activation. A fixed
source obligation lasts across combats until cleanup and does not force all
available creatures to block.

### Glorfindel, Dauntless Rescuer

Oracle ID: `93842030-2017-4233-a7ac-7112361c019f`.
Mana cost `{2}{G}`; Legendary Creature — Elf Noble; 3/2.

> Whenever you scry, choose one and Glorfindel gets +1/+1 until end of turn.
> • Glorfindel must be blocked this turn if able.
> • Glorfindel can't be blocked by more than one creature each combat this turn.

Scope includes the actual scry trigger, both modes, exactly one common +1/+1
pump for each resolved trigger, one-blocker sufficiency in the first mode, the
maximum-one-blocker restriction in the second, combination across two triggers,
and cleanup. Scry by an opponent does not trigger it; scry 2 is one event.

### Loathsome Catoblepas

Oracle ID: `c6f68e4b-af2b-43d5-8052-104defe7f3ec`.
Mana cost `{5}{B}`; Creature — Beast; 3/3.

> {2}{G}: This creature must be blocked this turn if able.
> When this creature dies, target creature an opponent controls gets -3/-3 until end of turn.

Scope includes exact three-mana activation and the complete death trigger,
opponent-only creature targeting, -3/-3, expiry, another creature's death as a
nonmatch, and exile as a non-death transition.

## Ownership diagnosis and bounded implementation

The complete combat-requirement shape already consumes the full `if able`
suffix and typed turn/combat duration. Target lowering already maps a complete
`this creature` or `this permanent` phrase to the source object. Card-name
preprocessing emits these source subjects, but the specific primitive registry
only admitted `it`, `that`, `they`, and `target`. That mismatch sent complete
source-owned requirements into generic verb search, which cannot own `must`.

The production change adds only `this` to the must-be-blocked primitive's
candidate heads. The must-be-blocked shape retains its source subject's raw
tokens before subject-edge trimming, and the primitive does not trim that
slice again. Its newly reachable source subjects are required to consist
of word tokens and a complete existing source-reference shape; mana symbols,
colons, or unsupported qualifications cannot be discarded by word projection.
The existing complete combat-shape parser still owns `if able`, suffix
consumption, and duration. Existing typed restrictions and runtime executors
are reused. There are no card-name switches, synthesized bodies, unsupported
mechanic placeholders, new wire fields, or changes to attack-this-turn heads.

## Source-authored verification gates (all UNRUN)

- Grammar tests exercise the specialist, registry, and public sentence route
  independently for source creature/permanent, ordinary if-able, this turn,
  each combat this turn, and this combat. They inspect the exact typed source
  filter, single effect, and turn versus combat expiry.
- Grammar negatives reject missing `if able`, all/two-blocker substitutions,
  next-turn duration, unowned suffixes, crossing a sentence boundary, malformed
  source tokens, and unsupported qualified source subjects. Separate ownership
  assertions keep source attacks/attacks-or-blocks outside this registry change.
- `source_must_be_blocked_lowering.rs` compiles each entire metadata-bearing
  body through the strict compiler facade with loss capture. It checks expected
  ability counts and mandatory siblings, typed source restrictions, no invented
  target or activation cost, modal common pump and both modes, and the additional
  combat carrier. Unsupported extra siblings and malformed requirements must
  make strict whole-card compilation fail.
- `source_must_be_blocked.rs` independently calls the direct compiler and the
  artifact compiler, serializes and decodes the artifact, validates it, and
  materializes its decoded definition. Every path rejects parse loss and
  unimplemented content before the behavioral scenarios run.
- Native runtime scenarios cover legal and illegal blocker declarations,
  one-sufficient versus all-blockers, insufficient/incorrect-color mana, payment
  before resolution, source-only identity, third-party defenders, tapped/cannot-
  block inability, ability removal after resolution, blink identity, and cleanup.
  All printed sibling mechanics above are included in both materializations.

These are authored assertions, not passing results. The later authorized gate
must compile the tests first and run them before claiming measured support.
An unimplemented sibling body disqualifies the entire corresponding card.

## Compatibility boundary and accounting

The source base already carries the proposed artifact15/audit29 boundary.
This proposal changes which source texts compile and tightens malformed source
ownership. It therefore requires a **later compiler semantic boundary** before
baking/publishing artifacts or comparing a fresh audit. It does not alter the
artifact15 descriptor, assign a new release number, reuse old cache identity,
or rewrite the immutable Oct8 measurements. Boundary numbering, integration,
publication, and any accounting ledger update belong to the coordinator.

Candidate identities: 3. Newly measured fixed cards: 0. Executed tests: 0.

## Raw-source follow-up to independent review

Review of `7e4bc0c9470de16d5778c9a37b5437cf5bf64338` identified a real
ownership defect: the initial complete-source guard saw tokens after
`trim_shape_edges` and `LexedClause::trimmed` had already discarded punctuation.
A comma, period, semicolon, or quote directly before `must` could consequently
disappear. This follow-up preserves the original captured subject tokens only
for the newly reachable `this`-headed must-be-blocked case and removes the
primitive's second trim. A period is now visible to the existing sentence-
boundary rejection; the other retained nonword tokens reach the explicit
complete-source rejection. Attack and attacks-or-blocks families, non-source
normalization, suffix grammar, and compatibility descriptors are unchanged.

Additional UNRUN regressions cover all four punctuation tokens for both
creature/permanent subjects and all four supported duration surfaces through
the specialist and primitive registry. The preexisting positive matrix remains
the valid control. A whole-body negative preserves Loathsome Catoblepas's
complete printed death trigger and changes only the punctuation before the
activation's `must`; it demands strict rejection independently from compiler
lowering, direct runtime compilation, and artifact compilation. No unrelated
unsupported sibling is inserted to cause those negative results.

These corrections remain source-authored, not execution evidence. The original
commit is preserved and the follow-up is submitted for independent rereview.
